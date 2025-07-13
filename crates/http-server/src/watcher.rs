use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use workspace_manager::WorkspaceManager;

// use tempfile::TempDir;
use watchexec::Watchexec;
use watchexec_events::Event;
// use watchexec_events::{Priority};
use ignore_files::{IgnoreFilesFromOriginArgs, IgnoreFilter};
use watchexec_filterer_ignore::IgnoreFilterer;
use watchexec_signals;

use crate::queue::JobDispatcher;
use crate::queue::job::{Job, JobPriority};
use std::collections::HashSet;

const DEBOUNCE_DURATION: Duration = Duration::from_millis(2000);
const MAX_EVENTS_PER_DEBOUNCE_WINDOW: usize = 8192;
const EXCLUDED_SUBDIRECTORIES: &[&str] = &[".git", ".idea", ".vscode"];

pub struct Watcher {
    pub workspace_manager: Arc<WorkspaceManager>,
    pub job_dispatcher: Arc<JobDispatcher>,
    // Map of workspace path to its events, grouped by debounce windows
    workspace_events: Arc<Mutex<HashMap<PathBuf, Vec<Vec<Event>>>>>,
    // Track the start time of the current debounce window for each workspace
    debounce_windows: Arc<Mutex<HashMap<PathBuf, Instant>>>,
    event_sender: mpsc::Sender<(PathBuf, Vec<Event>)>,
}

impl Watcher {
    pub fn new(
        workspace_manager: Arc<WorkspaceManager>,
        job_dispatcher: Arc<JobDispatcher>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(MAX_EVENTS_PER_DEBOUNCE_WINDOW);
        let job_dispatcher_clone = job_dispatcher.clone();
        let watcher = Self {
            workspace_manager,
            job_dispatcher,
            workspace_events: Arc::new(Mutex::new(HashMap::new())),
            debounce_windows: Arc::new(Mutex::new(HashMap::new())),
            event_sender: tx,
        };

        let runtime = tokio::runtime::Handle::current();
        runtime.spawn(async move {
            Self::process_events(rx, job_dispatcher_clone).await;
        });

        watcher
    }

    // TODO: another bug where we have to CTRL+C for the actual job to be dispatched.
    async fn process_events(
        mut rx: mpsc::Receiver<(PathBuf, Vec<Event>)>,
        job_dispatcher: Arc<JobDispatcher>,
    ) {
        while let Some((path, events)) = rx.recv().await {
            info!("\nProcessing events for workspace: {path:?}");
            info!("Events in group: {}", events.len());

            let mut changed_paths = HashSet::new();
            for event in events {
                let paths = event.paths();
                for (path, _) in paths {
                    changed_paths.insert(path.to_path_buf());
                }
            }

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

    // TODO: we need to somehow listen changes from the workspace manager (does this emit events?) to account for new workspaces being added.
    // TODO: we need to pass messages via channel that is passed into the watcher, that spawns a re-indexing thread, for a specific workspace.

    pub async fn start(&self) {
        let workspace_manager = self.workspace_manager.clone();
        let project_paths = workspace_manager
            .list_all_projects()
            .iter()
            .map(|p| PathBuf::from(p.project_path.clone()))
            .collect::<Vec<_>>();

        info!(
            "Starting file watcher for {} workspace folders",
            project_paths.len()
        );
        info!(
            "Watcher is excluding the following (sub)directories: {:?}",
            EXCLUDED_SUBDIRECTORIES
        );

        for project_path in project_paths {
            self.start_workspace_watcher(&project_path).await;
        }
    }

    async fn start_workspace_watcher(&self, workspace_path: &Path) {
        let args = IgnoreFilesFromOriginArgs::from(workspace_path);
        let (files, _errors) = ignore_files::from_origin(args).await;
        let ignore_filter = IgnoreFilter::new(workspace_path, &files[..]).await;

        if let Ok(ignore_filter) = ignore_filter {
            let ignore_filterer = IgnoreFilterer(ignore_filter);
            let workspace_path_clone = workspace_path.to_path_buf();
            let events_map = self.workspace_events.clone();
            let windows_map = self.debounce_windows.clone();
            let event_sender = self.event_sender.clone();

            match Watchexec::new(move |mut action| {
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
                    // Filter out git-related events
                    let has_git = event.paths().any(|(path, _)| {
                        path.components().any(|comp| {
                            EXCLUDED_SUBDIRECTORIES.contains(&comp.as_os_str().to_str().unwrap())
                        })
                    });

                    if !has_git {
                        current_group.push(event.clone());
                        Self::handle_file_event(event);
                    }
                }

                // If we have events and debounce window elapsed, process them
                if !current_group.is_empty()
                    && current_time.duration_since(*window_start) >= DEBOUNCE_DURATION
                {
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

                // TODO: handle SIGTERM, SIGKILL
                // The quit is initiated once the action handler returns, not when this method is called.
                if action
                    .signals()
                    .any(|sig| sig == watchexec_signals::Signal::Interrupt)
                {
                    info!("Received interrupt signal - stopping file watcher");
                    action.quit();
                }

                action
            }) {
                Ok(wx) => {
                    wx.config.filterer(ignore_filterer);
                    wx.config
                        .pathset([workspace_path.to_string_lossy().into_owned()]);
                    wx.config.throttle(DEBOUNCE_DURATION);

                    let runtime = tokio::runtime::Handle::current();
                    runtime.spawn(async move {
                        if let Err(e) = wx.main().await {
                            error!("Error in file watcher: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to create file watcher: {}", e);
                }
            }
        } else {
            error!("Failed to create ignore filter");
        }
    }

    fn handle_file_event(event: &Event) {
        // Check if this event has actual file paths (real file events)
        let paths: Vec<_> = event.paths().collect();

        info!("File change detected:");

        // Log the affected files
        for (path, file_type) in paths {
            let relative_path = path
                .strip_prefix("/private")
                .map(|p| Path::new("/").join(p))
                .unwrap_or_else(|_| path.to_path_buf());

            // TODO: expand scope to cover any type of directory exclusion
            // Check if any component in the path is .git
            if relative_path
                .components()
                .any(|comp| comp.as_os_str() == ".git")
            {
                debug!("Ignoring git directory change: {:?}", relative_path);
                continue;
            }

            info!("  Changed file: {:?}", relative_path);

            if let Some(ft) = file_type {
                debug!("  File type: {:?}", ft);
            }
        }

        // Log event kinds
        for tag in &event.tags {
            match tag {
                watchexec_events::Tag::FileEventKind(kind) => {
                    info!("  Event kind: {:?}", kind);
                }
                watchexec_events::Tag::Source(source) => {
                    debug!("  Source: {:?}", source);
                }
                _ => {}
            }
        }
    }

    // Add method to access events for a workspace
    pub fn get_workspace_events(&self, workspace_path: &Path) -> Vec<Vec<Event>> {
        self.workspace_events
            .lock()
            .unwrap()
            .get(&workspace_path.to_path_buf())
            .cloned()
            .unwrap_or_default()
    }
}

// NOTE: I implemented this because I'm not sure why server is not gracefully exiting
impl Drop for Watcher {
    fn drop(&mut self) {
        info!("Watcher dropping, shutting down file watchers and event processors");

        // Clear the events map to stop processing
        if let Ok(mut events) = self.workspace_events.lock() {
            events.clear();
        }

        // Clear the debounce windows
        if let Ok(mut windows) = self.debounce_windows.lock() {
            windows.clear();
        }

        // Close the event channel by dropping the sender
        // This will cause the receiver loop to exit
        drop(self.event_sender.clone());

        info!("Watcher cleanup complete");
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[tokio::test]
    async fn test_watcher() {
        // TODO: add tests
    }
}
