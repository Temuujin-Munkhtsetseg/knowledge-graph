use crate::endpoints::{root::RootEndpointDef, workspace_index::WorkspaceIndexEndpointDef};
use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, TS)]
#[ts(export, export_to = "api.ts")]
#[derive(Default)]
pub struct ApiContract {
    pub root: RootEndpointDef,
    pub workspace_index: WorkspaceIndexEndpointDef,
}
