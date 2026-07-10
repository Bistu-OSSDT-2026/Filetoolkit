// 图片批量处理命令(B-1)
//
// 功能:批量压缩/格式转换/调整尺寸
// 底层使用 image crate + BatchRunner 并行框架

use std::io::Cursor;
use std::path::{Path, PathBuf};

use tauri::AppHandle;
use image::imageops::FilterType;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::webp::WebPEncoder;

use crate::common::error::AppError;
use crate::common::types::{Task, TaskResult};
use crate::worker::BatchRunner;

// ============================================================
// 辅助函数
// ============================================================

/// 根据输出格式生成输出文件路径。
/// 如果文件已存在,自动添加数字后缀避免覆盖。
fn get_output_path(input: &str, output_dir: &str, format: &str) -> String {
    let path = Path::new(input);
    let stem = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = match format {
        "jpg" | "jpeg" => "jpg",
        "png" => "png",
        "webp" => "webp",
        _ => format,
    };

    let base = PathBuf::from(output_dir).join(format!("{}.{}", stem, ext));
    if !base.exists() {
        return base.to_string_lossy().to_string();
    }

    // 文件已存在,加后缀避免覆盖
    let mut i = 1;
    loop {
        let candidate = PathBuf::from(output_dir).join(format!("{}_{}.{}", stem, i, ext));
        if !candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
        i += 1;
    }
}

/// 处理单张图片:打开 → 可选 resize → 编码 → 写入
fn process_single_image(
    task: &Task,
    runner: &BatchRunner,
) -> Result<String, AppError> {
    // 检查取消
    if runner.is_cancelled() {
        return Err(AppError::TaskCancelled);
    }

    // 从 params 中提取参数
    let quality: u8 = task.params.get("quality")
        .and_then(|v| v.as_u64()).map(|v| v as u8).unwrap_or(80);
    let format: String = task.params.get("format")
        .and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| "jpg".into());
    let max_width: Option<u32> = task.params.get("maxWidth")
        .and_then(|v| v.as_u64()).map(|v| v as u32);
    let max_height: Option<u32> = task.params.get("maxHeight")
        .and_then(|v| v.as_u64()).map(|v| v as u32);

    // 打开图片
    let img = image::open(&task.input_path)
        .map_err(|e| AppError::ProcessingFailed(
            format!("无法打开图片 {}: {}", task.input_path, e)
        ))?;

    let mut img = img;

    // 调整尺寸(仅当超出限制时)
    match (max_width, max_height) {
        (Some(w), Some(h)) => {
            if img.width() > w || img.height() > h {
                img = img.resize(w, h, FilterType::Lanczos3);
            }
        }
        (Some(w), None) => {
            if img.width() > w {
                img = img.resize(w, u32::MAX, FilterType::Lanczos3);
            }
        }
        (None, Some(h)) => {
            if img.height() > h {
                img = img.resize(u32::MAX, h, FilterType::Lanczos3);
            }
        }
        _ => {}
    }

    // 创建输出目录
    if let Some(parent) = Path::new(&task.output_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Io(format!("创建目录失败: {}", e)))?;
    }

    // 根据格式编码并保存
    match format.as_str() {
        "jpg" | "jpeg" => {
            let mut buf = Vec::new();
            {
                let encoder = JpegEncoder::new_with_quality(
                    Cursor::new(&mut buf), quality
                );
                img.write_with_encoder(encoder)
                    .map_err(|e| AppError::ProcessingFailed(
                        format!("JPEG 编码失败: {}", e)
                    ))?;
            }
            std::fs::write(&task.output_path, &buf)
                .map_err(|e| AppError::Io(format!("写入文件失败: {}", e)))?;
        }
        "png" => {
            let mut buf = Vec::new();
            img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
                .map_err(|e| AppError::ProcessingFailed(
                    format!("PNG 编码失败: {}", e)
                ))?;
            std::fs::write(&task.output_path, &buf)
                .map_err(|e| AppError::Io(format!("写入文件失败: {}", e)))?;
        }
        "webp" => {
            let mut buf = Vec::new();
            // WebP 编码(当前 image crate 仅支持无损编码)
            let encoder = WebPEncoder::new_lossless(&mut buf);
            img.write_with_encoder(encoder)
                .map_err(|e| AppError::ProcessingFailed(
                    format!("WebP 编码失败: {}", e)
                ))?;
            std::fs::write(&task.output_path, &buf)
                .map_err(|e| AppError::Io(format!("写入文件失败: {}", e)))?;
        }
        _ => {
            return Err(AppError::UnsupportedFormat(format));
        }
    }

    Ok(format!("处理完成: {}", task.input_path))
}

// ============================================================
// Tauri 命令
// ============================================================

/// 批量压缩/转换图片。
///
/// # 参数
///
/// - `files`: 输入文件路径列表
/// - `quality`: 质量 0-100(JPEG/WebP 有效,PNG 为无损)
/// - `format`: 输出格式 "jpg" | "png" | "webp"
/// - `max_width`: 可选,最大宽度(超出则等比缩小)
/// - `max_height`: 可选,最大高度(超出则等比缩小)
/// - `output_dir`: 输出目录
///
/// # 返回
///
/// `Vec<TaskResult>` 每个文件处理结果,包含原大小和新大小。
#[tauri::command]
pub async fn compress_images(
    app: AppHandle,
    files: Vec<String>,
    quality: u8,
    format: String,
    max_width: Option<u32>,
    max_height: Option<u32>,
    output_dir: String,
) -> Result<Vec<TaskResult>, String> {
    // 1. 参数校验
    if quality > 100 {
        return Err("质量参数必须在 0-100 之间".into());
    }
    let fmt = format.to_lowercase();
    if !["jpg", "jpeg", "png", "webp"].contains(&fmt.as_str()) {
        return Err(format!(
            "不支持的输出格式: {}, 可选: jpg, png, webp", format
        ));
    }
    if files.is_empty() {
        return Err("文件列表不能为空".into());
    }

    // 2. 创建输出目录
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("创建输出目录失败: {}", e))?;

    // 3. 构建任务列表
    let tasks: Vec<Task> = files.iter().map(|f| {
        let output = get_output_path(f, &output_dir, &fmt);
        Task {
            id: uuid::Uuid::new_v4().to_string(),
            input_path: f.clone(),
            output_path: output,
            status: crate::common::types::TaskStatus::Pending,
            params: serde_json::json!({
                "quality": quality,
                "format": fmt,
                "maxWidth": max_width,
                "maxHeight": max_height,
            }),
        }
    }).collect();

    // 4. 用 BatchRunner 并行执行
    let result = tokio::task::spawn_blocking(move || {
        let runner = BatchRunner::new(app);
        runner.run(tasks, |task, runner| {
            process_single_image(task, runner)
        })
    }).await
        .map_err(|e| format!("任务执行失败: {}", e))?
        .map_err(|e| e.to_string())?;

    Ok(result)
}
