use crate::{
    contracts::{MigrationEnvelope, Operation, UpdateConflict, UpdatePlan},
    state::UpdaterState,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("conflicts require user confirmation")]
    Conflicts(Vec<UpdateConflict>),
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientUpdateCheck {
    pub current_version: String,
    pub update_available: bool,
    pub to_version: String,
    pub migration_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientVersionOption {
    pub version: String,
    pub label: String,
    pub is_latest: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSource {
    pub key: String,
    pub label: String,
    pub priority: i32,
    pub requires_login: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ClientState {
    current_version: String,
    applied_migrations: Vec<String>,
    managed_files: HashMap<String, ManagedFile>,
    addon_state: HashMap<String, bool>,
    unfinished_transaction: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedFile {
    sha256: String,
    path: String,
}

pub async fn fetch_next(
    state: &UpdaterState,
    version: &str,
    source_key: Option<&str>,
) -> Result<Option<MigrationEnvelope>, EngineError> {
    let mut request = reqwest::Client::new()
        .get(format!(
            "{}/api/updater/migrations/next",
            state.console_origin.trim_end_matches('/')
        ))
        .query(&[("currentVersion", version)]);
    if let Some(source_key) = source_key {
        request = request.query(&[("sourceKey", source_key)]);
    }
    if let Some(token) = state.access_token.read().await.clone() {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?;
    if response.status().as_u16() == 204 {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(EngineError::Message(format!(
            "Migration request failed: {}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map(Some)
        .map_err(|error| EngineError::Message(error.to_string()))
}

pub async fn check_next(
    state: &UpdaterState,
    version: &str,
) -> Result<ClientUpdateCheck, EngineError> {
    let mut request = reqwest::Client::new()
        .get(format!(
            "{}/api/updater/check",
            state.console_origin.trim_end_matches('/')
        ))
        .query(&[("currentVersion", version)]);
    if let Some(token) = state.access_token.read().await.clone() {
        request = request.bearer_auth(token);
    }
    request
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?
        .error_for_status()
        .map_err(|error| EngineError::Message(error.to_string()))?
        .json()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))
}

pub async fn list_versions(state: &UpdaterState) -> Result<Vec<ClientVersionOption>, EngineError> {
    reqwest::Client::new()
        .get(format!(
            "{}/api/updater/versions",
            state.console_origin.trim_end_matches('/')
        ))
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?
        .error_for_status()
        .map_err(|error| EngineError::Message(error.to_string()))?
        .json()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))
}

pub fn fallback_version_options() -> Vec<ClientVersionOption> {
    vec![ClientVersionOption {
        version: "__no-version__".into(),
        label: "__no-version__".into(),
        is_latest: false,
    }]
}

pub async fn list_sources(
    state: &UpdaterState,
    locale: &str,
) -> Result<Vec<DownloadSource>, EngineError> {
    let mut request = reqwest::Client::new().get(format!(
        "{}/api/updater/sources?locale={locale}",
        state.console_origin.trim_end_matches('/')
    ));
    if let Some(token) = state.access_token.read().await.clone() {
        request = request.bearer_auth(token);
    }
    request
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?
        .error_for_status()
        .map_err(|error| EngineError::Message(error.to_string()))?
        .json()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))
}

pub async fn apply_next(
    state: &UpdaterState,
    selected_version: Option<String>,
    source_key: Option<String>,
) -> Result<Option<String>, EngineError> {
    let mut client_state = load_client_state(&state.game_dir)?;
    if client_state.current_version.is_empty() {
        client_state.current_version = selected_version
            .ok_or_else(|| EngineError::Message("请选择当前客户端版本后再更新".into()))?;
    }
    let migration =
        match fetch_next(state, &client_state.current_version, source_key.as_deref()).await? {
            Some(value) => value,
            None => return Ok(None),
        };
    verify_envelope(&migration)?;
    verify_anchors(&state.game_dir, &migration.anchors)?;
    let bytes = download_package(&migration).await?;
    verify_package(&bytes, &migration)?;
    let extracted = extract_plan(&bytes)?;
    if extracted.migration_id != migration.migration_id
        || extracted.from_version != migration.from_version
        || extracted.to_version != migration.to_version
    {
        return Err(EngineError::Message(
            "ZIP 内更新计划与 Console 迁移记录不一致".into(),
        ));
    }
    let conflicts = preflight_conflicts(
        &state.game_dir,
        &extracted,
        &client_state,
        &*state.resolutions.read().await,
        &bytes,
    )?;
    if !conflicts.is_empty() {
        return Err(EngineError::Conflicts(conflicts));
    }
    apply_transaction(
        &state.game_dir,
        &extracted,
        &bytes,
        &mut client_state,
        &*state.resolutions.read().await,
    )?;
    Ok(Some(migration.to_version))
}

fn verify_envelope(value: &MigrationEnvelope) -> Result<(), EngineError> {
    if value.plan.schema_version != 1
        || value.package_urls.is_empty()
        || value.package_size.parse::<u64>().unwrap_or(0) == 0
    {
        return Err(EngineError::Message("更新迁移记录无效".into()));
    }
    Ok(())
}
async fn download_package(value: &MigrationEnvelope) -> Result<Vec<u8>, EngineError> {
    let client = reqwest::Client::new();
    let mut failure = String::new();
    for url in &value.package_urls {
        match client.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                return response
                    .bytes()
                    .await
                    .map(|v| v.to_vec())
                    .map_err(|e| EngineError::Message(e.to_string()))
            }
            Ok(response) => failure = format!("{}: {}", url, response.status()),
            Err(error) => failure = format!("{}: {}", url, error),
        }
    }
    Err(EngineError::Message(format!("所有下载源均失败：{failure}")))
}
fn verify_package(bytes: &[u8], value: &MigrationEnvelope) -> Result<(), EngineError> {
    if bytes.len().to_string() != value.package_size {
        return Err(EngineError::Message("更新 ZIP 大小校验失败".into()));
    }
    let hash = hex::encode(Sha256::digest(bytes));
    if !hash.eq_ignore_ascii_case(&value.package_sha256) {
        return Err(EngineError::Message("更新 ZIP SHA-256 校验失败".into()));
    }
    let key = std::env::var("HYDCRAFT_UPDATE_PUBLIC_KEY")
        .map_err(|_| EngineError::Message("缺少 HYDCRAFT_UPDATE_PUBLIC_KEY".into()))?;
    let key_bytes: [u8; 32] = STANDARD
        .decode(key)
        .map_err(|e| EngineError::Message(e.to_string()))?
        .try_into()
        .map_err(|_| EngineError::Message("更新公钥长度无效".into()))?;
    let signature = Signature::from_slice(
        &STANDARD
            .decode(&value.signature)
            .map_err(|e| EngineError::Message(e.to_string()))?,
    )
    .map_err(|e| EngineError::Message(e.to_string()))?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| EngineError::Message(e.to_string()))?
        .verify(bytes, &signature)
        .map_err(|_| EngineError::Message("更新 ZIP 签名校验失败".into()))
}
fn extract_plan(bytes: &[u8]) -> Result<UpdatePlan, EngineError> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| EngineError::Message(e.to_string()))?;
    let mut entry = archive
        .by_name("update-plan.json")
        .map_err(|_| EngineError::Message("ZIP 缺少 update-plan.json".into()))?;
    let mut json = String::new();
    entry
        .read_to_string(&mut json)
        .map_err(|e| EngineError::Message(e.to_string()))?;
    serde_json::from_str(&json).map_err(|e| EngineError::Message(e.to_string()))
}
fn state_path(game: &Path) -> PathBuf {
    game.join(".minecraft").join("hydcraft.json")
}

fn existing_state_path(game: &Path) -> Option<PathBuf> {
    [state_path(game), game.join("hydcraft.json")]
        .into_iter()
        .find(|path| path.is_file())
}

pub fn inspect_client_version(game: &Path) -> Result<Option<String>, EngineError> {
    let Some(path) = existing_state_path(game) else {
        return Ok(None);
    };
    let bytes = fs::read(path).map_err(|e| EngineError::Message(e.to_string()))?;
    let state = serde_json::from_slice::<ClientState>(&bytes)
        .map_err(|_| EngineError::Message("hydcraft.json 无法解析".into()))?;
    if state.current_version.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(state.current_version))
}
fn load_client_state(game: &Path) -> Result<ClientState, EngineError> {
    let Some(path) = existing_state_path(game) else {
        return Ok(ClientState::default());
    };
    Ok(
        serde_json::from_slice(&fs::read(path).map_err(|e| EngineError::Message(e.to_string()))?)
            .unwrap_or_default(),
    )
}
fn verify_anchors(game: &Path, anchors: &[crate::contracts::Anchor]) -> Result<(), EngineError> {
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
fn preflight_conflicts(
    game: &Path,
    plan: &UpdatePlan,
    state: &ClientState,
    resolutions: &HashMap<String, String>,
    bytes: &[u8],
) -> Result<Vec<UpdateConflict>, EngineError> {
    let mut output = Vec::new();
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| EngineError::Message(error.to_string()))?;
    for operation in &plan.operations {
        if let Operation::EnsureFile {
            id, source, target, ..
        } = operation
        {
            if resolutions.contains_key(id) {
                continue;
            }
            let mut payload = archive
                .by_name(&format!("payload/{source}"))
                .map_err(|_| EngineError::Message(format!("ZIP 缺少 payload/{source}")))?;
            let mut content = Vec::new();
            payload
                .read_to_end(&mut content)
                .map_err(|error| EngineError::Message(error.to_string()))?;
            let expected = hex::encode(Sha256::digest(&content));
            let target_path = safe_join(game, target)?;
            if target_path.is_file() && sha256(&target_path)?.eq_ignore_ascii_case(&expected) {
                continue;
            }
            if target_path.is_file() {
                output.push(UpdateConflict {
                    operation_id: id.clone(),
                    target: target.clone(),
                    reason: "目标路径已有内容不同的文件，请确认覆盖".into(),
                    candidates: vec![target.clone()],
                });
            } else {
                let candidates = find_hash(game, &expected)?;
                if !candidates.is_empty() {
                    output.push(UpdateConflict {
                        operation_id: id.clone(),
                        target: target.clone(),
                        reason: "发现已提前安装的相同文件，请确认接管并移动到受管路径".into(),
                        candidates,
                    });
                }
            }
        }
        if let Operation::RemoveFile {
            id,
            target,
            expected_sha256,
            ..
        } = operation
        {
            if resolutions.contains_key(id) {
                continue;
            }
            let path = safe_join(game, target)?;
            if !path.is_file() {
                let wanted = expected_sha256.as_deref().or_else(|| {
                    state
                        .managed_files
                        .get(id)
                        .map(|value| value.sha256.as_str())
                });
                let candidates = wanted
                    .map(|hash| find_hash(game, hash))
                    .transpose()?
                    .unwrap_or_default();
                output.push(UpdateConflict {
                    operation_id: id.clone(),
                    target: target.clone(),
                    reason: if candidates.is_empty() {
                        "目标文件缺失，请确认它是否已被手动删除".into()
                    } else {
                        "发现内容相同但位置改变的文件，请确认要删除的目标".into()
                    },
                    candidates,
                });
            } else if let Some(expected) = expected_sha256 {
                if !sha256(&path)?.eq_ignore_ascii_case(expected) {
                    output.push(UpdateConflict {
                        operation_id: id.clone(),
                        target: target.clone(),
                        reason: "目标文件内容已被用户修改".into(),
                        candidates: vec![target.clone()],
                    });
                }
            }
        }
    }
    Ok(output)
}
fn apply_transaction(
    game: &Path,
    plan: &UpdatePlan,
    bytes: &[u8],
    client_state: &mut ClientState,
    resolutions: &HashMap<String, String>,
) -> Result<(), EngineError> {
    let tx_root = game
        .join(".minecraft")
        .join(".hydcraft")
        .join("backups")
        .join(&plan.migration_id);
    fs::create_dir_all(&tx_root).map_err(|e| EngineError::Message(e.to_string()))?;
    client_state.unfinished_transaction = Some(plan.migration_id.clone());
    save_state(game, client_state)?;
    let result = (|| {
        let mut archive =
            ZipArchive::new(Cursor::new(bytes)).map_err(|e| EngineError::Message(e.to_string()))?;
        for operation in &plan.operations {
            if let Some(target) = operation.target() {
                backup(game, &tx_root, target)?;
            }
            if let Some(selected) = resolutions.get(operation.id()) {
                if operation.target().is_some_and(|target| selected != target) {
                    backup(game, &tx_root, selected)?;
                }
            }
            apply_operation(game, &mut archive, operation, client_state, resolutions)?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        rollback(game, &tx_root)?;
        return Err(error);
    }
    client_state.current_version = plan.to_version.clone();
    client_state
        .applied_migrations
        .push(plan.migration_id.clone());
    client_state.unfinished_transaction = None;
    save_state(game, client_state)
}
fn apply_operation(
    game: &Path,
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    operation: &Operation,
    state: &mut ClientState,
    resolutions: &HashMap<String, String>,
) -> Result<(), EngineError> {
    match operation {
        Operation::EnsureFile {
            id, source, target, ..
        } => {
            let destination = safe_join(game, target)?;
            let mut item = archive
                .by_name(&format!("payload/{source}"))
                .map_err(|_| EngineError::Message(format!("ZIP 缺少 payload/{source}")))?;
            let mut content = Vec::new();
            item.read_to_end(&mut content)
                .map_err(|e| EngineError::Message(e.to_string()))?;
            let content_hash = hex::encode(Sha256::digest(&content));
            if destination.is_file() && sha256(&destination)?.eq_ignore_ascii_case(&content_hash) {
                state.managed_files.insert(
                    id.clone(),
                    ManagedFile {
                        sha256: content_hash,
                        path: target.clone(),
                    },
                );
                return Ok(());
            }
            if let Some(selected) = resolutions.get(id) {
                if selected != target {
                    let existing = safe_join(game, selected)?;
                    if existing.is_file() && sha256(&existing)?.eq_ignore_ascii_case(&content_hash)
                    {
                        if let Some(parent) = destination.parent() {
                            fs::create_dir_all(parent)
                                .map_err(|error| EngineError::Message(error.to_string()))?;
                        }
                        fs::rename(existing, &destination)
                            .map_err(|error| EngineError::Message(error.to_string()))?;
                        state.managed_files.insert(
                            id.clone(),
                            ManagedFile {
                                sha256: content_hash,
                                path: target.clone(),
                            },
                        );
                        return Ok(());
                    }
                }
            }
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|e| EngineError::Message(e.to_string()))?;
            }
            let temporary = destination.with_extension("hydcraft-next");
            fs::write(&temporary, &content).map_err(|e| EngineError::Message(e.to_string()))?;
            fs::rename(&temporary, &destination)
                .map_err(|e| EngineError::Message(e.to_string()))?;
            state.managed_files.insert(
                id.clone(),
                ManagedFile {
                    sha256: content_hash,
                    path: target.clone(),
                },
            );
        }
        Operation::RemoveFile { id, target, .. } => {
            let selected = resolutions.get(id).map(String::as_str).unwrap_or(target);
            let path = safe_join(game, selected)?;
            if path.is_file() {
                fs::remove_file(path).map_err(|e| EngineError::Message(e.to_string()))?;
            }
            state.managed_files.remove(id);
        }
        Operation::ReplaceText {
            target,
            expected,
            replacement,
            ..
        } => {
            let path = safe_join(game, target)?;
            let content =
                fs::read_to_string(&path).map_err(|e| EngineError::Message(e.to_string()))?;
            if !content.contains(expected) {
                return Err(EngineError::Message(format!(
                    "文本预期内容不匹配：{target}"
                )));
            }
            fs::write(path, content.replacen(expected, replacement, 1))
                .map_err(|e| EngineError::Message(e.to_string()))?;
        }
        Operation::PatchJson {
            target,
            pointer,
            value,
            ..
        } => {
            let path = safe_join(game, target)?;
            let mut json: serde_json::Value = serde_json::from_slice(
                &fs::read(&path).map_err(|e| EngineError::Message(e.to_string()))?,
            )
            .map_err(|e| EngineError::Message(e.to_string()))?;
            *json
                .pointer_mut(pointer)
                .ok_or_else(|| EngineError::Message(format!("JSON 指针不存在：{pointer}")))? =
                value.clone();
            fs::write(
                path,
                serde_json::to_vec_pretty(&json)
                    .map_err(|e| EngineError::Message(e.to_string()))?,
            )
            .map_err(|e| EngineError::Message(e.to_string()))?;
        }
        Operation::PatchToml {
            target,
            key_path,
            value,
            ..
        } => {
            let path = safe_join(game, target)?;
            let mut doc = fs::read_to_string(&path)
                .map_err(|e| EngineError::Message(e.to_string()))?
                .parse::<toml_edit::DocumentMut>()
                .map_err(|e| EngineError::Message(e.to_string()))?;
            let mut node = doc.as_item_mut();
            for key in key_path {
                node = node
                    .get_mut(key)
                    .ok_or_else(|| EngineError::Message(format!("TOML 键不存在：{key}")))?;
            }
            *node = toml_edit::value(value.clone());
            fs::write(path, doc.to_string()).map_err(|e| EngineError::Message(e.to_string()))?;
        }
        Operation::PatchProperties {
            target, key, value, ..
        } => {
            let path = safe_join(game, target)?;
            let content =
                fs::read_to_string(&path).map_err(|e| EngineError::Message(e.to_string()))?;
            let mut found = false;
            let updated = content
                .lines()
                .map(|line| {
                    if line.trim_start().starts_with(&format!("{key}=")) {
                        found = true;
                        format!("{key}={value}")
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            if !found {
                return Err(EngineError::Message(format!("Properties 键不存在：{key}")));
            }
            fs::write(path, format!("{updated}\n"))
                .map_err(|e| EngineError::Message(e.to_string()))?;
        }
        Operation::EnsureDirectory { target, .. } => {
            fs::create_dir_all(safe_join(game, target)?)
                .map_err(|e| EngineError::Message(e.to_string()))?
        }
        Operation::RemoveEmptyDirectory { target, .. } => {
            let path = safe_join(game, target)?;
            if path.is_dir()
                && fs::read_dir(&path)
                    .map_err(|e| EngineError::Message(e.to_string()))?
                    .next()
                    .is_none()
            {
                fs::remove_dir(path).map_err(|e| EngineError::Message(e.to_string()))?;
            }
        }
        Operation::AddonActivate { addon_id, .. } => {
            state.addon_state.insert(addon_id.clone(), true);
        }
        Operation::AddonDeactivate { addon_id, .. } => {
            state.addon_state.insert(addon_id.clone(), false);
        }
    };
    Ok(())
}
fn safe_join(root: &Path, target: &str) -> Result<PathBuf, EngineError> {
    let path = Path::new(target);
    if path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        return Err(EngineError::Message(format!("非法更新路径：{target}")));
    }
    Ok(root.join(path))
}
fn sha256(path: &Path) -> Result<String, EngineError> {
    Ok(hex::encode(Sha256::digest(
        fs::read(path).map_err(|e| EngineError::Message(e.to_string()))?,
    )))
}
fn find_hash(root: &Path, expected: &str) -> Result<Vec<String>, EngineError> {
    let mut output = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
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
fn backup(game: &Path, backup_root: &Path, target: &str) -> Result<(), EngineError> {
    let source = safe_join(game, target)?;
    let backup = safe_join(backup_root, target)?;
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent).map_err(|e| EngineError::Message(e.to_string()))?;
    }
    if source.is_file() {
        fs::copy(source, backup).map_err(|e| EngineError::Message(e.to_string()))?;
    } else {
        fs::write(
            PathBuf::from(format!("{}.hydcraft-absent", backup.display())),
            b"",
        )
        .map_err(|e| EngineError::Message(e.to_string()))?;
    }
    Ok(())
}
fn rollback(game: &Path, backup_root: &Path) -> Result<(), EngineError> {
    for entry in WalkDir::new(backup_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let relative = entry
            .path()
            .strip_prefix(backup_root)
            .map_err(|e| EngineError::Message(e.to_string()))?;
        let relative_text = relative.to_string_lossy();
        if let Some(original_relative) = relative_text.strip_suffix(".hydcraft-absent") {
            let original = game.join(original_relative);
            if original.is_file() {
                fs::remove_file(original).map_err(|e| EngineError::Message(e.to_string()))?;
            }
            continue;
        }
        let target = game.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| EngineError::Message(e.to_string()))?;
        }
        fs::copy(entry.path(), target).map_err(|e| EngineError::Message(e.to_string()))?;
    }
    Ok(())
}
fn save_state(game: &Path, state: &ClientState) -> Result<(), EngineError> {
    let path = state_path(game);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| EngineError::Message(e.to_string()))?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(state).map_err(|e| EngineError::Message(e.to_string()))?,
    )
    .map_err(|e| EngineError::Message(e.to_string()))
}
