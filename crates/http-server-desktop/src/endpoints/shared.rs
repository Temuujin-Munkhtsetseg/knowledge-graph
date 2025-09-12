use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Default, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct StatusResponse {
    pub status: String,
}
