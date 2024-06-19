use serde::{Deserialize, Serialize};

pub mod telemetry;
pub mod stats;

#[derive(Deserialize,Serialize, Debug)]
pub struct TelemetryData {
    pub ip_address : String,
    pub isp_info : String,
    pub extra : String,
    pub user_agent : String,
    pub lang : String,
    pub download : String,
    pub upload : String,
    pub ping : String,
    pub jitter : String,
    pub log : String,
    pub uuid : String,
    pub timestamp : i64,
}