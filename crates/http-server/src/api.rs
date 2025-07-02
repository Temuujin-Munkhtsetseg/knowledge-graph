use crate::endpoints::{info::InfoEndpointDef, workspace_index::WorkspaceIndexEndpointDef};
use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
#[derive(Default)]
pub struct ApiContract {
    pub info: InfoEndpointDef,
    pub workspace_index: WorkspaceIndexEndpointDef,
}
