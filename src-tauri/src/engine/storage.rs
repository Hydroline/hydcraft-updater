use super::EngineError;
use crate::contracts::Anchor;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClientState {
    pub(super) current_version: String,
    pub(super) applied_migrations: Vec<String>,
    pub(super) managed_files: HashMap<String, ManagedFile>,
    pub(super) addon_state: HashMap<String, bool>,
    pub(super) unfinished_transaction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ManagedFile {
    pub(super) sha256: String,
    pub(super) path: String,
}

pub(super) fn state_path(game: &Path) -> PathBuf {
    game.join(".minecraft").join("hydcraft.json")
}

fn existing_state_path(game: &Path) -> Option<PathBuf> {
    [state_path(game), game.join("hydcraft.json")]
        .into_iter()
        .find(|path| path.is_file())
}

pub(super) fn inspect_client_version(game: &Path) -> Result<Option<String>, EngineError> {
    let Some(path) = existing_state_path(game) else {
        return Ok(None);
    };
    let bytes = fs::read(path).map_err(|error| EngineError::Message(error.to_string()))?;
    let state = serde_json::from_slice::<ClientState>(&bytes)
        .map_err(|_| EngineError::Message("hydcraft.json 无法解析".into()))?;
    if state.current_version.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(state.current_version))
}

pub(super) fn load_client_state(game: &Path) -> Result<ClientState, EngineError> {
    let Some(path) = existing_state_path(game) else {
        return Ok(ClientState::default());
    };
    Ok(serde_json::from_slice(
        &fs::read(path).map_err(|error| EngineError::Message(error.to_string()))?,
    )
    .unwrap_or_default())
}

pub(super) fn verify_anchors(game: &Path, anchors: &[Anchor]) -> Result<(), EngineError> {
    for anchor in anchors {
        let path = safe_join(game, &anchor.path)?;
        let actual = sha256(&path)?;
        if !actual.eq_ignore_ascii_case(&anchor.sha256) {
            return Err(EngineError::Message(format!(
                "客户端锚点不匹配：{}",
                anchor.path
            )));
        }
    }
    Ok(())
}

pub(super) fn safe_join(root: &Path, target: &str) -> Result<PathBuf, EngineError> {
    let path = Path::new(target);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        return Err(EngineError::Message(format!("非法更新路径：{target}")));
    }
    Ok(root.join(path))
}

pub(super) fn sha256(path: &Path) -> Result<String, EngineError> {
    Ok(hex::encode(Sha256::digest(fs::read(path).map_err(
        |error| EngineError::Message(error.to_string()),
    )?)))
}

pub(super) fn find_hash(root: &Path, expected: &str) -> Result<Vec<String>, EngineError> {
    let mut output = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        if sha256(entry.path())?.eq_ignore_ascii_case(expected) {
            output.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    Ok(output)
}

pub(super) fn save_state(game: &Path, state: &ClientState) -> Result<(), EngineError> {
    let path = state_path(game);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| EngineError::Message(error.to_string()))?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(state)
            .map_err(|error| EngineError::Message(error.to_string()))?,
    )
    .map_err(|error| EngineError::Message(error.to_string()))
}
