use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "api.ts")]
pub struct StatusResponse {
    pub status: String,
}
