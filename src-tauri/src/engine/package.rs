use super::EngineError;
use crate::contracts::{MigrationEnvelope, UpdatePlan};
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use zip::ZipArchive;

pub(super) fn verify_envelope(value: &MigrationEnvelope) -> Result<(), EngineError> {
    if value.plan.schema_version != 1
        || value.package_urls.is_empty()
        || value.package_size.parse::<u64>().unwrap_or(0) == 0
    {
        return Err(EngineError::Message("更新迁移记录无效".into()));
    }
    Ok(())
}

pub(super) fn verify_package(bytes: &[u8], value: &MigrationEnvelope) -> Result<(), EngineError> {
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
        .map_err(|error| EngineError::Message(error.to_string()))?
        .try_into()
        .map_err(|_| EngineError::Message("更新公钥长度无效".into()))?;
    let signature = Signature::from_slice(
        &STANDARD
            .decode(&value.signature)
            .map_err(|error| EngineError::Message(error.to_string()))?,
    )
    .map_err(|error| EngineError::Message(error.to_string()))?;
    VerifyingKey::from_bytes(&key_bytes)
        .map_err(|error| EngineError::Message(error.to_string()))?
        .verify(bytes, &signature)
        .map_err(|_| EngineError::Message("更新 ZIP 签名校验失败".into()))
}

pub(super) fn extract_plan(bytes: &[u8]) -> Result<UpdatePlan, EngineError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| EngineError::Message(error.to_string()))?;
    let mut entry = archive
        .by_name("update-plan.json")
        .map_err(|_| EngineError::Message("ZIP 缺少 update-plan.json".into()))?;
    let mut json = String::new();
    entry
        .read_to_string(&mut json)
        .map_err(|error| EngineError::Message(error.to_string()))?;
    serde_json::from_str(&json).map_err(|error| EngineError::Message(error.to_string()))
}
