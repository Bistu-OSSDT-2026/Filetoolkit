// Office 文档转换命令(B-3)
//
// 功能:Word/Excel/PPT ↔ PDF, Word ↔ HTML
// 底层调用 LibreOffice headless 子进程

use std::path::PathBuf;
use std::process::Stdio;

use crate::common::dependency;

/// 支持的文件扩展名列表
const SUPPORTED_EXTENSIONS: &[&str] = &["docx", "doc", "xlsx", "xls", "pptx", "ppt", "odt", "ods", "odp"];

/// 支持的目标格式
const SUPPORTED_TARGETS: &[&str] = &["pdf", "html", "docx", "txt", "csv"];

/// 转换 Office 文档。
///
/// - `input`: 输入文件路径
/// - `target_format`: 目标格式 "pdf" | "html" | "docx" | "txt" | "csv"
/// - `output`: 输出文件路径(需包含目录和文件名)
#[tauri::command]
pub async fn convert_office(
    input: String,
    target_format: String,
    output: String,
) -> Result<String, String> {
    // 1. 参数校验
    let input_path = std::path::Path::new(&input);
    if !input_path.exists() {
        return Err(format!("输入文件不存在: {}", input));
    }

    let ext = input_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!("不支持的输入格式: .{}, 支持的格式: {:?}", ext, SUPPORTED_EXTENSIONS));
    }

    let fmt = target_format.to_lowercase();
    if !SUPPORTED_TARGETS.contains(&fmt.as_str()) {
        return Err(format!("不支持的目标格式: {}, 支持的格式: {:?}", fmt, SUPPORTED_TARGETS));
    }

    // 2. 检查 LibreOffice 依赖
    let soffice_path = dependency::require_dependency("libreoffice")
        .map_err(|e| format!("LibreOffice 未安装: {}", e))?;

    // 3. 准备输出
    let output_path = std::path::Path::new(&output);
    let output_dir = output_path.parent()
        .ok_or_else(|| "无效的输出路径".to_string())?;

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("创建输出目录失败: {}", e))?;

    // 4. 调用 LibreOffice headless
    let status = tokio::process::Command::new(&soffice_path)
        .args([
            "--headless",
            "--convert-to", &fmt,
            "--outdir",
        ])
        .arg(output_dir)
        .arg(&input)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .await
        .map_err(|e| format!("启动 LibreOffice 失败: {}", e))?;

    if !status.success() {
        return Err("文档转换失败,请检查 LibreOffice 是否正确安装".into());
    }

    // 5. LibreOffice 自动生成文件名,可能和我们期望的不同
    // 如果 LibreOffice 生成的文件名与 output 不同,则重命名
    let generated_name = input_path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| format!("{}.{}", s, fmt))
        .unwrap_or_else(|| format!("output.{}", fmt));

    let generated_path = output_dir.join(&generated_name);

    if generated_path.exists() && generated_path != output_path {
        // 删除已存在的 output 文件
        let _ = std::fs::remove_file(&output);
        // 重命名
        std::fs::rename(&generated_path, &output)
            .map_err(|e| format!("重命名输出文件失败: {}", e))?;
    }

    let file_size = std::fs::metadata(&output)
        .map(|m| m.len()).unwrap_or(0);

    Ok(format!(
        "转换完成: {} → {}\n文件大小: {}",
        input,
        output,
        if file_size >= 1024 * 1024 {
            format!("{:.1} MB", file_size as f64 / (1024.0 * 1024.0))
        } else if file_size >= 1024 {
            format!("{:.0} KB", file_size as f64 / 1024.0)
        } else {
            format!("{} B", file_size)
        }
    ))
}
