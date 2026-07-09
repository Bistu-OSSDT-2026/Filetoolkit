// PDF 处理命令(B-2)
//
// 功能:合并 / 拆分 / 压缩
// 底层使用 lopdf crate

use std::collections::BTreeMap;
use std::path::PathBuf;

use lopdf::{Document, Object, ObjectId};

// ============================================================
// 辅助函数
// ============================================================

/// 解析页码范围字符串,返回 (start_page, end_page),1-indexed。
///
/// 支持格式:
/// - "3" → 单页 3
/// - "1-5" → 第 1 页到第 5 页
/// - "5-" → 第 5 页到末尾
fn parse_range(range: &str, total_pages: u32) -> Result<(u32, u32), String> {
    if let Some((start_str, end_str)) = range.split_once('-') {
        let s = start_str.trim();
        if s.is_empty() {
            return Err(format!("无效页码范围: {}", range));
        }
        let start: u32 = s.parse().map_err(|_| format!("无效页码: {}", s))?;
        if start < 1 || start > total_pages {
            return Err(format!("页码 {} 超出范围 (1-{})", start, total_pages));
        }
        let end_str = end_str.trim();
        let end = if end_str.is_empty() {
            total_pages
        } else {
            let e: u32 = end_str.parse().map_err(|_| format!("无效页码: {}", end_str))?;
            if e > total_pages {
                return Err(format!("页码 {} 超出范围 (1-{})", e, total_pages));
            }
            e
        };
        if start > end {
            return Err(format!("起始页码 {} 不能大于结束页码 {}", start, end));
        }
        Ok((start, end))
    } else {
        // 单页
        let p: u32 = range.trim().parse().map_err(|_| format!("无效页码: {}", range))?;
        if p < 1 || p > total_pages {
            return Err(format!("页码 {} 超出范围 (1-{})", p, total_pages));
        }
        Ok((p, p))
    }
}

/// 格式化文件大小为人类可读形式
fn format_size(size: u64) -> String {
    if size >= 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else if size >= 1024 {
        format!("{:.0} KB", size as f64 / 1024.0)
    } else {
        format!("{} B", size)
    }
}

// ============================================================
// 内部合并实现(基于 lopdf 官方示例)
// ============================================================

/// 合并多个 Document 为一个。
fn merge_documents(docs: Vec<Document>) -> Result<Document, String> {
    if docs.is_empty() {
        return Err("没有文档可合并".into());
    }
    if docs.len() == 1 {
        return Ok(docs.into_iter().next().unwrap());
    }

    let mut max_id: u32 = 1;
    let mut target = Document::with_version("1.7");

    let mut all_page_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut all_other_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();

    let mut catalog_info: Option<(ObjectId, Object)> = None;
    let mut pages_info: Option<(ObjectId, Object)> = None;

    for mut doc in docs {
        // 重新编号避免 ObjectId 冲突
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        let pages = doc.get_pages();

        // 收集 Page 对象
        for (_, object_id) in pages {
            if let Ok(obj) = doc.get_object(object_id) {
                all_page_objects.insert(object_id, obj.clone());
            }
        }

        // 分类收集其他对象
        for (id, obj) in doc.objects {
            match obj.type_name().unwrap_or("") {
                "Catalog" => {
                    catalog_info = Some((id, obj));
                }
                "Pages" => {
                    if let Ok(dict) = obj.as_dict() {
                        let mut dict = dict.clone();
                        if let Some((_, ref old_obj)) = pages_info {
                            if let Ok(old_dict) = old_obj.as_dict() {
                                dict.extend(old_dict);
                            }
                        }
                        pages_info = Some((id, Object::Dictionary(dict)));
                    }
                }
                "Page" => {}     // 已单独处理
                "Outlines" => {} // 忽略
                "Outline" => {}  // 忽略
                _ => {
                    all_other_objects.insert(id, obj);
                }
            }
        }
    }

    // 将非页对象插入目标文档
    for (id, obj) in all_other_objects {
        target.objects.insert(id, obj);
    }

    // 设置 Pages 对象
    let (page_id, page_obj) = pages_info.ok_or_else(|| "未找到 Pages 根对象".to_string())?;

    if let Ok(dict) = page_obj.as_dict() {
        let mut dict = dict.clone();
        dict.set("Count", all_page_objects.len() as u32);
        dict.set(
            "Kids",
            all_page_objects.keys().map(|&id| Object::Reference(id)).collect::<Vec<_>>(),
        );
        target.objects.insert(page_id, Object::Dictionary(dict));
    }

    // 插入 Page 对象,更新 Parent 指向
    for (object_id, object) in all_page_objects {
        if let Ok(dict) = object.as_dict() {
            let mut dict = dict.clone();
            dict.set("Parent", page_id);
            target.objects.insert(object_id, Object::Dictionary(dict));
        }
    }

    // 设置 Catalog
    let (catalog_id, catalog_obj) = catalog_info.ok_or_else(|| "未找到 Catalog 根对象".to_string())?;
    if let Ok(dict) = catalog_obj.as_dict() {
        let mut dict = dict.clone();
        dict.set("Pages", page_id);
        dict.remove(b"Outlines");
        target.objects.insert(catalog_id, Object::Dictionary(dict));
    }

    target.trailer.set("Root", catalog_id);
    target.max_id = target.objects.len() as u32;
    target.renumber_objects();
    target.adjust_zero_pages();

    Ok(target)
}

// ============================================================
// 命令:合并 PDF
// ============================================================

/// 合并多个 PDF 文件为一个。
///
/// - `files`: 输入 PDF 文件路径列表,按此顺序合并
/// - `output_path`: 输出 PDF 文件路径
#[tauri::command]
pub fn merge_pdfs(
    files: Vec<String>,
    output_path: String,
) -> Result<String, String> {
    if files.len() < 2 {
        return Err("至少需要两个 PDF 文件才能合并".into());
    }

    // 加载所有文档
    let mut docs = Vec::new();
    for file in &files {
        let doc = Document::load(file)
            .map_err(|e| format!("加载 PDF 失败 ({}): {}", file, e))?;
        docs.push(doc);
    }

    // 合并
    let mut merged = merge_documents(docs)?;

    // 创建输出目录
    if let Some(parent) = PathBuf::from(&output_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建输出目录失败: {}", e))?;
    }

    merged.save(&output_path)
        .map_err(|e| format!("保存 PDF 失败: {}", e))?;

    // 统计信息
    let total_size: u64 = files.iter()
        .filter_map(|f| std::fs::metadata(f).ok().map(|m| m.len()))
        .sum();

    Ok(format!(
        "合并完成: {} 个文件 → {}\n总大小: {}",
        files.len(),
        output_path,
        format_size(total_size)
    ))
}

// ============================================================
// 命令:拆分 PDF
// ============================================================

/// 按页码范围拆分 PDF。
///
/// - `file`: 输入 PDF 文件路径
/// - `ranges`: 页码范围数组,如 ["1-3", "4-6", "7-"]
/// - `output_dir`: 输出目录
///
/// 返回生成的 PDF 文件路径列表。
#[tauri::command]
pub fn split_pdf(
    file: String,
    ranges: Vec<String>,
    output_dir: String,
) -> Result<Vec<String>, String> {
    if ranges.is_empty() {
        return Err("页码范围不能为空".into());
    }

    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("创建输出目录失败: {}", e))?;

    let source_doc = Document::load(&file)
        .map_err(|e| format!("加载 PDF 失败: {}", e))?;
    let total_pages = source_doc.get_pages().len() as u32;

    let mut outputs = Vec::new();

    for range_str in &ranges {
        let (start, end) = parse_range(range_str, total_pages)?;

        // 重新加载(避免克隆大文档)
        let mut doc = Document::load(&file)
            .map_err(|e| format!("加载 PDF 失败: {}", e))?;

        let page_numbers: Vec<u32> = doc.get_pages().keys().copied().collect();

        // 删除不在范围内的页面
        let to_delete: Vec<u32> = page_numbers.iter()
            .filter(|&&p| p < start || p > end)
            .copied()
            .collect();

        if !to_delete.is_empty() {
            doc.delete_pages(&to_delete);
        }

        // 输出文件名
        let end_label = if end == total_pages { "end".to_string() } else { end.to_string() };
        let output_name = format!("split_{}-{}.pdf", start, end_label);
        let output_path = PathBuf::from(&output_dir).join(&output_name);

        doc.save(&output_path)
            .map_err(|e| format!("保存 PDF 失败: {}", e))?;

        outputs.push(output_path.to_string_lossy().to_string());
    }

    Ok(outputs)
}

// ============================================================
// 命令:压缩 PDF
// ============================================================

/// 压缩 PDF 文件(重压缩所有流 + 清理冗余对象)。
///
/// - `file`: 输入 PDF 文件路径
/// - `output_path`: 输出 PDF 文件路径
#[tauri::command]
pub fn compress_pdf(
    file: String,
    output_path: String,
) -> Result<String, String> {
    let original_size = std::fs::metadata(&file)
        .map(|m| m.len()).map_err(|e| format!("读取原文件失败: {}", e))?;

    let mut doc = Document::load(&file)
        .map_err(|e| format!("加载 PDF 失败: {}", e))?;

    // 压缩所有流(FlateDecode)
    doc.compress();

    // 清理冗余对象
    doc.prune_objects();

    if let Some(parent) = PathBuf::from(&output_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建输出目录失败: {}", e))?;
    }

    doc.save(&output_path)
        .map_err(|e| format!("保存 PDF 失败: {}", e))?;

    let new_size = std::fs::metadata(&output_path)
        .map(|m| m.len()).unwrap_or(0);

    let ratio = if original_size > 0 {
        (1.0 - new_size as f64 / original_size as f64) * 100.0
    } else {
        0.0
    };

    Ok(format!(
        "压缩完成: {} → {}, 压缩率 {:.1}%",
        format_size(original_size),
        format_size(new_size),
        ratio
    ))
}
