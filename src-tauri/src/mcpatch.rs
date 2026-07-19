use crate::contracts::{SourceDefinition, UpdateManifest};
use crate::state::UpdaterState;
use std::path::PathBuf;
use tokio::process::Command;

pub async fn update_client(
    state: &UpdaterState,
    manifest: &UpdateManifest,
    loopback_url: &str,
) -> Result<(), String> {
    let source_urls = ordered_sources(&manifest.sources, loopback_url);
    let source_lines = source_urls
        .iter()
        .map(|source| format!("  - {}", source))
        .collect::<Vec<_>>()
        .join("\n");
    let jar = std::env::var_os("HYDCRAFT_MCPATCH_JAR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            state
                .game_dir
                .join(".hydcraft")
                .join("tools")
                .join("Mcpatch-0.0.11.jar")
        });
    if !jar.is_file() {
        return Err(format!("MCPatch jar is missing: {}", jar.display()));
    }
    let config_path = jar
        .parent()
        .ok_or_else(|| "MCPatch jar has no parent directory".to_string())?
        .join("mcpatch.yml");
    let config = format!(
        "urls:\n{}\nversion-file-path: ../../version-label.txt\nallow-error: false\nshow-no-update-message: false\nshow-has-update-message: false\nauto-close-changelogs: 0\nsilent-mode: true\ndisable-theme: true\nbase-path: '../..'\nhttp-timeout: 7000\nretries: 3\nignore-ssl-cert: false\n",
        source_lines
    );
    tokio::fs::write(&config_path, config)
        .await
        .map_err(|error| error.to_string())?;
    let java = std::env::var("HYDCRAFT_JAVA").unwrap_or_else(|_| "java".into());
    let status = Command::new(java)
        .arg("-jar")
        .arg(jar)
        .current_dir(&state.game_dir)
        .status()
        .await
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("MCPatch exited with {status}"))
    }
}

fn ordered_sources(sources: &[SourceDefinition], loopback_url: &str) -> Vec<String> {
    let mut public = sources
        .iter()
        .filter(|source| source.kind == "public")
        .collect::<Vec<_>>();
    public.sort_by_key(|source| source.priority);
    let mut public = public
        .into_iter()
        .filter_map(|source| source.base_url.clone())
        .collect::<Vec<_>>();
    public.push(loopback_url.to_string());
    public
}
