use anyhow::Result;
use std::sync::Arc;
use workspace_manager::WorkspaceManager;

pub struct ListArgs {
    pub projects: bool,
    pub workspace_folders: bool,
    pub header: bool,
}

#[cfg(any(debug_assertions, feature = "dev-tools"))]
pub fn run(workspace_manager: Arc<WorkspaceManager>, args: ListArgs) -> Result<()> {
    if args.workspace_folders {
        let workspace_folders = workspace_manager.list_workspace_folders();
        if args.header {
            println!("Workspace folders:");
        }
        for workspace_folder in workspace_folders {
            // We're printing to stdout, so we don't need to use tracing
            println!("{}", workspace_folder.workspace_folder_path);
        }
    }
    if args.projects {
        let projects = workspace_manager.list_all_projects();
        if args.header {
            println!("Projects:");
        }
        for project in projects {
            println!("{}", project.project_path);
        }
    }
    Ok(())
}

#[cfg(not(any(debug_assertions, feature = "dev-tools")))]
pub fn run(_workspace_manager: Arc<WorkspaceManager>, _args: ListArgs) -> Result<()> {
    anyhow::bail!("List command is not available. Use --features dev-tools to enable.")
}
