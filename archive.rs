// 批量解压/压缩命令(B-4)
//
// 功能:ZIP/7z/RAR/TAR.GZ 批量解压, ZIP 批量打包(支持加密)
// 底层使用 zip/flate2/tar crate + 外部工具(7z)子进程

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::common::error::{AppError, AppResult};
use crate::common::types::{Task, TaskResult, TaskStatus};
use crate::worker::BatchRunner;

// ============================================================
// 辅助函数
// ============================================================

/// 获取文件扩展名(小写)
fn get_ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default()
}

/// 是否为支持的压缩包格式
fn is_archive_ext(ext: &str) -> bool {
    matches!(ext, "zip" | "tar" | "gz" | "tgz" | "7z" | "rar")
}

/// 解压单个 ZIP 文件
fn extract_zip(input: &str, output_dir: &Path) -> AppResult<String> {
    let file = std::fs::File::open(input)
        .map_err(|e| AppError::Io(format!("打开文件失败: {}", e)))?;

    let mut archive = zip::read::ZipArchive::new(file)
        .map_err(|e| AppError::ProcessingFailed(format!("读取 ZIP 失败: {}", e)))?;

    archive.extract(output_dir)
        .map_err(|e| AppError::ProcessingFailed(format!("解压 ZIP 失败: {}", e)))?;

    Ok(format!("解压完成: {}", input))
}

/// 解压单个 TAR.GZ 文件
fn extract_tar_gz(input: &str, output_dir: &Path) -> AppResult<String> {
    let file = std::fs::File::open(input)
        .map_err(|e| AppError::Io(format!("打开文件失败: {}", e)))?;

    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    archive.unpack(output_dir)
        .map_err(|e| AppError::ProcessingFailed(format!("解压 TAR.GZ 失败: {}", e)))?;

    Ok(format!("解压完成: {}", input))
}

/// 解压单个 7z 文件(通过调用 7z 子进程)
fn extract_7z(input: &str, output_dir: &Path) -> AppResult<String> {
    // 尝试在 PATH 中查找 7z
    let seven_zip = find_in_path("7z")
        .or_else(|| find_in_path("7za"))
        .or_else(|| find_in_path("7zr"));

    let seven_zip = seven_zip
        .ok_or_else(|| AppError::DependencyNotFound(
            "未找到 7z 命令。请安装 7-Zip 并将其添加到 PATH。".into()
        ))?;

    let status = std::process::Command::new(&seven_zip)
        .args(["x", input, format!("-o{}", output_dir.display()).as_str(), "-y"])
        .status()
        .map_err(|e| AppError::ProcessingFailed(format!("运行 7z 失败: {}", e)))?;

    if status.success() {
        Ok(format!("解压完成: {}", input))
    } else {
        Err(AppError::ProcessingFailed(format!("7z 解压失败: {}", input)))
    }
}

/// 在 PATH 中查找可执行文件
fn find_in_path(exe_name: &str) -> Option<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(if cfg!(windows) { ';' } else { ':' }) {
            // Windows 上尝试 .exe 和 .bat 扩展名
            let candidates = if cfg!(windows) {
                vec![
                    PathBuf::from(dir).join(format!("{}.exe", exe_name)),
                    PathBuf::from(dir).join(format!("{}.bat", exe_name)),
                    PathBuf::from(dir).join(exe_name),
                ]
            } else {
                vec![PathBuf::from(dir).join(exe_name)]
            };
            for candidate in candidates {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// 创建 ZIP 压缩包
fn create_zip(
    files: &[String],
    output_path: &str,
    password: Option<&str>,
) -> AppResult<String> {
    let file = std::fs::File::create(output_path)
        .map_err(|e| AppError::Io(format!("创建 ZIP 文件失败: {}", e)))?;

    let mut writer = zip::write::ZipWriter::new(file);

    for path_str in files {
        let path = Path::new(path_str);
        if !path.exists() {
            continue;
        }

        let name = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        if path.is_file() {
            let options = if let Some(pwd) = password {
                zip::write::FileOptions::<()>::default()
                    .with_aes_encryption(zip::AesMode::Aes256, pwd)
            } else {
                zip::write::FileOptions::<()>::default()
            };

            writer.start_file(name, options)
                .map_err(|e| AppError::ProcessingFailed(format!("写入 ZIP 条目失败: {}", e)))?;

            let mut f = std::fs::File::open(path)
                .map_err(|e| AppError::Io(format!("读取文件失败: {}", e)))?;

            let mut buf = Vec::new();
            f.read_to_end(&mut buf)
                .map_err(|e| AppError::Io(format!("读取文件失败: {}", e)))?;

            writer.write_all(&buf)
                .map_err(|e| AppError::ProcessingFailed(format!("写入文件到 ZIP 失败: {}", e)))?;
        } else if path.is_dir() {
            writer.add_directory::<&str, ()>(name, zip::write::FileOptions::<()>::default())
                .map_err(|e| AppError::ProcessingFailed(format!("添加目录到 ZIP 失败: {}", e)))?;
        }
    }

    writer.finish()
        .map_err(|e| AppError::ProcessingFailed(format!("完成 ZIP 写入失败: {}", e)))?;

    Ok(format!("压缩完成: {}", output_path))
}

// ============================================================
// 命令:批量解压
// ============================================================

/// 批量解压压缩包。
///
/// - `files`: 压缩包文件路径列表(支持 zip, tar.gz, 7z)
/// - `output_dir`: 输出目录(每个文件解压到 output_dir/{文件名}/)
///
/// 返回每个文件的解压结果路径。
#[tauri::command]
pub async fn batch_extract(
    app: AppHandle,
    files: Vec<String>,
    output_dir: String,
) -> Result<Vec<TaskResult>, String> {
    if files.is_empty() {
        return Err("文件列表不能为空".into());
    }

    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("创建输出目录失败: {}", e))?;

    // 构建任务
    let tasks: Vec<Task> = files.iter().map(|f| {
        let path = Path::new(f);
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("extracted");

        // 处理 .tar.gz 双重扩展名
        let output_name = if f.ends_with(".tar.gz") || f.ends_with(".tgz") {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("extracted")
                .to_string()
        } else {
            stem.to_string()
        };

        let output_path = PathBuf::from(&output_dir).join(&output_name);

        Task {
            id: uuid::Uuid::new_v4().to_string(),
            input_path: f.clone(),
            output_path: output_path.to_string_lossy().to_string(),
            status: TaskStatus::Pending,
            params: serde_json::Value::Null,
        }
    }).collect();

    // 用 BatchRunner 并行执行
    let result = tokio::task::spawn_blocking(move || {
        let runner = BatchRunner::new(app);

        runner.run(tasks, |task, runner| {
            if runner.is_cancelled() {
                return Err(AppError::TaskCancelled);
            }

            let ext = get_ext(&task.input_path);
            let output_dir = Path::new(&task.output_path);

            std::fs::create_dir_all(output_dir)
                .map_err(|e| AppError::Io(format!("创建输出目录失败: {}", e)))?;

            match ext.as_str() {
                "zip" => extract_zip(&task.input_path, output_dir),
                "gz" | "tgz" => {
                    // 检查是否为 .tar.gz
                    if task.input_path.ends_with(".tar.gz") || ext == "tgz" {
                        extract_tar_gz(&task.input_path, output_dir)
                    } else {
                        Err(AppError::UnsupportedFormat(
                            format!("不支持的格式: .{}, 仅支持 .tar.gz", ext)
                        ))
                    }
                }
                "tar" => {
                    // 普通 tar 文件(无压缩)
                    let file = std::fs::File::open(&task.input_path)
                        .map_err(|e| AppError::Io(format!("打开文件失败: {}", e)))?;
                    let mut archive = tar::Archive::new(file);
                    archive.unpack(output_dir)
                        .map_err(|e| AppError::ProcessingFailed(
                            format!("解压 tar 失败: {}", e)
                        ))?;
                    Ok(format!("解压完成: {}", task.input_path))
                }
                "7z" => extract_7z(&task.input_path, output_dir),
                _ => Err(AppError::UnsupportedFormat(
                    format!("不支持该格式, 支持的格式: zip, tar, tar.gz, 7z")
                )),
            }
        })
    }).await
        .map_err(|e| format!("任务执行失败: {}", e))?
        .map_err(|e| e.to_string())?;

    Ok(result)
}

// ============================================================
// 命令:批量压缩
// ============================================================

/// 批量打包文件/文件夹为压缩包。
///
/// - `files`: 要打包的文件/文件夹路径列表
/// - `format`: 压缩格式 "zip" (目前仅支持 zip)
/// - `password`: 可选,加密密码
/// - `output_dir`: 输出目录
///
/// 返回生成的压缩包路径列表。
#[tauri::command]
pub async fn batch_compress(
    app: AppHandle,
    files: Vec<String>,
    format: String,
    password: Option<String>,
    output_dir: String,
) -> Result<Vec<TaskResult>, String> {
    if files.is_empty() {
        return Err("文件列表不能为空".into());
    }

    let fmt = format.to_lowercase();
    if fmt != "zip" {
        return Err(format!("暂不支持格式: {}, 目前仅支持 zip", format));
    }

    std::fs::create_dir_all(&output_dir)
        .map_err(|e| format!("创建输出目录失败: {}", e))?;

    // 构建任务:每 20 个文件打包为一个压缩包
    // 如果文件较少,全部打包为一个
    const BATCH_SIZE: usize = 20;

    let file_batches: Vec<Vec<String>> = if files.len() <= BATCH_SIZE {
        vec![files.clone()]
    } else {
        files.chunks(BATCH_SIZE).map(|chunk| chunk.to_vec()).collect()
    };

    let tasks: Vec<Task> = file_batches.iter().enumerate().map(|(i, batch)| {
        let batch_suffix = if file_batches.len() > 1 {
            format!("_{}", i + 1)
        } else {
            String::new()
        };
        let output_name = format!("archive{}.zip", batch_suffix);
        let output_path = PathBuf::from(&output_dir).join(&output_name);

        Task {
            id: uuid::Uuid::new_v4().to_string(),
            input_path: batch.join("|"), // 用 | 分隔多个输入文件
            output_path: output_path.to_string_lossy().to_string(),
            status: TaskStatus::Pending,
            params: serde_json::json!({
                "format": fmt,
                "password": password,
            }),
        }
    }).collect();

    // 用 BatchRunner 并行执行
    let result = tokio::task::spawn_blocking(move || {
        let runner = BatchRunner::new(app);

        runner.run(tasks, |task, runner| {
            if runner.is_cancelled() {
                return Err(AppError::TaskCancelled);
            }

            // 解析文件列表(用 | 分隔)
            let file_list: Vec<String> = task.input_path
                .split('|')
                .map(|s| s.to_string())
                .collect();

            // 密码从 params 读取
            let pwd = task.params.get("password")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty());

            create_zip(&file_list, &task.output_path, pwd)
        })
    }).await
        .map_err(|e| format!("任务执行失败: {}", e))?
        .map_err(|e| e.to_string())?;

    Ok(result)
}
