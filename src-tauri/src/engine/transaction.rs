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
    state.last_transaction = None;
    state.unfinished_transaction = None;
    storage::save_state(game, &state)?;
    let _ = storage::clear_directory(&storage::backups_path(game));
    Ok(())
}
use zip::ZipArchive;

const SKIP_RESOLUTION: &str = "__hydcraft_skip__";

pub(super) fn preflight_conflicts(
    game: &Path,
    plan: &UpdatePlan,
    state: &storage::ClientState,
    resolutions: &HashMap<String, String>,
    bytes: &[u8],
    anchors: &[Anchor],
    anchor_mismatches: &[Anchor],
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
                    operation_type: "ensureFile".into(),
                    target_action: "overwrite".into(),
                    target: target.clone(),
                    reason: "目标路径已有内容不同的文件，请确认覆盖".into(),
                    candidates: vec![target.clone()],
                });
            } else {
                let candidates = storage::find_hash(game, &expected)?;
                if !candidates.is_empty() {
                    output.push(UpdateConflict {
                        operation_id: id.clone(),
                        operation_type: "ensureFile".into(),
                        target_action: "install".into(),
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
                    operation_type: "removeFile".into(),
                    target_action: "acknowledgeMissing".into(),
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
                        operation_type: "removeFile".into(),
                        target_action: "delete".into(),
                        target: target.clone(),
                        reason: "目标文件内容已被用户修改".into(),
                        candidates: vec![target.clone()],
                    });
                }
            }
        }
        if resolutions.contains_key(operation.id()) {
            continue;
        }
        let issue = match operation {
            Operation::ReplaceText {
                target,
                expected,
                replacement,
                ..
            } => match fs::read_to_string(storage::safe_join(game, target)?) {
                Ok(content) if content.contains(expected) || content.contains(replacement) => None,
                Ok(_) => Some("找不到预期文本，无法安全替换".into()),
                Err(_) => Some("目标文本文件缺失或无法读取".into()),
            },
            Operation::PatchJson {
                target, pointer, ..
            } => {
                let path = storage::safe_join(game, target)?;
                match fs::read(&path)
                    .ok()
                    .and_then(|content| serde_json::from_slice::<serde_json::Value>(&content).ok())
                {
                    Some(value) if value.pointer(pointer).is_some() => None,
                    Some(_) => Some(format!("JSON 指针不存在：{pointer}")),
                    None => Some("JSON 文件缺失或格式无效".into()),
                }
            }
            Operation::PatchToml {
                target, key_path, ..
            } => {
                let path = storage::safe_join(game, target)?;
                match fs::read_to_string(&path)
                    .ok()
                    .and_then(|content| content.parse::<toml_edit::DocumentMut>().ok())
                {
                    Some(document) => {
                        let mut node = document.as_item();
                        let mut missing_key = None;
                        for key in key_path {
                            let Some(next) = node.get(key) else {
                                missing_key = Some(key);
                                break;
                            };
                            node = next;
                        }
                        missing_key.map(|key| format!("TOML 键不存在：{key}"))
                    }
                    None => Some("TOML 文件缺失或格式无效".into()),
                }
            }
            Operation::PatchProperties { target, key, .. } => {
                match fs::read_to_string(storage::safe_join(game, target)?) {
                    Ok(content)
                        if content
                            .lines()
                            .any(|line| line.trim_start().starts_with(&format!("{key}="))) =>
                    {
                        None
                    }
                    Ok(_) => Some(format!("Properties 键不存在：{key}")),
                    Err(_) => Some("Properties 文件缺失或无法读取".into()),
                }
            }
            Operation::RemoveEmptyDirectory { target, .. } => {
                let path = storage::safe_join(game, target)?;
                if !path.exists()
                    || (path.is_dir()
                        && fs::read_dir(&path)
                            .map_err(|error| EngineError::Message(error.to_string()))?
                            .next()
                            .is_none())
                {
                    None
                } else {
                    Some("目录不为空，无法安全删除".into())
                }
            }
            _ => None,
        };
        if let Some(reason) = issue {
            output.push(UpdateConflict {
                operation_id: operation.id().to_owned(),
                operation_type: operation.type_name().into(),
                target_action: "apply".into(),
                target: operation.target().unwrap_or_default().to_owned(),
                reason,
                candidates: operation.target().into_iter().map(str::to_owned).collect(),
            });
        }
    }
    for anchor in anchor_mismatches {
        if plan
            .operations
            .iter()
            .any(|operation| operation.target() == Some(&anchor.path))
        {
            continue;
        }
        let operation_id = format!("verify-anchor:{}", anchor.path);
        if resolutions.contains_key(&operation_id) {
            continue;
        }
        output.push(UpdateConflict {
            operation_id,
            operation_type: "verifyAnchor".into(),
            target_action: "confirm".into(),
            target: anchor.path.clone(),
            reason: "客户端文件与当前版本不一致，请确认是否继续处理本次迁移".into(),
            candidates: vec![anchor.path.clone()],
        });
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
) -> Result<bool, EngineError> {
    let tx_root = game
        .join(".minecraft")
        .join(".hydcraft")
        .join("backups")
        .join(&plan.migration_id);
    if tx_root.exists() {
        fs::remove_dir_all(&tx_root).map_err(|error| EngineError::Message(error.to_string()))?;
    }
    fs::create_dir_all(&tx_root).map_err(|error| EngineError::Message(error.to_string()))?;
    let previous_managed_files = client_state.managed_files.clone();
    let previous_addon_state = client_state.addon_state.clone();
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
    let partially_applied = plan.operations.iter().any(|operation| {
        resolutions
            .get(operation.id())
            .is_some_and(|resolution| resolution == SKIP_RESOLUTION)
    });
    if partially_applied {
        client_state.last_transaction = Some(storage::CompletedTransaction {
            migration_id: plan.migration_id.clone(),
            from_version: plan.from_version.clone(),
            to_version: plan.to_version.clone(),
            previous_managed_files: previous_managed_files.clone(),
            previous_addon_state: previous_addon_state.clone(),
        });
        client_state.unfinished_transaction = None;
        storage::save_state(game, client_state)?;
        return Ok(true);
    }
    client_state.current_version = plan.to_version.clone();
    client_state
        .applied_migrations
        .push(plan.migration_id.clone());
    client_state.last_transaction = Some(storage::CompletedTransaction {
        migration_id: plan.migration_id.clone(),
        from_version: plan.from_version.clone(),
        to_version: plan.to_version.clone(),
        previous_managed_files,
        previous_addon_state,
    });
    client_state.unfinished_transaction = None;
    storage::save_state(game, client_state)?;
    Ok(false)
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
        let _ = fs::remove_dir_all(&tx_root);
    }
    client_state.unfinished_transaction = None;
    storage::save_state(game, &client_state)?;
    Ok(true)
}

pub(super) fn rollback_last_update(game: &Path) -> Result<(), EngineError> {
    let mut client_state = storage::load_client_state(game)?;
    if client_state.unfinished_transaction.is_some() {
        return Err(EngineError::Message(
            "当前仍有未完成的更新，完成恢复后才能回滚".into(),
        ));
    }
    let Some(transaction) = client_state.last_transaction.clone() else {
        return Err(EngineError::Message("没有可回滚的客户端更新".into()));
    };
    let tx_root = storage::transaction_backup_path(game, &transaction.migration_id)?;
    if !tx_root.is_dir() {
        return Err(EngineError::Message(
            "回滚备份不存在，无法回滚客户端".into(),
        ));
    }

    rollback(game, &tx_root)?;
    client_state.current_version = transaction.from_version;
    client_state.managed_files = transaction.previous_managed_files;
    client_state.addon_state = transaction.previous_addon_state;
    client_state
        .applied_migrations
        .retain(|migration_id| migration_id != &transaction.migration_id);
    client_state.last_transaction = None;
    client_state.unfinished_transaction = None;
    storage::save_state(game, &client_state)?;
    let _ = fs::remove_dir_all(tx_root);
    Ok(())
}

fn apply_operation(
    game: &Path,
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    operation: &Operation,
    state: &mut storage::ClientState,
    resolutions: &HashMap<String, String>,
) -> Result<(), EngineError> {
    if resolutions
        .get(operation.id())
        .is_some_and(|resolution| resolution == SKIP_RESOLUTION)
    {
        return Ok(());
    }
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
                if content.contains(replacement) {
                    return Ok(());
                }
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
