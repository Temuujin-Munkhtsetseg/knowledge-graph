use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use ignore_files::{IgnoreFilesFromOriginArgs, IgnoreFilter};
use watchexec::Watchexec;
use watchexec_events::Event;
use watchexec_filterer_ignore::IgnoreFilterer;
// use watchexec_events::{Priority};
// use watchexec_signals;

use crate::queue::JobDispatcher;
use crate::queue::job::{Job, JobPriority};
use workspace_manager::WorkspaceManager;

const IGNORE_FILTER_TIMEOUT: Duration = Duration::from_secs(30);
const WATCHER_SPAWN_INTERVAL: Duration = Duration::from_millis(200);
const DEBOUNCE_DURATION: Duration = Duration::from_millis(3000);
const MAX_EVENTS_PER_DEBOUNCE_WINDOW: usize = 8192;
const EXCLUDED_SUBDIRECTORIES: &[&str] = &[".git", ".idea", ".vscode"];

pub struct Watcher {
    // Used to list all projects and their paths
    pub workspace_manager: Arc<WorkspaceManager>,
    // Set of watched workspace folders for which we have a watcher active
    pub watched_workspace_folders: Arc<Mutex<HashSet<PathBuf>>>,
    // Used to trigger reindexing jobs
    pub job_dispatcher: Arc<JobDispatcher>,
    // Map of workspace path to its events, grouped by debounce windows
    workspace_events: Arc<Mutex<HashMap<PathBuf, Vec<Vec<PathBuf>>>>>,
    // Track the start time of the current debounce window for each workspace
    debounce_windows: Arc<Mutex<HashMap<PathBuf, Instant>>>,
    // Track the task handles for each workspace watcher so we can stop them
    watcher_handles: Arc<Mutex<HashMap<PathBuf, JoinHandle<()>>>>,
    // For sending the changed paths to the job dispatcher
    event_sender: mpsc::Sender<(PathBuf, Vec<PathBuf>)>,
    // Current runtime for the watcher
    runtime: tokio::runtime::Handle,
    // Cancellation token for graceful shutdown
    cancellation_token: CancellationToken,
}

impl Watcher {
    pub fn new(
        workspace_manager: Arc<WorkspaceManager>,
        job_dispatcher: Arc<JobDispatcher>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(MAX_EVENTS_PER_DEBOUNCE_WINDOW);
        let job_dispatcher_clone = job_dispatcher.clone();
        let cancellation_token = CancellationToken::new();
        let watcher = Self {
            workspace_manager,
            watched_workspace_folders: Arc::new(Mutex::new(HashSet::new())),
            job_dispatcher,
            workspace_events: Arc::new(Mutex::new(HashMap::new())),
            debounce_windows: Arc::new(Mutex::new(HashMap::new())),
            watcher_handles: Arc::new(Mutex::new(HashMap::new())),
            event_sender: tx,
            runtime: tokio::runtime::Handle::current(),
            cancellation_token,
        };

        watcher.runtime.spawn(async move {
            Self::process_events(rx, job_dispatcher_clone).await;
        });

        watcher
    }

    async fn process_events(
        mut rx: mpsc::Receiver<(PathBuf, Vec<PathBuf>)>,
        job_dispatcher: Arc<JobDispatcher>,
    ) {
        while let Some((path, changed_paths)) = rx.recv().await {
            if changed_paths.is_empty() {
                info!("No changed paths, skipping reindexing job dispatch");
                continue;
            }

            info!("\nProcessing events for workspace: {path:?}");
            info!("changed paths in group: {}", changed_paths.len());
            info!("Changed paths: {changed_paths:?}");

            let job = Job::ReindexWorkspaceFolderWithWatchedFiles {
                workspace_folder_path: path.to_string_lossy().into_owned(),
                workspace_changes: changed_paths.into_iter().collect(),
                priority: JobPriority::Normal,
            };

            info!("Watcher dispatching re-indexing job: {:?}", job);
            let job_id = job_dispatcher.dispatch(job).await.unwrap();
            info!("Watcher dispatched re-indexing job with id: {:?}", job_id);
        }
    }

    async fn stop_abandoned_watchers(&self, project_paths: &[PathBuf]) {
        let mut watched_folders = self.watched_workspace_folders.lock().unwrap();
        let current_paths: HashSet<PathBuf> = project_paths.iter().cloned().collect();
        let folders_to_remove: Vec<PathBuf> = watched_folders
            .difference(&current_paths)
            .cloned()
            .collect();

        for folder in folders_to_remove {
            info!(
                "Stopping workspace watcher for removed folder: {:?}",
                folder
            );
            // Join on the abandoned watchers
            if let Ok(mut handles) = self.watcher_handles.lock() {
                if let Some(handle) = handles.remove(&folder) {
                    handle.abort();
                }
            }

            // Remove events, debounce windows, and watched folder
            if let Ok(mut events) = self.workspace_events.lock() {
                events.remove(&folder);
            }
            if let Ok(mut windows) = self.debounce_windows.lock() {
                windows.remove(&folder);
            }
            watched_folders.remove(&folder);
        }
    }

    async fn monitor_workspace_folders(watcher: Arc<Watcher>) {
        loop {
            if watcher.cancellation_token.is_cancelled() {
                info!("Workspace folder monitoring shutting down");
                break;
            }

            let project_paths = watcher
                .workspace_manager
                .list_all_projects()
                .iter()
                .map(|p| PathBuf::from(p.project_path.clone()))
                .collect::<Vec<_>>();

            watcher.stop_abandoned_watchers(&project_paths).await;

            let paths_needing_watchers = {
                let mut watched_folders = watcher.watched_workspace_folders.lock().unwrap();

                // Find project paths that don't have watchers yet
                let paths_needing_watchers: Vec<PathBuf> = project_paths
                    .into_iter()
                    .filter(|project_path| !watched_folders.contains(project_path))
                    .collect();

                // Mark these paths as being watched (optimistically)
                for path in &paths_needing_watchers {
                    watched_folders.insert(path.clone());
                }

                paths_needing_watchers
            };

            for project_path in paths_needing_watchers {
                info!("Starting new workspace watcher for: {:?}", project_path);
                watcher.start_workspace_watcher(&project_path).await;
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
    }

    async fn resolve_ignore_filter(
        workspace_path: &Path,
    ) -> Result<IgnoreFilterer, Box<dyn std::error::Error>> {
        // Move git config reading to blocking context to avoid Rc<gix_config::file::Metadata> Send issues
        // Note: This is due to the fact that the ignore_files crate uses gix_config::file::Metadata which is not Send
        let workspace_path_clone = workspace_path.to_path_buf();
        let ignore_filter = tokio::time::timeout(
            IGNORE_FILTER_TIMEOUT,
            tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async {
                    let args = IgnoreFilesFromOriginArgs::from(workspace_path_clone.as_path());
                    let (files, _errors) = ignore_files::from_origin(args).await;
                    IgnoreFilter::new(&workspace_path_clone, &files[..]).await
                })
            }),
        )
        .await
        .map_err(|_| "Timeout resolving ignore filter")?
        .expect("Failed to resolve ignore filter")?;
        Ok(IgnoreFilterer(ignore_filter))
    }

    async fn start_workspace_watcher(&self, workspace_path: &Path) {
        if let Ok(ignore_filterer) = Self::resolve_ignore_filter(workspace_path).await {
            let workspace_path_clone = workspace_path.to_path_buf();
            let events_map = self.workspace_events.clone();
            let windows_map = self.debounce_windows.clone();
            let event_sender = self.event_sender.clone();

            match Watchexec::new(move |action| {
                debug!(
                    "Received watchexec action with {} events",
                    action.events.len()
                );

                let current_time = Instant::now();
                let mut windows = windows_map.lock().unwrap();
                let mut events = events_map.lock().unwrap();

                // Get or create window start time for this workspace
                let window_start = windows
                    .entry(workspace_path_clone.clone())
                    .or_insert(current_time);

                // Get the current group of events for this workspace
                let workspace_events = events.entry(workspace_path_clone.clone()).or_default();

                // Create first group if none exists
                if workspace_events.is_empty() {
                    workspace_events.push(Vec::new());
                }

                // Add events to the current group
                let current_group = workspace_events.last_mut().unwrap();

                for event in action.events.iter() {
                    current_group.extend(Self::handle_file_event(event));
                }

                // If we have events and debounce window elapsed, process them
                if current_time.duration_since(*window_start) >= DEBOUNCE_DURATION {
                    *window_start = current_time;
                    let events_to_process = workspace_events.pop().unwrap();
                    workspace_events.push(Vec::new());

                    let path = workspace_path_clone.clone();
                    let sender = event_sender.clone();
                    tokio::spawn(async move {
                        if let Err(e) = sender.send((path, events_to_process)).await {
                            error!("Failed to send events for processing: {}", e);
                        }
                    });
                }
                action
            }) {
                Ok(wx) => {
                    wx.config.filterer(ignore_filterer);
                    wx.config
                        .pathset([workspace_path.to_string_lossy().into_owned()]);
                    wx.config.throttle(DEBOUNCE_DURATION);

                    let handle = self.runtime.spawn(async move {
                        if let Err(e) = wx.main().await {
                            error!("Error in file watcher: {}", e);
                        }
                    });

                    // Store the task handle so we can stop it later
                    let mut handles = self.watcher_handles.lock().unwrap();
                    handles.insert(workspace_path.to_path_buf(), handle);
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
                !path.components().any(|comp| {
                    EXCLUDED_SUBDIRECTORIES.contains(&comp.as_os_str().to_str().unwrap())
                })
            })
            .collect();

        let mut retained_paths = HashSet::new();

        // retain only the paths that are not excluded
        for (path, file_type) in &event_paths {
            let relative_path = path
                .strip_prefix("/private")
                .map(|p| Path::new("/").join(p))
                .unwrap_or_else(|_| path.to_path_buf());

            if let Some(ft) = file_type {
                debug!("  File type: {:?}", ft);
            }
            retained_paths.insert(relative_path);
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
        if let Ok(mut events) = self.workspace_events.lock() {
            events.clear();
        }

        // Clear the debounce windows
        if let Ok(mut windows) = self.debounce_windows.lock() {
            windows.clear();
        }

        // Clear watched folders
        if let Ok(mut watched_folders) = self.watched_workspace_folders.lock() {
            watched_folders.clear();
        }

        info!("Watcher cleanup complete");
    }
}
