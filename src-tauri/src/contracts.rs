use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDefinition {
    pub id: String,
    pub kind: String,
    pub base_url: Option<String>,
    pub priority: u32,
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonCategory {
    pub id: String,
    pub title: String,
    pub bucket_prefix: String,
    pub install_target: String,
    pub requires_login: bool,
    pub required_roles: Vec<String>,
    pub required_entitlements: Vec<String>,
    pub artifacts: Vec<AddonArtifact>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonArtifact {
    pub relative_path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub schema_version: u8,
    pub client_version: String,
    pub sources: Vec<SourceDefinition>,
    pub categories: Vec<AddonCategory>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterStatus {
    pub phase: String,
    pub message: String,
    pub remaining_seconds: Option<u8>,
}
