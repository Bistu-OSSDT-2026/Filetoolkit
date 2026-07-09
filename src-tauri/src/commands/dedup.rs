// 重复文件查重命令
//
// 功能:扫描文件夹,按大小+内容哈希(blake3)查找重复文件,支持批量删除
// 算法:两阶段 — 先按文件大小分组预筛,再对同大小的文件进行内容哈希精确比对

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

use rayon::prelude::*;
use serde::Serialize;

/// 扫描结果
#[derive(Serialize, Clone)]
pub struct DuplicateResult {
    pub success: bool,
    pub msg: String,
    pub duplicate_groups: Vec<Vec<String>>,
}

/// 删除结果
#[derive(Serialize, Clone)]
pub struct DeleteResult {
    pub success: bool,
    pub msg: String,
    pub delete_fail: Vec<String>,
}

/// 快速哈希一个文件的 blake3 值（用于同大小文件的精确比对）
fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|e| format!("打开失败 {}: {}", path.display(), e))?;

    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 65536]; // 64KB 缓冲区
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("读取失败 {}: {}", path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// 扫描文件夹,返回重复文件分组。
///
/// # 参数
/// - `folder_path`: 要扫描的文件夹路径
///
/// # 返回
/// - `duplicate_groups`: 每组是一个路径列表,组内文件内容完全相同
#[tauri::command]
pub fn scan_duplicate_files(
    folder_path: String,
) -> Result<DuplicateResult, String> {
    let root = Path::new(&folder_path);
    if !root.is_dir() {
        return Err(format!("不是有效的文件夹: {}", folder_path));
    }

    // 阶段 1: 收集文件并按大小分组
    let mut size_map: HashMap<u64, Vec<String>> = HashMap::new();

    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path().to_string_lossy().to_string();
        if let Ok(meta) = fs::metadata(&path) {
            let size = meta.len();
            size_map.entry(size).or_default().push(path);
        }
    }

    // 阶段 2: 对同大小的文件做内容哈希比对
    let mut groups: Vec<Vec<String>> = Vec::new();

    for (_, paths) in size_map {
        if paths.len() < 2 {
            continue; // 只有一个文件,不可能是重复
        }

        // 计算每个文件的哈希
        let hashed: Vec<(String, String)> = paths
            .par_iter()
            .filter_map(|p| {
                hash_file(Path::new(p))
                    .ok()
                    .map(|hash| (hash, p.clone()))
            })
            .collect();

        // 按哈希值分组
        let mut hash_groups: HashMap<String, Vec<String>> = HashMap::new();
        for (hash, path) in hashed {
            hash_groups.entry(hash).or_default().push(path);
        }

        // 收集有重复的组
        for (_, group) in hash_groups {
            if group.len() >= 2 {
                groups.push(group);
            }
        }
    }

    let total_groups = groups.len();
    let total_files: usize = groups.iter().map(|g| g.len()).sum();

    Ok(DuplicateResult {
        success: true,
        msg: format!(
            "发现 {} 组重复文件,共 {} 个文件",
            total_groups, total_files
        ),
        duplicate_groups: groups,
    })
}

/// 删除指定路径的文件。
///
/// # 参数
/// - `del_paths`: 要删除的文件路径列表
///
/// # 返回
/// - `delete_fail`: 删除失败的文件路径及原因
#[tauri::command]
pub fn delete_duplicate(
    del_paths: Vec<String>,
) -> DeleteResult {
    let mut delete_fail = Vec::new();
    let mut success_count = 0u32;

    for path in &del_paths {
        match fs::remove_file(path) {
            Ok(()) => success_count += 1,
            Err(e) => {
                delete_fail.push(format!("{}: {}", path, e));
            }
        }
    }

    DeleteResult {
        success: delete_fail.is_empty(),
        msg: if delete_fail.is_empty() {
            format!("已删除 {} 个文件", success_count)
        } else {
            format!(
                "删除 {}/{} 个成功, {} 个失败",
                success_count,
                del_paths.len(),
                delete_fail.len()
            )
        },
        delete_fail,
    }
}
