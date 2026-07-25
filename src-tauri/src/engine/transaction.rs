use super::{storage, EngineError};
use crate::{
    contracts::{Anchor, Operation, UpdateConflict, UpdatePlan},
    state::UpdaterState,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaseManifest {
    package_format: String,
    version: String,
}

pub(super) async fn install_base_package(
    state: &UpdaterState,
    bytes: &[u8],
    version: &str,
    mode: &str,
) -> Result<(), EngineError> {
    let game = &state.game_dir;
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| EngineError::Message(error.to_string()))?;
    let mut manifest_bytes = Vec::new();
    {
        let mut manifest_entry = archive
            .by_name("base-manifest.json")
            .map_err(|_| EngineError::Message("完整包缺少 base-manifest.json".into()))?;
        manifest_entry
            .read_to_end(&mut manifest_bytes)
            .map_err(|error| EngineError::Message(error.to_string()))?;
    }
    let manifest: BaseManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| EngineError::Message(format!("完整包清单无效：{error}")))?;
    if manifest.package_format != "hydcraft-base-zip-v1" || manifest.version != version {
        return Err(EngineError::Message("完整包与所选客户端版本不匹配".into()));
    }

    let mods_root = ".minecraft/versions/HydCraft Oxygen/mods/";
    let total_items = archive
        .file_names()
        .filter(|name| {
            let target = name.replace('\\', "/");
            target != "base-manifest.json"
                && target != ".minecraft/hydcraft.json"
                && !(mode == "mods" && !target.starts_with(mods_root))
        })
        .count() as u64;
    let mut completed_items = 0_u64;
    state
        .set_operation_status("extracting", Some(completed_items), Some(total_items))
        .await;
    for index in 0..archive.len() {
        let (target, content) = {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| EngineError::Message(error.to_string()))?;
            if entry.is_dir() || entry.name() == "base-manifest.json" {
                continue;
            }
            let target = entry.name().replace('\\', "/");
            if target == ".minecraft/hydcraft.json"
                || (mode == "mods" && !target.starts_with(mods_root))
            {
                continue;
            }
            let mut content = Vec::new();
            entry
                .read_to_end(&mut content)
                .map_err(|error| EngineError::Message(error.to_string()))?;
            (target, content)
        };
        let destination = storage::safe_join(game, &target)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| EngineError::Message(error.to_string()))?;
        }
        let temporary = destination.with_extension("hydcraft-next");
        fs::write(&temporary, content).map_err(|error| EngineError::Message(error.to_string()))?;
        fs::rename(&temporary, &destination)
            .map_err(|error| EngineError::Message(error.to_string()))?;
        completed_items += 1;
        if completed_items == total_items || completed_items % 10 == 0 {
            state
                .set_operation_status("extracting", Some(completed_items), Some(total_items))
                .await;
        }
    }
    let mut state = storage::load_client_state(game)?;
    state.current_version = version.to_owned();
    state.unfinished_transaction = None;
    storage::save_state(game, &state)
}
use zip::ZipArchive;

pub(super) fn preflight_conflicts(
    game: &Path,
    plan: &UpdatePlan,
    state: &storage::ClientState,
    resolutions: &HashMap<String, String>,
    bytes: &[u8],
    anchors: &[Anchor],
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
            let target_path = storage::safe_join(game, target)?;
            if target_path.is_file()
                && storage::sha256(&target_path)?.eq_ignore_ascii_case(&expected)
            {
                continue;
            }
            if target_path.is_file() {
                if target_matches_verified_anchor(game, target, anchors)? {
                    continue;
                }
                output.push(UpdateConflict {
                    operation_id: id.clone(),
                    target: target.clone(),
                    reason: "目标路径已有内容不同的文件，请确认覆盖".into(),
                    candidates: vec![target.clone()],
                });
            } else {
                let candidates = storage::find_hash(game, &expected)?;
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
            let path = storage::safe_join(game, target)?;
            if !path.is_file() {
                let wanted = expected_sha256.as_deref().or_else(|| {
                    state
                        .managed_files
                        .get(id)
                        .map(|value| value.sha256.as_str())
                });
                let candidates = wanted
                    .map(|hash| storage::find_hash(game, hash))
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
                if !storage::sha256(&path)?.eq_ignore_ascii_case(expected) {
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

fn target_matches_verified_anchor(
    game: &Path,
    target: &str,
    anchors: &[Anchor],
) -> Result<bool, EngineError> {
    let target_path = storage::safe_join(game, target)?;
    for anchor in anchors {
        if storage::safe_join(game, &anchor.path)? == target_path
            && target_path.is_file()
            && storage::sha256(&target_path)?.eq_ignore_ascii_case(&anchor.sha256)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn apply_transaction(
    game: &Path,
    plan: &UpdatePlan,
    bytes: &[u8],
    client_state: &mut storage::ClientState,
    resolutions: &HashMap<String, String>,
) -> Result<(), EngineError> {
    let tx_root = game
        .join(".minecraft")
        .join(".hydcraft")
        .join("backups")
        .join(&plan.migration_id);
    fs::create_dir_all(&tx_root).map_err(|error| EngineError::Message(error.to_string()))?;
    client_state.unfinished_transaction = Some(plan.migration_id.clone());
    storage::save_state(game, client_state)?;
    let result = (|| {
        let mut archive = ZipArchive::new(Cursor::new(bytes))
            .map_err(|error| EngineError::Message(error.to_string()))?;
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
    storage::save_state(game, client_state)
}

pub(super) fn recover_unfinished_transaction(game: &Path) -> Result<bool, EngineError> {
    let mut client_state = storage::load_client_state(game)?;
    let Some(transaction_id) = client_state.unfinished_transaction.clone() else {
        return Ok(false);
    };
    let tx_root = game
        .join(".minecraft")
        .join(".hydcraft")
        .join("backups")
        .join(transaction_id);
    if tx_root.exists() {
        rollback(game, &tx_root)?;
    }
    client_state.unfinished_transaction = None;
    storage::save_state(game, &client_state)?;
    Ok(true)
}

fn apply_operation(
    game: &Path,
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    operation: &Operation,
    state: &mut storage::ClientState,
    resolutions: &HashMap<String, String>,
) -> Result<(), EngineError> {
    match operation {
        Operation::EnsureFile {
            id, source, target, ..
        } => {
            let destination = storage::safe_join(game, target)?;
            let mut item = archive
                .by_name(&format!("payload/{source}"))
                .map_err(|_| EngineError::Message(format!("ZIP 缺少 payload/{source}")))?;
            let mut content = Vec::new();
            item.read_to_end(&mut content)
                .map_err(|error| EngineError::Message(error.to_string()))?;
            let content_hash = hex::encode(Sha256::digest(&content));
            if destination.is_file()
                && storage::sha256(&destination)?.eq_ignore_ascii_case(&content_hash)
            {
                state.managed_files.insert(
                    id.clone(),
                    storage::ManagedFile {
                        sha256: content_hash,
                        path: target.clone(),
                    },
                );
                return Ok(());
            }
            if let Some(selected) = resolutions.get(id) {
                if selected != target {
                    let existing = storage::safe_join(game, selected)?;
                    if existing.is_file()
                        && storage::sha256(&existing)?.eq_ignore_ascii_case(&content_hash)
                    {
                        if let Some(parent) = destination.parent() {
                            fs::create_dir_all(parent)
                                .map_err(|error| EngineError::Message(error.to_string()))?;
                        }
                        fs::rename(existing, &destination)
                            .map_err(|error| EngineError::Message(error.to_string()))?;
                        state.managed_files.insert(
                            id.clone(),
                            storage::ManagedFile {
                                sha256: content_hash,
                                path: target.clone(),
                            },
                        );
                        return Ok(());
                    }
                }
            }
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| EngineError::Message(error.to_string()))?;
            }
            let temporary = destination.with_extension("hydcraft-next");
            fs::write(&temporary, &content)
                .map_err(|error| EngineError::Message(error.to_string()))?;
            fs::rename(&temporary, &destination)
                .map_err(|error| EngineError::Message(error.to_string()))?;
            state.managed_files.insert(
                id.clone(),
                storage::ManagedFile {
                    sha256: content_hash,
                    path: target.clone(),
                },
            );
        }
        Operation::RemoveFile { id, target, .. } => {
            let selected = resolutions.get(id).map(String::as_str).unwrap_or(target);
            let path = storage::safe_join(game, selected)?;
            if path.is_file() {
                fs::remove_file(path).map_err(|error| EngineError::Message(error.to_string()))?;
            }
            state.managed_files.remove(id);
        }
        Operation::ReplaceText {
            target,
            expected,
            replacement,
            ..
        } => {
            let path = storage::safe_join(game, target)?;
            let content = fs::read_to_string(&path)
                .map_err(|error| EngineError::Message(error.to_string()))?;
            if !content.contains(expected) {
                return Err(EngineError::Message(format!(
                    "文本预期内容不匹配：{target}"
                )));
            }
            fs::write(path, content.replacen(expected, replacement, 1))
                .map_err(|error| EngineError::Message(error.to_string()))?;
        }
        Operation::PatchJson {
            target,
            pointer,
            value,
            ..
        } => {
            let path = storage::safe_join(game, target)?;
            let mut json: serde_json::Value = serde_json::from_slice(
                &fs::read(&path).map_err(|error| EngineError::Message(error.to_string()))?,
            )
            .map_err(|error| EngineError::Message(error.to_string()))?;
            *json
                .pointer_mut(pointer)
                .ok_or_else(|| EngineError::Message(format!("JSON 指针不存在：{pointer}")))? =
                value.clone();
            fs::write(
                path,
                serde_json::to_vec_pretty(&json)
                    .map_err(|error| EngineError::Message(error.to_string()))?,
            )
            .map_err(|error| EngineError::Message(error.to_string()))?;
        }
        Operation::PatchToml {
            target,
            key_path,
            value,
            ..
        } => {
            let path = storage::safe_join(game, target)?;
            let mut doc = fs::read_to_string(&path)
                .map_err(|error| EngineError::Message(error.to_string()))?
                .parse::<toml_edit::DocumentMut>()
                .map_err(|error| EngineError::Message(error.to_string()))?;
            let mut node = doc.as_item_mut();
            for key in key_path {
                node = node
                    .get_mut(key)
                    .ok_or_else(|| EngineError::Message(format!("TOML 键不存在：{key}")))?;
            }
            *node = toml_edit::value(value.clone());
            fs::write(path, doc.to_string())
                .map_err(|error| EngineError::Message(error.to_string()))?;
        }
        Operation::PatchProperties {
            target, key, value, ..
        } => {
            let path = storage::safe_join(game, target)?;
            let content = fs::read_to_string(&path)
                .map_err(|error| EngineError::Message(error.to_string()))?;
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
                .map_err(|error| EngineError::Message(error.to_string()))?;
        }
        Operation::EnsureDirectory { target, .. } => {
            fs::create_dir_all(storage::safe_join(game, target)?)
                .map_err(|error| EngineError::Message(error.to_string()))?;
        }
        Operation::RemoveEmptyDirectory { target, .. } => {
            let path = storage::safe_join(game, target)?;
            if path.is_dir()
                && fs::read_dir(&path)
                    .map_err(|error| EngineError::Message(error.to_string()))?
                    .next()
                    .is_none()
            {
                fs::remove_dir(path).map_err(|error| EngineError::Message(error.to_string()))?;
            }
        }
        Operation::AddonActivate { addon_id, .. } => {
            state.addon_state.insert(addon_id.clone(), true);
        }
        Operation::AddonDeactivate { addon_id, .. } => {
            state.addon_state.insert(addon_id.clone(), false);
        }
    }
    Ok(())
}

fn backup(game: &Path, backup_root: &Path, target: &str) -> Result<(), EngineError> {
    let source = storage::safe_join(game, target)?;
    let backup = storage::safe_join(backup_root, target)?;
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent).map_err(|error| EngineError::Message(error.to_string()))?;
    }
    if source.is_file() {
        fs::copy(source, backup).map_err(|error| EngineError::Message(error.to_string()))?;
    } else {
        fs::write(
            PathBuf::from(format!("{}.hydcraft-absent", backup.display())),
            b"",
        )
        .map_err(|error| EngineError::Message(error.to_string()))?;
    }
    Ok(())
}

fn rollback(game: &Path, backup_root: &Path) -> Result<(), EngineError> {
    for entry in walkdir::WalkDir::new(backup_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let relative = entry
            .path()
            .strip_prefix(backup_root)
            .map_err(|error| EngineError::Message(error.to_string()))?;
        let relative_text = relative.to_string_lossy();
        if let Some(original_relative) = relative_text.strip_suffix(".hydcraft-absent") {
            let original = game.join(original_relative);
            if original.is_file() {
                fs::remove_file(original)
                    .map_err(|error| EngineError::Message(error.to_string()))?;
            }
            continue;
        }
        let target = game.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| EngineError::Message(error.to_string()))?;
        }
        fs::copy(entry.path(), target).map_err(|error| EngineError::Message(error.to_string()))?;
    }
    Ok(())
}
