use super::EngineError;
use crate::{
    contracts::{DownloadProgress, MigrationEnvelope},
    state::UpdaterState,
};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{Duration, Instant},
};

fn source_label(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|value| value.host_str().map(str::to_owned))
        .unwrap_or_else(|| url.to_owned())
}

fn package_cache_name(value: &MigrationEnvelope) -> Result<String, EngineError> {
    if value.package_sha256.len() != 64
        || !value
            .package_sha256
            .chars()
            .all(|value| value.is_ascii_hexdigit())
    {
        return Err(EngineError::Message("更新 ZIP SHA-256 格式无效".into()));
    }
    Ok(value.package_sha256.to_ascii_lowercase())
}

fn package_cache_paths(
    state: &UpdaterState,
    value: &MigrationEnvelope,
) -> Result<(PathBuf, PathBuf), EngineError> {
    let name = package_cache_name(value)?;
    let directory = state
        .game_dir
        .join(".minecraft")
        .join(".hydcraft")
        .join("downloads");
    fs::create_dir_all(&directory).map_err(|error| EngineError::Message(error.to_string()))?;
    Ok((
        directory.join(format!("{name}.zip")),
        directory.join(format!("{name}.zip.part")),
    ))
}

fn cached_package(
    path: &PathBuf,
    value: &MigrationEnvelope,
) -> Result<Option<Vec<u8>>, EngineError> {
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|error| EngineError::Message(error.to_string()))?;
    if bytes.len().to_string() != value.package_size {
        fs::remove_file(path).map_err(|error| EngineError::Message(error.to_string()))?;
        return Ok(None);
    }
    let hash = hex::encode(Sha256::digest(&bytes));
    if hash.eq_ignore_ascii_case(&value.package_sha256) {
        return Ok(Some(bytes));
    }
    fs::remove_file(path).map_err(|error| EngineError::Message(error.to_string()))?;
    Ok(None)
}

fn content_range_start(response: &reqwest::Response) -> Option<u64> {
    let value = response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)?
        .to_str()
        .ok()?;
    let range = value.strip_prefix("bytes ")?;
    range.split_once('-')?.0.parse().ok()
}

async fn ensure_success(
    response: reqwest::Response,
    endpoint: &str,
) -> Result<reqwest::Response, EngineError> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    let preview = response
        .bytes()
        .await
        .map(|body| {
            String::from_utf8_lossy(&body)
                .chars()
                .take(256)
                .collect::<String>()
        })
        .unwrap_or_else(|error| format!("响应体读取失败：{error}"));
    Err(EngineError::Message(format!(
        "{endpoint} 请求失败（HTTP {status}，Content-Type: {content_type}）：{preview}"
    )))
}

async fn decode_json<T: DeserializeOwned>(
    response: reqwest::Response,
    endpoint: &str,
) -> Result<T, EngineError> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    let content_encoding = response
        .headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("identity")
        .to_owned();
    let body = response.bytes().await.map_err(|error| {
        EngineError::Message(format!(
            "{endpoint} 响应体读取失败（HTTP {status}，Content-Encoding: {content_encoding}）：{error}"
        ))
    })?;
    serde_json::from_slice(&body).map_err(|error| {
        let preview = String::from_utf8_lossy(&body)
            .chars()
            .take(256)
            .collect::<String>();
        EngineError::Message(format!(
            "{endpoint} 返回了无法解析的响应（HTTP {status}，Content-Type: {content_type}，Content-Encoding: {content_encoding}）：{error}；响应片段：{preview}"
        ))
    })
}

pub(super) async fn fetch_next(
    state: &UpdaterState,
    version: &str,
    source_key: Option<&str>,
) -> Result<Option<MigrationEnvelope>, EngineError> {
    let mut request = reqwest::Client::new()
        .get(format!(
            "{}/api/updater/migrations/next",
            state.console_origin.trim_end_matches('/')
        ))
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
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
    let response = ensure_success(response, "Console 迁移接口").await?;
    decode_json(response, "Console 迁移接口").await.map(Some)
}

pub(super) async fn check_next(
    state: &UpdaterState,
    version: &str,
) -> Result<super::ClientUpdateCheck, EngineError> {
    let mut request = reqwest::Client::new()
        .get(format!(
            "{}/api/updater/check",
            state.console_origin.trim_end_matches('/')
        ))
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .query(&[("currentVersion", version)]);
    if let Some(token) = state.access_token.read().await.clone() {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?;
    let response = ensure_success(response, "Console 更新检查接口").await?;
    decode_json(response, "Console 更新检查接口").await
}

pub(super) async fn list_versions(
    state: &UpdaterState,
) -> Result<Vec<super::ClientVersionOption>, EngineError> {
    let response = reqwest::Client::new()
        .get(format!(
            "{}/api/updater/versions",
            state.console_origin.trim_end_matches('/')
        ))
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?;
    let response = ensure_success(response, "Console 客户端版本接口").await?;
    decode_json(response, "Console 客户端版本接口").await
}

pub(super) async fn list_sources(
    state: &UpdaterState,
    locale: &str,
) -> Result<Vec<super::DownloadSource>, EngineError> {
    let mut request = reqwest::Client::new()
        .get(format!(
            "{}/api/updater/sources?locale={locale}",
            state.console_origin.trim_end_matches('/')
        ))
        .header(reqwest::header::ACCEPT_ENCODING, "identity");
    if let Some(token) = state.access_token.read().await.clone() {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?;
    let response = ensure_success(response, "Console 下载源接口").await?;
    let mut sources: Vec<super::DownloadSource> =
        decode_json(response, "Console 下载源接口").await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(2500))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|error| EngineError::Message(error.to_string()))?;
    let origin = state.console_origin.trim_end_matches('/').to_owned();
    let access_token = state.access_token.read().await.clone();
    for source in &mut sources {
        if !source.available {
            continue;
        }
        source.latency_ms =
            probe_source_latency(&client, &origin, &source.key, access_token.as_deref()).await;
    }
    Ok(sources)
}

async fn probe_source_latency(
    client: &reqwest::Client,
    console_origin: &str,
    source_key: &str,
    access_token: Option<&str>,
) -> Option<u32> {
    let mut request = client.head(format!(
        "{console_origin}/api/updater/sources/{}/probe",
        url::form_urlencoded::byte_serialize(source_key.as_bytes()).collect::<String>()
    ));
    if let Some(token) = access_token {
        request = request.bearer_auth(token);
    }
    let started_at = Instant::now();
    request.send().await.ok()?;
    u32::try_from(started_at.elapsed().as_millis()).ok()
}

pub(super) async fn download_package(
    state: &UpdaterState,
    value: &MigrationEnvelope,
) -> Result<Vec<u8>, EngineError> {
    let client = reqwest::Client::new();
    let mut failures = Vec::with_capacity(value.package_urls.len());
    let expected_size = value
        .package_size
        .parse::<u64>()
        .map_err(|_| EngineError::Message("更新 ZIP 大小格式无效".into()))?;
    let (final_path, partial_path) = package_cache_paths(state, value)?;
    if let Some(bytes) = cached_package(&final_path, value)? {
        state
            .set_download_status(
                "更新包已从本地缓存读取",
                DownloadProgress {
                    source: "cache".into(),
                    downloaded_bytes: expected_size,
                    total_bytes: expected_size,
                    bytes_per_second: 0,
                    latency_ms: 0,
                    resumed: true,
                },
            )
            .await;
        return Ok(bytes);
    }

    for url in &value.package_urls {
        let label = source_label(url);
        let mut resume_from = partial_path
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if resume_from >= expected_size {
            let _ = fs::remove_file(&partial_path);
            resume_from = 0;
        }
        let mut request = client
            .get(url)
            .header(reqwest::header::ACCEPT_ENCODING, "identity");
        if resume_from > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
        }

        let request_started = Instant::now();
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                failures.push(format!("{label}：连接失败（{error}）"));
                continue;
            }
        };
        let latency_ms = request_started
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let status = response.status();
        if resume_from > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT {
            if content_range_start(&response) != Some(resume_from) {
                failures.push(format!("{label}：断点续传位置不匹配"));
                continue;
            }
        } else if resume_from > 0 && status.is_success() {
            resume_from = 0;
        } else if !status.is_success() {
            failures.push(format!("{label}：HTTP {status}"));
            continue;
        }

        let mut downloaded = resume_from;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(resume_from > 0)
            .truncate(resume_from == 0)
            .open(&partial_path)
            .map_err(|error| EngineError::Message(error.to_string()))?;
        let transfer_started = Instant::now();
        let mut last_emit = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .unwrap_or_else(Instant::now);
        let mut response = response;
        let mut transfer_failed = false;
        state
            .set_download_status(
                "正在下载更新包",
                DownloadProgress {
                    source: label.clone(),
                    downloaded_bytes: downloaded,
                    total_bytes: expected_size,
                    bytes_per_second: 0,
                    latency_ms,
                    resumed: resume_from > 0,
                },
            )
            .await;

        loop {
            let Some(chunk) = (match response.chunk().await {
                Ok(chunk) => chunk,
                Err(error) => {
                    failures.push(format!("{label}：读取响应体失败（{error}）"));
                    transfer_failed = true;
                    break;
                }
            }) else {
                break;
            };
            file.write_all(&chunk)
                .map_err(|error| EngineError::Message(error.to_string()))?;
            downloaded += chunk.len() as u64;
            if last_emit.elapsed() >= Duration::from_millis(200) || downloaded >= expected_size {
                let elapsed = transfer_started.elapsed().as_secs_f64().max(0.001);
                let received = downloaded.saturating_sub(resume_from);
                state
                    .set_download_status(
                        "正在下载更新包",
                        DownloadProgress {
                            source: label.clone(),
                            downloaded_bytes: downloaded.min(expected_size),
                            total_bytes: expected_size,
                            bytes_per_second: (received as f64 / elapsed) as u64,
                            latency_ms,
                            resumed: resume_from > 0,
                        },
                    )
                    .await;
                last_emit = Instant::now();
            }
        }
        if transfer_failed {
            continue;
        }
        file.flush()
            .map_err(|error| EngineError::Message(error.to_string()))?;
        drop(file);

        let actual_size = partial_path
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if actual_size != expected_size {
            failures.push(format!(
                "{label}：响应大小异常（声明 {expected_size}，实际 {actual_size}）"
            ));
            continue;
        }
        fs::rename(&partial_path, &final_path)
            .map_err(|error| EngineError::Message(error.to_string()))?;
        let bytes =
            fs::read(&final_path).map_err(|error| EngineError::Message(error.to_string()))?;
        let hash = hex::encode(Sha256::digest(&bytes));
        if !hash.eq_ignore_ascii_case(&value.package_sha256) {
            let _ = fs::remove_file(&final_path);
            let _ = fs::remove_file(&partial_path);
            failures.push(format!("{label}：SHA-256 不匹配（实际 {hash}）"));
            continue;
        }
        state
            .set_download_status(
                "更新包下载完成，正在校验",
                DownloadProgress {
                    source: label,
                    downloaded_bytes: expected_size,
                    total_bytes: expected_size,
                    bytes_per_second: 0,
                    latency_ms,
                    resumed: resume_from > 0,
                },
            )
            .await;
        return Ok(bytes.to_vec());
    }
    Err(EngineError::Message(format!(
        "更新包下载失败：已尝试 {} 个下载源；{}",
        failures.len(),
        failures.join("；")
    )))
}
