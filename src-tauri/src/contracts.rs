use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationEnvelope {
    pub migration_id: String,
    pub from_version: String,
    pub to_version: String,
    pub package_key: String,
    pub package_urls: Vec<String>,
    pub package_sha256: String,
    pub package_size: String,
    pub signature: String,
    pub plan: UpdatePlan,
    #[serde(default)]
    pub anchors: Vec<Anchor>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlan {
    pub schema_version: u8,
    pub migration_id: String,
    pub from_version: String,
    pub to_version: String,
    #[serde(default)]
    pub anchors: Vec<Anchor>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Anchor {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Operation {
    EnsureFile {
        id: String,
        reason: String,
        source: String,
        target: String,
    },
    RemoveFile {
        id: String,
        reason: String,
        target: String,
        #[serde(default)]
        expected_sha256: Option<String>,
        #[serde(default)]
        logical_id: Option<String>,
    },
    ReplaceText {
        id: String,
        reason: String,
        target: String,
        expected: String,
        replacement: String,
    },
    PatchJson {
        id: String,
        reason: String,
        target: String,
        pointer: String,
        value: Value,
    },
    PatchToml {
        id: String,
        reason: String,
        target: String,
        key_path: Vec<String>,
        value: String,
    },
    PatchProperties {
        id: String,
        reason: String,
        target: String,
        key: String,
        value: String,
    },
    EnsureDirectory {
        id: String,
        reason: String,
        target: String,
    },
    RemoveEmptyDirectory {
        id: String,
        reason: String,
        target: String,
    },
    AddonActivate {
        id: String,
        reason: String,
        addon_id: String,
    },
    AddonDeactivate {
        id: String,
        reason: String,
        addon_id: String,
    },
}

impl Operation {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::EnsureFile { .. } => "ensureFile",
            Self::RemoveFile { .. } => "removeFile",
            Self::ReplaceText { .. } => "replaceText",
            Self::PatchJson { .. } => "patchJson",
            Self::PatchToml { .. } => "patchToml",
            Self::PatchProperties { .. } => "patchProperties",
            Self::EnsureDirectory { .. } => "ensureDirectory",
            Self::RemoveEmptyDirectory { .. } => "removeEmptyDirectory",
            Self::AddonActivate { .. } => "addonActivate",
            Self::AddonDeactivate { .. } => "addonDeactivate",
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::EnsureFile { id, .. }
            | Self::RemoveFile { id, .. }
            | Self::ReplaceText { id, .. }
            | Self::PatchJson { id, .. }
            | Self::PatchToml { id, .. }
            | Self::PatchProperties { id, .. }
            | Self::EnsureDirectory { id, .. }
            | Self::RemoveEmptyDirectory { id, .. }
            | Self::AddonActivate { id, .. }
            | Self::AddonDeactivate { id, .. } => id,
        }
    }

    pub fn target(&self) -> Option<&str> {
        match self {
            Self::EnsureFile { target, .. }
            | Self::RemoveFile { target, .. }
            | Self::ReplaceText { target, .. }
            | Self::PatchJson { target, .. }
            | Self::PatchToml { target, .. }
            | Self::PatchProperties { target, .. }
            | Self::EnsureDirectory { target, .. }
            | Self::RemoveEmptyDirectory { target, .. } => Some(target),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConflict {
    pub operation_id: String,
    pub operation_type: String,
    pub target_action: String,
    pub target: String,
    pub reason: String,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterStatus {
    pub mode: String,
    pub phase: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_kind: Option<String>,
    pub remaining_seconds: Option<u8>,
    pub current_version: Option<String>,
    pub target_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_revision: Option<u32>,
    pub download: Option<DownloadProgress>,
    pub operation: Option<OperationProgress>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub source: String,
    pub source_url: Option<String>,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_second: u64,
    pub latency_ms: u64,
    pub resumed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationProgress {
    pub stage: String,
    pub completed_items: Option<u64>,
    pub total_items: Option<u64>,
}
