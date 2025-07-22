use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use ignore::WalkBuilder;
use ignore_files::{IgnoreFilesFromOriginArgs, IgnoreFilter};
use watchexec::WatchedPath;
use watchexec::Watchexec;
use watchexec_events::Event;
use watchexec_filterer_ignore::IgnoreFilterer;
// use watchexec_events::{Priority};
// use watchexec_signals;

use crate::queue::JobDispatcher;
use crate::queue::job::{Job, JobPriority, JobType};
use workspace_manager::{Status, WorkspaceManager};

const RESOLVE_IGNORE_FILTER_TIMEOUT: Duration = Duration::from_secs(30);
const WATCHER_SPAWN_INTERVAL: Duration = Duration::from_millis(200);
const DEBOUNCE_DURATION: Duration = Duration::from_millis(3000);
const MAX_EVENTS_PER_DEBOUNCE_WINDOW: usize = 8192;
const EXCLUDED_SUBDIRECTORIES: &[&str] = &[".git", ".idea", ".vscode", ".cache"];
const PERIODIC_REINDEX_INTERVAL: Duration = Duration::from_secs(600); // 10 minutes

#[derive(Default)]
pub struct WatcherConfig {
    periodic_force_index: bool,
}

impl WatcherConfig {
    pub fn new() -> Self {
        Self {
            periodic_force_index: false,
        }
    }
}

pub struct Watcher {
    // Used to list all workspaces and their project folders/paths
    pub workspace_manager: Arc<WorkspaceManager>,
    // Set of watched project folders for which we have a watcher active
    pub watched_project_folders: Arc<Mutex<HashSet<PathBuf>>>,
    // Used to trigger reindexing jobs
    pub job_dispatcher: Arc<JobDispatcher>,
    // Map of project path to its events, grouped by debounce windows
    project_events: Arc<Mutex<HashMap<PathBuf, Vec<Vec<PathBuf>>>>>,
    // Track the start time of the current debounce window for each project
    debounce_windows: Arc<Mutex<HashMap<PathBuf, Instant>>>,
    // Track the task handles for each project watcher so we can stop them
    watcher_handles: Arc<Mutex<HashMap<PathBuf, JoinHandle<()>>>>,
    // For sending the changed paths to the job dispatcher
    event_sender: mpsc::Sender<(PathBuf, PathBuf, Vec<PathBuf>)>,
    // Current runtime for the watcher
    runtime: tokio::runtime::Handle,
    // Cancellation token for graceful shutdown
    cancellation_token: CancellationToken,
    // Watcher config
    watcher_config: WatcherConfig,
}

impl Watcher {
    pub fn new(
        workspace_manager: Arc<WorkspaceManager>,
        job_dispatcher: Arc<JobDispatcher>,
        watcher_config: Option<WatcherConfig>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(MAX_EVENTS_PER_DEBOUNCE_WINDOW);
        let job_dispatcher_clone = job_dispatcher.clone();
        let cancellation_token = CancellationToken::new();
        let watcher = Self {
            workspace_manager,
            watched_project_folders: Arc::new(Mutex::new(HashSet::new())),
            job_dispatcher,
            project_events: Arc::new(Mutex::new(HashMap::new())),
            debounce_windows: Arc::new(Mutex::new(HashMap::new())),
            watcher_handles: Arc::new(Mutex::new(HashMap::new())),
            event_sender: tx,
            runtime: tokio::runtime::Handle::current(),
            cancellation_token,
            watcher_config: watcher_config.unwrap_or_default(),
        };

        watcher.runtime.spawn(async move {
            Self::process_events(rx, job_dispatcher_clone).await;
        });

        watcher
    }

    async fn process_events(
        mut rx: mpsc::Receiver<(PathBuf, PathBuf, Vec<PathBuf>)>,
        job_dispatcher: Arc<JobDispatcher>,
    ) {
        while let Some((workspace_path, project_path, changed_paths)) = rx.recv().await {
            if changed_paths.is_empty() {
                info!("No changed paths, skipping reindexing job dispatch");
                continue;
            }

            info!("\nProcessing events for project: {project_path:?}");
            info!("changed paths in group: {}", changed_paths.len());
            info!("Changed paths: {changed_paths:?}");

            // TODO: change this to ReindexProjectFolderWithWatchedFiles
            // TODO: Also allow ReindexWorkspaceFolderWithWatchedFiles with a flag in WatcherConfig
            let job = Job::ReindexWorkspaceFolderWithWatchedFiles {
                workspace_folder_path: workspace_path.to_string_lossy().into_owned(),
                workspace_changes: changed_paths.into_iter().collect(),
                priority: JobPriority::Normal,
            };

            info!("Watcher dispatching re-indexing job: {:?}", job);
            let job_id = job_dispatcher.dispatch(job).await.unwrap();
            info!("Watcher dispatched re-indexing job with id: {:?}", job_id);
        }
    }

    async fn stop_abandoned_project_watchers(&self, project_folder_paths: &[PathBuf]) {
        let mut watched_project_folders = self.watched_project_folders.lock().unwrap();
        let current_paths: HashSet<PathBuf> = project_folder_paths.iter().cloned().collect();
        let folders_to_remove: Vec<PathBuf> = watched_project_folders
            .difference(&current_paths)
            .cloned()
            .collect();

        for folder in folders_to_remove {
            info!("Stopping project watcher for removed folder: {:?}", folder);
            // Join on the abandoned watchers
            if let Ok(mut handles) = self.watcher_handles.lock() {
                if let Some(handle) = handles.remove(&folder) {
                    handle.abort();
                }
            }

            // Remove events, debounce windows, and watched folder
            if let Ok(mut events) = self.project_events.lock() {
                events.remove(&folder);
            }
            if let Ok(mut windows) = self.debounce_windows.lock() {
                windows.remove(&folder);
            }
            watched_project_folders.remove(&folder);
        }
    }

    async fn monitor_workspace_folders(watcher: Arc<Watcher>) {
        loop {
            if watcher.cancellation_token.is_cancelled() {
                info!("Workspace folder monitoring shutting down");
                break;
            }

            // Only proceed with launching watchers if the underlying projects are indexed or being reindexed
            let active_project_paths: Vec<(PathBuf, PathBuf)> = watcher
                .workspace_manager
                .list_all_projects()
                .iter()
                .filter(|p| p.status == Status::Indexed || p.status == Status::Reindexing)
                .map(|p: &workspace_manager::ProjectInfo| {
                    (
                        PathBuf::from(&p.workspace_folder_path),
                        PathBuf::from(&p.project_path),
                    )
                })
                .collect::<Vec<_>>();

            watcher
                .stop_abandoned_project_watchers(
                    &active_project_paths
                        .iter()
                        .map(|(_, p)| p.clone())
                        .collect::<Vec<_>>(),
                )
                .await;

            let paths_needing_watchers = {
                let mut watched_folders = watcher.watched_project_folders.lock().unwrap();

                // Find project folder paths that don't have watchers yet
                let paths_needing_watchers: Vec<(PathBuf, PathBuf)> = active_project_paths
                    .into_iter()
                    .filter(|(_, project_path)| !watched_folders.contains(project_path))
                    .collect();

                // Mark these paths as being watched (optimistically)
                for (_, project_path) in &paths_needing_watchers {
                    watched_folders.insert(project_path.clone());
                }

                paths_needing_watchers
            };

            for (workspace_folder_path, project_path) in &paths_needing_watchers {
                info!(
                    "Starting new project watcher for: {:?} in workspace: {:?}",
                    project_path, workspace_folder_path
                );
                watcher
                    .start_project_watcher(workspace_folder_path, project_path)
                    .await;
            }

            // Use select! to allow cancellation during sleep
            tokio::select! {
                _ = tokio::time::sleep(WATCHER_SPAWN_INTERVAL) => {},
                _ = watcher.cancellation_token.cancelled() => {
                    info!("Workspace folder monitoring cancelled during sleep");
                    break;
                }
            }
        }
    }

    pub async fn start(self: Arc<Self>) {
        info!(
            "Watcher is excluding the following (sub)directories: {:?}",
            EXCLUDED_SUBDIRECTORIES
        );

        let watcher_clone = Arc::clone(&self);
        self.runtime.spawn(async move {
            Self::monitor_workspace_folders(watcher_clone).await;
        });

        // Start periodic reindexing thread
        if self.watcher_config.periodic_force_index {
            let watcher_clone = Arc::clone(&self);
            self.runtime.spawn(async move {
                Self::periodic_force_index(watcher_clone).await;
            });
        }
    }

    async fn periodic_force_index(watcher: Arc<Watcher>) {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(PERIODIC_REINDEX_INTERVAL) => {
                    let projects = watcher.workspace_manager.list_all_projects();
                    let project_paths: Vec<PathBuf> = projects.iter().map(|p| PathBuf::from(&p.project_path)).collect();
                    watcher.stop_abandoned_project_watchers(&project_paths).await;

                    // After stopping all watchers, we can cancel all existing reindexing jobs
                    for workspace_folder in watcher.workspace_manager.list_workspace_folders() {
                        let projects_in_workspace = watcher.workspace_manager.list_projects_in_workspace(&workspace_folder.workspace_folder_path);
                        let project_paths_in_workspace: Vec<PathBuf> = projects_in_workspace.iter().map(|p| PathBuf::from(&p.project_path)).collect();

                        for project_path in project_paths_in_workspace {
                            let project_path_str = project_path.to_string_lossy().into_owned();
                            let reindexing_job_type = JobType::ReindexWorkspaceFolderWithWatchedFiles.as_str();
                            match watcher.job_dispatcher.cancel_existing_jobs_of_type(&project_path_str, reindexing_job_type)
                                .await {
                                    Ok(_) => {
                                        info!("Cancelled existing reindexing jobs for {}", project_path_str);
                                    }
                                    Err(e) => {
                                        error!("Failed to cancel existing reindexing jobs for {}: {}", project_path_str, e);
                                    }
                                }
                        }
                    }

                    // After cancelling all existing reindexing jobs, we can dispatch a fresh indexing job for each workspace folder
                    for workspace_folder in watcher.workspace_manager.list_workspace_folders() {
                        let job = Job::IndexWorkspaceFolder {
                                workspace_folder_path: workspace_folder.workspace_folder_path.clone(),
                                priority: JobPriority::High,
                        };
                        if let Err(e) = watcher.job_dispatcher.dispatch(job).await {
                            error!("Failed to dispatch periodic reindex job for {}: {}", workspace_folder.workspace_folder_path, e);
                        } else {
                            info!("Dispatched periodic reindex job for {}", workspace_folder.workspace_folder_path);
                        }
                    }
                }
                _ = watcher.cancellation_token.cancelled() => {
                    info!("Periodic force indexing job cancelled");
                    break;
                }
            }
        }
    }

    async fn resolve_ignore_filter(
        project_path: &Path,
    ) -> Result<IgnoreFilterer, Box<dyn std::error::Error>> {
        // Move git config reading to blocking context to avoid Rc<gix_config::file::Metadata> Send issues
        // Note: This is due to the fact that the ignore_files crate uses gix_config::file::Metadata which is not Send
        let project_path_clone = project_path.to_path_buf();
        let ignore_filter = tokio::time::timeout(
            RESOLVE_IGNORE_FILTER_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async {
                    let args = IgnoreFilesFromOriginArgs::from(project_path_clone.as_path());
                    let (files, _errors) = ignore_files::from_origin(args).await;
                    IgnoreFilter::new(&project_path_clone, &files[..]).await
                })
            }),
        )
        .await
        .map_err(|_| "Timeout resolving ignore filter")?
        .expect("Failed to resolve ignore filter")?;
        Ok(IgnoreFilterer(ignore_filter))
    }

    fn compute_project_watcher_pathset(
        workspace_manager: &WorkspaceManager,
        workspace_path: &Path,
        project_path: &Path,
    ) -> Vec<WatchedPath> {
        if project_path != workspace_path {
            return vec![WatchedPath::recursive(project_path.to_path_buf())];
        }

        let mut pathset: Vec<WatchedPath> = Vec::new();

        // Get all other project paths (excluding the current one)
        let projects = workspace_manager.list_all_projects();
        let other_project_paths: std::collections::HashSet<PathBuf> = projects
            .iter()
            .filter(|p| PathBuf::from(&p.project_path) != project_path)
            .map(|p| PathBuf::from(&p.project_path))
            .collect();

        // Watch the project path non-recursively (files directly in the project root)
        pathset.push(WatchedPath::non_recursive(project_path.to_path_buf()));

        let walker = WalkBuilder::new(project_path)
            .follow_links(false) // Don't follow symlinks
            .hidden(false) // Include hidden files/directories
            .git_ignore(true) // Respect .gitignore files
            .git_global(true) // Respect global git ignore
            .git_exclude(true) // Respect .git/info/exclude
            .build();

        for entry in walker.flatten() {
            let path = entry.path().to_path_buf();

            // Skip other project paths entirely
            if other_project_paths.contains(&path) {
                continue;
            }

            // Check ALL path components for excluded subdirectories
            if path.components().any(|comp| {
                if let Some(comp_str) = comp.as_os_str().to_str() {
                    EXCLUDED_SUBDIRECTORIES.contains(&comp_str)
                } else {
                    false
                }
            }) {
                continue;
            }

            if entry.file_type().is_some_and(|ft| ft.is_dir()) && path != project_path {
                if let Some(parent) = path.parent() {
                    let parent_has_conflicts = other_project_paths
                        .iter()
                        .any(|other_project| other_project.starts_with(parent));

                    if parent_has_conflicts || parent == project_path {
                        let dir_has_conflicts = other_project_paths
                            .iter()
                            .any(|other_project| other_project.starts_with(&path));

                        if dir_has_conflicts {
                            debug!(
                                "Adding non-recursive watch for directory with nested conflicts: {:?}",
                                path
                            );
                            pathset.push(WatchedPath::non_recursive(path));
                        } else {
                            debug!("Adding recursive watch for clean subdirectory: {:?}", path);
                            pathset.push(WatchedPath::recursive(path));
                        }
                    }
                }
            }
        }

        pathset
    }

    async fn start_project_watcher(&self, workspace_path: &Path, project_path: &Path) {
        if let Ok(ignore_filterer) = Self::resolve_ignore_filter(project_path).await {
            let project_path_clone = project_path.to_path_buf();
            let workspace_path_clone = workspace_path.to_path_buf();
            let events_map = self.project_events.clone();
            let windows_map = self.debounce_windows.clone();
            let event_sender = self.event_sender.clone();

            let pathset = Self::compute_project_watcher_pathset(
                &self.workspace_manager,
                workspace_path,
                project_path,
            );
            debug!(
                "computed pathset for project: {:?} in workspace: {:?} ->  {:?}",
                project_path, workspace_path, pathset
            );

            match Watchexec::new(move |action| {
                debug!(
                    "Received watchexec action with {} events",
                    action.events.len()
                );

                let current_time = Instant::now();
                let mut windows = windows_map.lock().unwrap();
                let mut events = events_map.lock().unwrap();

                // Get or create window start time for this project path
                let window_start = windows
                    .entry(project_path_clone.clone())
                    .or_insert(current_time);

                // Get the current group of events for this project path
                let project_events = events.entry(project_path_clone.clone()).or_default();

                // Create first group if none exists
                if project_events.is_empty() {
                    project_events.push(Vec::new());
                }

                // Add events to the current group
                let current_group = project_events.last_mut().unwrap();

                for event in action.events.iter() {
                    current_group.extend(Self::handle_file_event(event));
                }

                // If we have events and debounce window elapsed, process them
                if current_time.duration_since(*window_start) >= DEBOUNCE_DURATION {
                    *window_start = current_time;
                    let events_to_process = project_events.pop().unwrap();
                    project_events.push(Vec::new());

                    let ws_path = workspace_path_clone.clone();
                    let proj_path = project_path_clone.clone();
                    let sender = event_sender.clone();
                    tokio::spawn(async move {
                        if let Err(e) = sender.send((ws_path, proj_path, events_to_process)).await {
                            error!("Failed to send events for processing: {}", e);
                        }
                    });
                }
                action
            }) {
                Ok(wx) => {
                    wx.config.filterer(ignore_filterer);
                    wx.config.pathset(pathset);
                    wx.config.throttle(DEBOUNCE_DURATION);

                    let handle = self.runtime.spawn(async move {
                        if let Err(e) = wx.main().await {
                            error!("Error in file watcher: {}", e);
                        }
                    });

                    // Store the task handle so we can stop it later
                    let mut handles = self.watcher_handles.lock().unwrap();
                    handles.insert(project_path.to_path_buf(), handle);
                }
                Err(e) => {
                    error!("Failed to create file watcher: {}", e);
                }
            }
        } else {
            error!("Failed to create ignore filter");
        }
    }

    fn handle_file_event(event: &Event) -> HashSet<PathBuf> {
        // Check if this event has actual file paths (real file events)
        let event_paths: Vec<_> = event
            .paths()
            .filter(|(path, _)| {
                // Check if any component of the path matches excluded subdirectories
                // retain only the paths that are not excluded
                !path.components().any(|comp| {
                    EXCLUDED_SUBDIRECTORIES.contains(&comp.as_os_str().to_str().unwrap())
                })
            })
            .collect();

        let mut retained_paths = HashSet::new();

        // sanitize the paths to remove the /private prefix (macos) and future OS specific prefixes
        for (path, file_type) in &event_paths {
            let sanitized_path = path
                .strip_prefix("/private")
                .map(|p| Path::new("/").join(p))
                .unwrap_or_else(|_| path.to_path_buf());

            if let Some(ft) = file_type {
                debug!("  File type: {:?}", ft);
            }
            retained_paths.insert(sanitized_path);
        }

        retained_paths
    }
}

// NOTE: I implemented this because I'm not sure why server is not gracefully exiting
impl Drop for Watcher {
    fn drop(&mut self) {
        info!("Watcher dropping, shutting down file watchers and event processors");

        // Cancel the cancellation token to signal all background tasks to shut down
        self.cancellation_token.cancel();

        // Stop all running watcher tasks
        if let Ok(mut handles) = self.watcher_handles.lock() {
            for (path, handle) in handles.drain() {
                info!("Stopping watcher task for: {:?}", path);
                handle.abort();
            }
        }

        // Clear the events map to stop processing
        if let Ok(mut events) = self.project_events.lock() {
            events.clear();
        }

        // Clear the debounce windows
        if let Ok(mut windows) = self.debounce_windows.lock() {
            windows.clear();
        }

        // Clear watched folders
        if let Ok(mut watched_folders) = self.watched_project_folders.lock() {
            watched_folders.clear();
        }

        info!("Watcher cleanup complete");
    }
}
