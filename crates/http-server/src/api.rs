use crate::endpoints::{
    events::EventsEndpointDef, info::InfoEndpointDef, workspace_delete::WorkspaceDeleteEndpointDef,
    workspace_index::WorkspaceIndexEndpointDef, workspace_list::WorkspaceListEndpointDef,
};
use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, TS)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
#[derive(Default)]
pub struct ApiContract {
    pub info: InfoEndpointDef,
    pub workspace_index: WorkspaceIndexEndpointDef,
    pub workspace_list: WorkspaceListEndpointDef,
    pub workspace_delete: WorkspaceDeleteEndpointDef,
    pub index: WorkspaceIndexEndpointDef,
    pub events: EventsEndpointDef,
}
