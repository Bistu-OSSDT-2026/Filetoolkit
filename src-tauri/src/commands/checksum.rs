use serde::{Deserialize, Serialize};
use std::io::Read;
use tauri::AppHandle;
use md5;
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};
use blake3;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FileVerifyItem {
    pub path: String,
    pub expected: String,
    pub algorithm: String,
}

#[tauri::command]
pub async fn verify_checksum(
    app_handle: AppHandle,
    file: String,
    expected: String,
    algorithm: String,
) -> Result<bool, String> {
    let real_hash = compute_checksum(app_handle, file, algorithm).await?;
    Ok(real_hash.eq_ignore_ascii_case(&expected))
}

#[tauri::command]
pub async fn batch_verify(
    app_handle: AppHandle,
    files: Vec<FileVerifyItem>,
) -> Result<Vec<(String, bool)>, String> {
    let mut result_list = Vec::new();
    for item in files {
        let match_flag = verify_checksum(
            app_handle.clone(),
            item.path.clone(),
            item.expected,
            item.algorithm,
        )
        .await?;
        result_list.push((item.path, match_flag));
    }
    Ok(result_list)
}

async fn compute_checksum(
    _app_handle: AppHandle,
    file: String,
    algorithm: String,
) -> Result<String, String> {
    let mut file = std::fs::File::open(&file).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;

    match algorithm.to_lowercase().as_str() {
        "md5" => {
            let hash = md5::compute(&buffer);
            Ok(hex::encode(hash.0))
        }
        "sha1" => {
            let mut hasher = Sha1::new();
            hasher.update(&buffer);
            Ok(hex::encode(hasher.finalize()))
        }
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(&buffer);
            Ok(hex::encode(hasher.finalize()))
        }
        "sha512" => {
            let mut hasher = Sha512::new();
            hasher.update(&buffer);
            Ok(hex::encode(hasher.finalize()))
        }
        "blake3" => {
            let hash = blake3::hash(&buffer);
            Ok(hex::encode(hash.as_bytes()))
        }
        _ => Err("不支持的算法".to_string()),
    }
}