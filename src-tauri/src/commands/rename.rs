// 批量重命名命令
//
// 功能:为文件添加前缀/后缀并按顺序编号
// 命名规则: {prefix}_{编号}_{suffix}.{原扩展名}

use std::path::Path;
use serde::Serialize;

/// 重命名结果
#[derive(Serialize, Clone)]
pub struct RenameResult {
    pub success: bool,
    pub msg: String,
    pub fail_list: Vec<String>,
}

/// 批量重命名文件。
///
/// # 参数
/// - `file_paths`: 要重命名的文件路径列表
/// - `prefix`: 文件名前缀（可选）
/// - `start_num`: 起始编号（可选，默认 1）
/// - `suffix`: 文件名后缀（可选）
///
/// # 命名规则
/// `{prefix}_{编号}_{suffix}.{原扩展名}`
#[tauri::command]
pub fn rename_files(
    file_paths: Vec<String>,
    prefix: Option<String>,
    start_num: Option<u32>,
    suffix: Option<String>,
) -> RenameResult {
    let prefix = prefix.unwrap_or_default();
    let start = start_num.unwrap_or(1);
    let suffix = suffix.unwrap_or_default();
    let mut fail_list = Vec::new();
    let mut success_count = 0u32;

    for (i, file_path) in file_paths.iter().enumerate() {
        let original = Path::new(file_path);
        let parent = match original.parent() {
            Some(p) => p,
            None => {
                fail_list.push(format!("{}: 无法获取父目录", file_path));
                continue;
            }
        };

        let ext = original
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // 构建新文件名: {prefix}_{编号}_{suffix}.ext
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

        if new_path.exists() {
            fail_list.push(format!(
                "{} → {}: 目标已存在",
                file_path,
                new_path.display()
            ));
            continue;
        }

        match std::fs::rename(original, &new_path) {
            Ok(()) => success_count += 1,
            Err(e) => {
                fail_list.push(format!("{}: {}", file_path, e));
            }
        }
    }

    let total = file_paths.len();
    RenameResult {
        success: fail_list.is_empty(),
        msg: if fail_list.is_empty() {
            format!("全部成功: {}/{}", success_count, total)
        } else {
            format!("{}/{} 成功, {} 失败", success_count, total, fail_list.len())
        },
        fail_list,
    }
}
