use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
pub struct DoctorInput {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DoctorAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallSource {
    CargoBin,
    LocalBin,
    SystemBin,
    CustomPath,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DoctorReport {
    pub current_version: String,
    pub executable_path: String,
    pub os: String,
    pub arch: String,
    pub latest_tag: Option<String>,
    pub latest_version: Option<String>,
    pub install_source: InstallSource,
    pub matching_asset: Option<DoctorAsset>,
    pub update_available: Option<bool>,
    pub recommended_install_command: String,
    pub recommended_update_command: String,
}
