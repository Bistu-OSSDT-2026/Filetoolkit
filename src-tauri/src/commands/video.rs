// 视频/音频处理命令
//
// 底层统一调用 ffmpeg 子进程。
// 需要系统安装了 ffmpeg 并可在 PATH 中找到。

use std::process::Command;
use tauri::AppHandle;

/// 检查 ffmpeg 是否可用
#[tauri::command]
pub fn check_ffmpeg() -> Result<String, String> {
    match Command::new("ffmpeg").arg("-version").output() {
        Ok(output) if output.status.success() => {
            let version_line = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("ffmpeg")
                .to_string();
            Ok(version_line)
        }
        Ok(_) => Err("ffmpeg 未正确安装".into()),
        Err(_) => Err("ffmpeg 未安装。请从 https://ffmpeg.org/download.html 下载并添加到系统 PATH 中。".into()),
    }
}

/// 运行 ffmpeg 命令,返回 stdout 或错误信息
fn run_ffmpeg(args: &[&str]) -> Result<String, String> {
    let output = Command::new("ffmpeg")
        .args(args)
        .arg("-y") // 自动覆盖已有输出文件
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .output()
        .map_err(|e| format!("ffmpeg 执行失败(是否已安装?): {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // 取最后一行错误信息
        let msg = stderr.lines().last().unwrap_or(&stderr);
        Err(format!("ffmpeg 错误: {}", msg))
    }
}

/// 检测可用的 GPU 硬件编码器
#[tauri::command]
pub fn detect_gpu_encoders() -> Result<Vec<serde_json::Value>, String> {
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
        .map_err(|e| format!("ffmpeg 执行失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut encoders = Vec::new();

    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if lower.contains("nvenc") || lower.contains("h264_nvenc") || lower.contains("hevc_nvenc") {
            if !encoders.iter().any(|e: &serde_json::Value| e["name"] == "NVENC") {
                encoders.push(serde_json::json!({"name": "NVENC", "codec": "h264/h265"}));
            }
        } else if lower.contains("qsv") || lower.contains("h264_qsv") {
            if !encoders.iter().any(|e: &serde_json::Value| e["name"] == "QSV") {
                encoders.push(serde_json::json!({"name": "QSV", "codec": "h264/h265"}));
            }
        } else if lower.contains("amf") || lower.contains("h264_amf") {
            if !encoders.iter().any(|e: &serde_json::Value| e["name"] == "AMF") {
                encoders.push(serde_json::json!({"name": "AMF", "codec": "h264/h265"}));
            }
        }
    }

    Ok(encoders)
}

// ============================================================
// 视频剪切
// ============================================================

/// 剪切视频片段。
///
/// - `input`: 输入文件路径
/// - `start`: 开始时间 (HH:MM:SS 或秒数)
/// - `end`: 结束时间
/// - `output`: 输出路径
/// - `mode`: "fast"(不重新编码) 或 "accurate"(重新编码)
#[tauri::command]
pub fn cut_video(
    _app: AppHandle,
    input: String,
    start: String,
    end: String,
    output: String,
    mode: Option<String>,
) -> Result<String, String> {
    let use_fast = mode.as_deref() != Some("accurate");

    if use_fast {
        // 快速模式: stream copy (不重新编码,速度快)
        run_ffmpeg(&[
            "-ss", &start,
            "-i", &input,
            "-to", &end,
            "-c", "copy",
            "-avoid_negative_ts", "make_zero",
            &output,
        ])?;
    } else {
        // 精确模式: 重新编码
        run_ffmpeg(&[
            "-ss", &start,
            "-i", &input,
            "-to", &end,
            "-c:v", "libx264",
            "-c:a", "aac",
            &output,
        ])?;
    }

    Ok(format!("视频剪切完成: {}", output))
}

// ============================================================
// 视频转码
// ============================================================

/// 视频格式转换/编码。
///
/// - `input`: 输入文件路径
/// - `output_format`: 目标编码格式 (h264, h265, vp9)
/// - `crf`: 质量参数 (0-51, 越小质量越高, 23 默认)
/// - `output`: 输出路径
#[tauri::command]
pub fn transcode_video(
    _app: AppHandle,
    input: String,
    output_format: Option<String>,
    crf: Option<u8>,
    output: String,
) -> Result<String, String> {
    let fmt = output_format.as_deref().unwrap_or("h264");
    let quality = crf.unwrap_or(23).to_string();

    match fmt {
        "h264" => {
            run_ffmpeg(&[
                "-i", &input,
                "-c:v", "libx264",
                "-crf", &quality,
                "-preset", "medium",
                "-c:a", "aac",
                &output,
            ])?;
        }
        "h265" => {
            run_ffmpeg(&[
                "-i", &input,
                "-c:v", "libx265",
                "-crf", &quality,
                "-preset", "medium",
                "-c:a", "aac",
                &output,
            ])?;
        }
        "vp9" => {
            run_ffmpeg(&[
                "-i", &input,
                "-c:v", "libvpx-vp9",
                "-crf", &quality,
                "-b:v", "0",
                "-c:a", "libopus",
                &output,
            ])?;
        }
        _ => return Err(format!("不支持的编码格式: {}", fmt)),
    }

    Ok(format!("转码完成: {}", output))
}

// ============================================================
// 视频转 GIF
// ============================================================

/// 将视频片段转为 GIF。
///
/// - `input`: 输入文件路径
/// - `start`: 开始时间
/// - `duration`: 持续秒数
/// - `fps`: 帧率
/// - `width`: 输出宽度(等比缩放)
/// - `output`: 输出路径 (.gif)
#[tauri::command]
pub fn video_to_gif(
    _app: AppHandle,
    input: String,
    start: String,
    duration: f64,
    fps: u32,
    width: u32,
    output: String,
) -> Result<String, String> {
    let fps_str = fps.to_string();
    let width_str = width.to_string();
    let dur_str = duration.to_string();

    // ffmpeg 生成调色板 + GIF (标准两步法)
    let temp_palette = format!("{}_palette.png", output.trim_end_matches(".gif"));

    // 步骤1: 生成调色板
    run_ffmpeg(&[
        "-ss", &start,
        "-t", &dur_str,
        "-i", &input,
        "-vf", &format!("fps={},scale={}:-1:flags=lanczos,palettegen", fps_str, width_str),
        &temp_palette,
    ])?;

    // 步骤2: 用调色板生成 GIF
    let result = run_ffmpeg(&[
        "-ss", &start,
        "-t", &dur_str,
        "-i", &input,
        "-i", &temp_palette,
        "-lavfi", &format!("fps={},scale={}:-1:flags=lanczos[x];[x][1:v]paletteuse", fps_str, width_str),
        &output,
    ]);

    // 清理临时调色板文件
    let _ = std::fs::remove_file(&temp_palette);

    result?;
    Ok(format!("GIF 生成完成: {}", output))
}

// ============================================================
// 音频提取
// ============================================================

/// 从视频/音频文件中提取音频。
///
/// - `input`: 输入文件路径
/// - `format`: 输出格式 (mp3, aac, flac, ogg, wav)
/// - `bitrate`: 比特率 (如 "192k")
/// - `output`: 输出路径
#[tauri::command]
pub fn extract_audio(
    _app: AppHandle,
    input: String,
    format: String,
    bitrate: String,
    output: String,
) -> Result<String, String> {
    match format.as_str() {
        "mp3" => {
            run_ffmpeg(&[
                "-i", &input,
                "-vn", // 不要视频流
                "-c:a", "libmp3lame",
                "-b:a", &bitrate,
                &output,
            ])?;
        }
        "aac" => {
            run_ffmpeg(&[
                "-i", &input,
                "-vn",
                "-c:a", "aac",
                "-b:a", &bitrate,
                &output,
            ])?;
        }
        "flac" => {
            run_ffmpeg(&[
                "-i", &input,
                "-vn",
                "-c:a", "flac",
                &output,
            ])?;
        }
        "ogg" => {
            run_ffmpeg(&[
                "-i", &input,
                "-vn",
                "-c:a", "libvorbis",
                "-b:a", &bitrate,
                &output,
            ])?;
        }
        "wav" => {
            run_ffmpeg(&[
                "-i", &input,
                "-vn",
                "-c:a", "pcm_s16le",
                &output,
            ])?;
        }
        _ => return Err(format!("不支持的音频格式: {}", format)),
    }

    Ok(format!("音频提取完成: {}", output))
}
