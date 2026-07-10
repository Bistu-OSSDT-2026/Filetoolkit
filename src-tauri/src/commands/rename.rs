// 批量重命名命令
//
// 功能:为文件添加前缀/后缀并按顺序编号
// 命名规则: {prefix}_{编号}_{suffix}.{原扩展名}

use std::path::{Path, PathBuf};
use serde::Serialize;

/// 重命名结果
#[derive(Serialize, Clone)]
pub struct RenameResult {
    pub success: bool,
    pub msg: String,
    pub fail_list: Vec<String>,
}

/// 核心重命名逻辑 —— 供 Tauri 命令和流水线 executor 共用。
/// 返回 (成功数量, 新文件路径列表)
pub fn do_rename(
    file_paths: &[String],
    prefix: &str,
    start_num: u32,
    suffix: &str,
    output_dir: Option<&str>,
) -> Result<Vec<String>, Vec<String>> {
    let prefix = prefix;
    let start = start_num;
    let suffix = suffix;
    let mut outputs = Vec::new();
    let mut fail_list = Vec::new();

    for (i, file_path) in file_paths.iter().enumerate() {
        let original = Path::new(file_path);
        let parent = if let Some(od) = output_dir {
            PathBuf::from(od)
        } else {
            original.parent().map(|p| p.to_path_buf()).unwrap_or_default()
        };

        let ext = original.extension().and_then(|e| e.to_str()).unwrap_or("");
        let num = start + i as u32;

        let new_name = match (prefix.is_empty(), suffix.is_empty()) {
            (true, true) => format!("{}.{}", num, ext),
            (true, false) => format!("{}_{}.{}", num, suffix, ext),
            (false, true) => format!("{}_{}.{}", prefix, num, ext),
            (false, false) => {
                if ext.is_empty() {
                    format!("{}_{}_{}", prefix, num, suffix)
                } else {
                    format!("{}_{}_{}.{}", prefix, num, suffix, ext)
                }
            }
        };

        let new_path = parent.join(&new_name);

        if new_path.exists() && new_path != original {
            fail_list.push(format!("{} → {}: 目标已存在", file_path, new_path.display()));
            continue;
        }

        match std::fs::copy(original, &new_path) {
            Ok(_) => outputs.push(new_path.display().to_string()),
            Err(e) => fail_list.push(format!("{}: {}", file_path, e)),
        }
    }

    if fail_list.is_empty() { Ok(outputs) } else { Err(fail_list) }
}

/// Tauri 命令:批量重命名文件
#[tauri::command]
pub fn rename_files(
    file_paths: Vec<String>,
    prefix: Option<String>,
    start_num: Option<u32>,
    suffix: Option<String>,
) -> RenameResult {
    let pfx = prefix.unwrap_or_default();
    let start = start_num.unwrap_or(1);
    let sfx = suffix.unwrap_or_default();
    let total = file_paths.len();

    match do_rename(&file_paths, &pfx, start, &sfx, None) {
        Ok(outputs) => RenameResult {
            success: true,
            msg: format!("全部成功: {}/{}", outputs.len(), total),
            fail_list: vec![],
        },
        Err(fail_list) => {
            let ok = total.saturating_sub(fail_list.len());
            RenameResult {
                success: ok > 0,
                msg: format!("{}/{} 成功, {} 失败", ok, total, fail_list.len()),
                fail_list,
            }
        }
    }
}
