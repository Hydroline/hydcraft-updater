use super::EngineError;
use crate::contracts::Anchor;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{BufReader, Read},
    path::{Component, Path, PathBuf},
};
use walkdir::WalkDir;

#[derive(Default)]
pub(super) struct HashIndex {
    paths_by_hash: HashMap<String, Vec<String>>,
}

impl HashIndex {
    pub(super) fn insert(&mut self, root: &Path, path: &Path, hash: String) {
        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        self.paths_by_hash
            .entry(hash)
            .or_default()
            .push(relative_path);
    }

    pub(super) fn find(&self, expected: &str) -> Vec<String> {
        self.paths_by_hash
            .get(&expected.to_ascii_lowercase())
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClientState {
    pub(super) current_version: String,
    pub(super) applied_migrations: Vec<String>,
    pub(super) managed_files: HashMap<String, ManagedFile>,
    pub(super) addon_state: HashMap<String, bool>,
    pub(super) unfinished_transaction: Option<String>,
    #[serde(default)]
    pub(super) last_transaction: Option<CompletedTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CompletedTransaction {
    pub(super) migration_id: String,
    pub(super) from_version: String,
    pub(super) to_version: String,
    #[serde(default)]
    pub(super) previous_managed_files: HashMap<String, ManagedFile>,
    #[serde(default)]
    pub(super) previous_addon_state: HashMap<String, bool>,
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

pub(super) fn hydcraft_path(game: &Path) -> PathBuf {
    game.join(".hydcraft")
}

pub(super) fn downloads_path(game: &Path) -> PathBuf {
    hydcraft_path(game).join("downloads")
}

pub(super) fn backups_path(game: &Path) -> PathBuf {
    hydcraft_path(game).join("backups")
}

pub(super) fn transaction_backup_path(
    game: &Path,
    migration_id: &str,
) -> Result<PathBuf, EngineError> {
    safe_join(&backups_path(game), migration_id)
}

pub(super) fn directory_size(path: &Path) -> Result<u64, EngineError> {
    if !path.is_dir() {
        return Ok(0);
    }
    let mut size = 0_u64;
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        size = size.saturating_add(
            entry
                .metadata()
                .map_err(|error| EngineError::Message(error.to_string()))?
                .len(),
        );
    }
    Ok(size)
}

pub(super) fn clear_directory(path: &Path) -> Result<(), EngineError> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|error| EngineError::Message(error.to_string()))?;
    }
    fs::create_dir_all(path).map_err(|error| EngineError::Message(error.to_string()))
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

pub(super) fn mismatched_anchors(
    game: &Path,
    anchors: &[Anchor],
) -> Result<Vec<Anchor>, EngineError> {
    let mut mismatches = Vec::new();
    for anchor in anchors {
        let path = safe_join(game, &anchor.path)?;
        let matches = path.is_file() && sha256(&path)?.eq_ignore_ascii_case(&anchor.sha256);
        if !matches {
            mismatches.push(anchor.clone());
        }
    }
    Ok(mismatches)
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
    let file = fs::File::open(path).map_err(|error| EngineError::Message(error.to_string()))?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut buffer = [0_u8; 1024 * 1024];
    let mut hasher = Sha256::new();
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| EngineError::Message(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub(super) fn hash_index_files(root: &Path) -> Vec<PathBuf> {
    let internal_root = hydcraft_path(root);
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| entry.path() != internal_root)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect()
}

pub(super) fn save_state(game: &Path, state: &ClientState) -> Result<(), EngineError> {
    let path = state_path(game);
    if !path.parent().is_some_and(Path::is_dir) {
        return Err(EngineError::Message("客户端缺少 .minecraft 目录".into()));
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(state)
            .map_err(|error| EngineError::Message(error.to_string()))?,
    )
    .map_err(|error| EngineError::Message(error.to_string()))
}
