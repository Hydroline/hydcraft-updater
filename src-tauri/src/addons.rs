use crate::{contracts::UpdateManifest, proxy, state::UpdaterState};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

pub async fn apply_selection(
    state: &UpdaterState,
    manifest: &UpdateManifest,
    ids: &[String],
) -> Result<(), String> {
    let proxy_base = proxy::start(
        state.console_origin.clone(),
        manifest.client_version.clone(),
        state.access_token.read().await.clone(),
    )
    .await?;
    let proxy_origin = proxy_base.trim_end_matches("/client/");
    for category in manifest
        .categories
        .iter()
        .filter(|category| ids.contains(&category.id))
    {
        let target = installation_target(&state.game_dir, &category.install_target)?;
        for artifact in &category.artifacts {
            let destination = safe_join(&target, &artifact.relative_path)?;
            let url = format!(
                "{}/addons/{}/{}",
                proxy_origin,
                urlencoding::encode(&category.id),
                artifact
                    .relative_path
                    .split('/')
                    .map(urlencoding::encode)
                    .collect::<Vec<_>>()
                    .join("/")
            );
            download_verified(&url, &destination, &artifact.sha256, artifact.size).await?;
        }
    }
    let state_path = state
        .game_dir
        .join(".hydcraft")
        .join("addon-selection.json");
    let parent = state_path
        .parent()
        .ok_or_else(|| "HydCraft state directory is invalid".to_string())?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| error.to_string())?;
    tokio::fs::write(
        state_path,
        serde_json::to_vec_pretty(ids).map_err(|error| error.to_string())?,
    )
    .await
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn installation_target(game_dir: &Path, target: &str) -> Result<PathBuf, String> {
    safe_join(game_dir, target)
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::Prefix(_) | Component::RootDir
            )
        })
    {
        return Err(format!("Unsafe addon path: {relative}"));
    }
    Ok(root.join(path))
}

async fn download_verified(
    url: &str,
    destination: &Path,
    expected_hash: &str,
    expected_size: u64,
) -> Result<(), String> {
    let response = reqwest::get(url).await.map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Addon download failed: {}", response.status()));
    }
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if bytes.len() as u64 != expected_size {
        return Err(format!("Addon size mismatch: {}", destination.display()));
    }
    let actual_hash = hex::encode(Sha256::digest(&bytes));
    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
        return Err(format!("Addon hash mismatch: {}", destination.display()));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| "Addon destination is invalid".to_string())?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| error.to_string())?;
    let temporary = destination.with_extension("hydcraft-part");
    tokio::fs::write(&temporary, &bytes)
        .await
        .map_err(|error| error.to_string())?;
    tokio::fs::rename(&temporary, destination)
        .await
        .map_err(|error| error.to_string())
}
