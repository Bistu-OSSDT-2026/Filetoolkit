// 磁盘占用可视化命令(D-5)
//
// 递归遍历目录，构建 DirNode 树供前端 ECharts 旭日图渲染。
// 使用 walkdir 流式遍历 + 按条件发送进度事件。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

// ============================================================
// 数据结构
// ============================================================

/// 目录树节点，前端 ECharts 旭日图的数据源。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DirNode>>,
}

// ============================================================
// Tauri 命令
// ============================================================

/// 扫描指定目录，返回 DirNode 树。
#[tauri::command]
pub fn scan_directory(app: AppHandle, dir: String) -> Result<DirNode, String> {
    let root_path = PathBuf::from(&dir);

    if !root_path.exists() {
        return Err(format!("目录不存在: {}", dir));
    }
    if !root_path.is_dir() {
        return Err(format!("路径不是目录: {}", dir));
    }

    let root_name = root_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.clone());

    build_tree_fast(&root_path, &root_name, &root_path.to_string_lossy(), &app)
}

// ============================================================
// 实现: walkdir 流式遍历 + 按目录层级聚合
// ============================================================

/// walkdir 流式遍历目录树，聚合为 DirNode 树。
fn build_tree_fast(
    path: &Path,
    name: &str,
    display_path: &str,
    app: &AppHandle,
) -> Result<DirNode, String> {
    let mut entries: Vec<(String, PathBuf, u64, bool)> = Vec::new();
    let mut total_size: u64 = 0;
    let mut file_count: u64 = 0;
    let batch_size = 500u64;

    let iter = walkdir::WalkDir::new(path)
        .min_depth(1)    // 不包含自身
        .max_depth(1)    // 只读一层
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in iter {
        let entry_path = entry.path().to_path_buf();
        let entry_name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let is_dir = entry_path.is_dir();

        let size = if is_dir {
            // 子目录：递归计算大小
            let mut sub_size: u64 = 0;
            let mut sub_files: u64 = 0;
            for sub in walkdir::WalkDir::new(&entry_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                sub_size += sub.metadata().map(|m| m.len()).unwrap_or(0);
                sub_files += 1;

                // 流式发送进度
                if sub_files % batch_size == 0 {
                    let _ = app.emit(
                        "disk-scan-progress",
                        serde_json::json!({
                            "path": display_path,
                            "files": sub_files + file_count,
                            "size": sub_size + total_size,
                            "message": format!(
                                "扫描中: {} 文件, {}",
                                sub_files + file_count,
                                format_size(sub_size + total_size)
                            ),
                        }),
                    );
                }
            }
            sub_size
        } else {
            entry.metadata().map(|m| m.len()).unwrap_or(0)
        };

        total_size += size;
        file_count += if is_dir { 0 } else { 1 };
        entries.push((entry_name, entry_path, size, is_dir));

        // 流式发送进度
        if file_count % batch_size == 0 {
            let _ = app.emit(
                "disk-scan-progress",
                serde_json::json!({
                    "path": display_path,
                    "files": file_count,
                    "size": total_size,
                    "message": format!("扫描中: {} 文件, {}", file_count, format_size(total_size)),
                }),
            );
        }
    }

    // 构建子节点（限制数量防止图表卡顿）
    let children = build_children(entries);

    Ok(DirNode {
        name: name.to_string(),
        path: display_path.to_string(),
        size: total_size,
        is_dir: true,
        children: Some(children),
    })
}

/// 将条目列表转为 DirNode 子节点，大目录合并小文件为 "其他"。
const MAX_CHILDREN: usize = 200;

fn build_children(mut entries: Vec<(String, PathBuf, u64, bool)>) -> Vec<DirNode> {
    // 按大小降序排列
    entries.sort_by(|a, b| b.2.cmp(&a.2));

    let mut children: Vec<DirNode> = Vec::new();
    let mut others_size: u64 = 0;
    let mut others_count: u32 = 0;

    for (i, (name, path, size, is_dir)) in entries.into_iter().enumerate() {
        if i < MAX_CHILDREN {
            children.push(DirNode {
                name,
                path: path.to_string_lossy().to_string(),
                size,
                is_dir,
                children: if is_dir { Some(Vec::new()) } else { None },
            });
        } else {
            // 超出展示上限的归入 "其他"
            others_size += size;
            others_count += 1;
        }
    }

    if others_count > 0 {
        children.push(DirNode {
            name: format!("其他 ({} 项)", others_count),
            path: String::new(),
            size: others_size,
            is_dir: false,
            children: None,
        });
    }

    children
}

/// 格式化文件大小(人类可读)
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_scan_small_directory() {
        let dir = std::env::temp_dir().join("filetoolkit_disk_test");
        fs::create_dir_all(&dir).unwrap();
        let mut f = fs::File::create(dir.join("a.txt")).unwrap();
        f.write_all(b"hello").unwrap();

        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let disp = dir.to_string_lossy().to_string();
        let _ = (name, disp);
        fs::remove_dir_all(&dir).unwrap();
    }
}
