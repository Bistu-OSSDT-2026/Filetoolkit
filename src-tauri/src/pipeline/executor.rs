// ★ 流水线执行引擎(A-6 核心,A-8 配套)
//
// 职责:接收一个 Pipeline + 输入文件 → 按拓扑序逐节点执行 → 回传进度。
// 与 preview.rs 的区别:preview 只预测不改文件,executor 真正调用各命令。

use std::collections::HashMap;
use std::io::Cursor;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter};

use crate::common::error::{AppError, AppResult};
use crate::common::types::Progress;
use crate::pipeline::model::{Pipeline, PipelineNode, PipelineProgress, PipelineStatus};
use crate::pipeline::preview; // 复用拓扑排序
use crate::pipeline::registry;
use crate::commands::rename::do_rename;
use crate::commands::pdf::do_split_pdf;

// ============================================================
// 执行结果
// ============================================================

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    /// 是否全部成功
    pub success: bool,
    /// 每一步的结果
    pub steps: Vec<StepResult>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    pub node_id: String,
    pub node_label: String,
    pub success: bool,
    pub output_files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================
// 节点处理器类型
// ============================================================

/// 节点处理函数签名:
/// 输入 files → 参数 → 输出目录 → 返回输出文件列表
pub type NodeHandler = Arc<
    dyn Fn(Vec<String>, serde_json::Value, String) -> AppResult<Vec<String>> + Send + Sync
>;

/// 处理器注册表:node_type → handler
pub struct HandlerRegistry {
    handlers: HashMap<String, NodeHandler>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    /// 注册一个节点处理器。
    pub fn register<F>(&mut self, node_type: &str, handler: F)
    where
        F: Fn(Vec<String>, serde_json::Value, String) -> AppResult<Vec<String>> + Send + Sync + 'static,
    {
        self.handlers.insert(node_type.to_string(), Arc::new(handler));
    }

    /// 获取处理器。
    pub fn get(&self, node_type: &str) -> Option<&NodeHandler> {
        self.handlers.get(node_type)
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // ---- 图片压缩 ----
        registry.register("image_compress", |files, params, out_dir| {
            let quality = params.get("quality").and_then(|v| v.as_u64()).unwrap_or(80) as u8;
            let format = params.get("format").and_then(|v| v.as_str()).unwrap_or("jpg");
            let max_w = params.get("maxWidth").and_then(|v| v.as_u64()).map(|v| v as u32);
            let max_h = params.get("maxHeight").and_then(|v| v.as_u64()).map(|v| v as u32);

            let mut outputs = Vec::new();
            for file in &files {
                let data = std::fs::read(file)
                    .map_err(|e| AppError::Io(format!("读取失败 {}: {}", file, e)))?;
                let mut img = image::load_from_memory(&data)
                    .map_err(|e| AppError::ProcessingFailed(format!("解码失败 {}: {}", file, e)))?;

                // 等比缩放（仅当超出限制时）
                if let (Some(mw), Some(mh)) = (max_w, max_h) {
                    if img.width() > mw || img.height() > mh {
                        img = img.resize(mw, mh, image::imageops::FilterType::Lanczos3);
                    }
                } else if let Some(mw) = max_w {
                    if img.width() > mw {
                        let ratio = mw as f64 / img.width() as f64;
                        let h = (img.height() as f64 * ratio) as u32;
                        img = img.resize(mw, h, image::imageops::FilterType::Lanczos3);
                    }
                } else if let Some(mh) = max_h {
                    if img.height() > mh {
                        let ratio = mh as f64 / img.height() as f64;
                        let w = (img.width() as f64 * ratio) as u32;
                        img = img.resize(w, mh, image::imageops::FilterType::Lanczos3);
                    }
                }

                let stem = std::path::Path::new(file).file_stem()
                    .and_then(|s| s.to_str()).unwrap_or("output");
                let out_path = format!("{}/{}.{}", out_dir, stem, format);

                // 使用指定格式和质量编码
                match format {
                    "jpg" | "jpeg" => {
                        let mut buf = Vec::new();
                        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                            std::io::Cursor::new(&mut buf), quality,
                        );
                        img.write_with_encoder(encoder)
                            .map_err(|e| AppError::ProcessingFailed(format!("JPEG编码失败: {}", e)))?;
                        std::fs::write(&out_path, &buf)
                            .map_err(|e| AppError::Io(format!("写入失败 {}: {}", out_path, e)))?;
                    }
                    "webp" => {
                        let mut buf = Vec::new();
                        let encoder = image::codecs::webp::WebPEncoder::new_lossless(
                            std::io::Cursor::new(&mut buf),
                        );
                        img.write_with_encoder(encoder)
                            .map_err(|e| AppError::ProcessingFailed(format!("WebP编码失败: {}", e)))?;
                        std::fs::write(&out_path, &buf)
                            .map_err(|e| AppError::Io(format!("写入失败 {}: {}", out_path, e)))?;
                    }
                    _ => {
                        img.save(&out_path)
                            .map_err(|e| AppError::ProcessingFailed(format!("保存失败 {}: {}", out_path, e)))?;
                    }
                }
                outputs.push(out_path);
            }
            Ok(outputs)
        });

        // ---- 重命名（调用 commands/rename.rs 的真实逻辑）----
        registry.register("rename", |files, params, out_dir| {
            let prefix = params.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
            let start = params.get("startNum").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
            let suffix = params.get("suffix").and_then(|v| v.as_str()).unwrap_or("");
            do_rename(&files, prefix, start, suffix, Some(&out_dir))
                .map_err(|e| AppError::ProcessingFailed(e.join("; ")))
        });

        // ---- 打包 ----
        registry.register("archive_compress", |files, params, out_dir| {
            let password = params.get("password").and_then(|v| v.as_str())
                .filter(|p| !p.is_empty());
            let out_path = format!("{}/archive.zip", out_dir);

            let file = std::fs::File::create(&out_path)
                .map_err(|e| AppError::Io(e.to_string()))?;
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();

            for path in &files {
                let name = std::path::Path::new(path).file_name()
                    .and_then(|n| n.to_str()).unwrap_or("unknown");
                zip.start_file(name, options)
                    .map_err(|e| AppError::ProcessingFailed(format!("zip错误: {}", e)))?;
                let data = std::fs::read(path)
                    .map_err(|e| AppError::Io(e.to_string()))?;
                std::io::Write::write_all(&mut zip, &data)
                    .map_err(|e| AppError::Io(e.to_string()))?;
            }
            let _ = zip.finish();
            Ok(vec![out_path])
        });

        // ---- PDF合并: lopdf 合并多页 ----
        registry.register("pdf_merge", |files, params, out_dir| {
            let name = params.get("outputName").and_then(|v| v.as_str()).unwrap_or("merged");
            let out_path = format!("{}/{}.pdf", out_dir, name);

            let mut merged = lopdf::Document::new();
            // 收集所有源文档的页面对象，重新编号后插入
            let mut max_id: u32 = 1;
            let mut page_ids: Vec<lopdf::ObjectId> = Vec::new();

            for file in &files {
                let mut doc = lopdf::Document::load(file)
                    .map_err(|e| AppError::ProcessingFailed(format!("加载PDF失败 {}: {}", file, e)))?;
                doc.renumber_objects_with(max_id);
                max_id = doc.max_id + 1;

                let pages = doc.get_pages();
                for (_, pid) in &pages {
                    page_ids.push(*pid);
                    if let Ok(obj) = doc.get_object(*pid) {
                        merged.objects.insert(*pid, obj.clone());
                    }
                }
                // 复制非页面对象
                for (oid, obj) in &doc.objects {
                    if !merged.objects.contains_key(oid) {
                        merged.objects.insert(*oid, obj.clone());
                    }
                }
            }

            // 设置 Pages 根
            let pages_id = merged.new_object_id();
            let kids: Vec<lopdf::Object> = page_ids.iter()
                .map(|&id| lopdf::Object::Reference(id))
                .collect();
            let pages_dict = lopdf::Dictionary::from_iter(vec![
                ("Type", lopdf::Object::Name(b"Pages".to_vec())),
                ("Count", lopdf::Object::Integer(page_ids.len() as i64)),
                ("Kids", lopdf::Object::Array(kids)),
            ]);
            merged.objects.insert(pages_id, lopdf::Object::Dictionary(pages_dict));

            // 设置 Catalog
            let catalog_id = merged.new_object_id();
            let catalog_dict = lopdf::Dictionary::from_iter(vec![
                ("Type", lopdf::Object::Name(b"Catalog".to_vec())),
                ("Pages", lopdf::Object::Reference(pages_id)),
            ]);
            merged.objects.insert(catalog_id, lopdf::Object::Dictionary(catalog_dict));
            merged.trailer.set("Root", catalog_id);
            merged.max_id = merged.objects.len() as u32;

            merged.save(&out_path)
                .map_err(|e| AppError::ProcessingFailed(format!("保存PDF失败: {}", e)))?;
            Ok(vec![out_path])
        });

        // ---- PDF压缩: lopdf compress ----
        registry.register("pdf_compress", |files, _params, out_dir| {
            let mut outputs = Vec::new();
            for file in &files {
                let mut doc = lopdf::Document::load(file)
                    .map_err(|e| AppError::ProcessingFailed(format!("加载PDF失败 {}: {}", file, e)))?;
                doc.compress();
                let name = std::path::Path::new(file).file_name()
                    .and_then(|n| n.to_str()).unwrap_or("compressed.pdf");
                let out_path = format!("{}/{}", out_dir, name);
                doc.save(&out_path)
                    .map_err(|e| AppError::ProcessingFailed(format!("保存PDF失败: {}", e)))?;
                outputs.push(out_path);
            }
            Ok(outputs)
        });

        // ---- 辅助: ffmpeg 子进程 ----
        fn run_ff(args: &[&str]) -> Result<(), String> {
            let out = Command::new("ffmpeg").args(args).arg("-y").arg("-hide_banner")
                .arg("-loglevel").arg("error").output()
                .map_err(|e| format!("ffmpeg 未安装或执行失败: {}", e))?;
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let msg = format!("ffmpeg({}): {}", args.last().unwrap_or(&"?"), stderr.lines().last().unwrap_or(&stderr));
                return Err(msg);
            }
            Ok(())
        }

        // ---- 视频剪切 ----
        registry.register("video_cut", |files, params, out_dir| {
            let start = params.get("start").and_then(|v| v.as_str()).unwrap_or("00:00:00");
            let end = params.get("end").and_then(|v| v.as_str()).unwrap_or("00:00:10");
            let mode = params.get("mode").and_then(|v| v.as_str()).unwrap_or("fast");
            let mut outputs = Vec::new();
            for file in &files {
                let ext = std::path::Path::new(file).extension().and_then(|e| e.to_str()).unwrap_or("mp4");
                let out = std::path::PathBuf::from(&out_dir).join(format!("cut_output.{}", ext));
                let out_str = out.to_string_lossy().to_string();
                if mode == "fast" {
                    run_ff(&["-ss", start, "-i", file, "-to", end, "-c", "copy", "-avoid_negative_ts", "make_zero", &out_str])
                        .map_err(|e| AppError::ProcessingFailed(e))?;
                } else {
                    run_ff(&["-ss", start, "-i", file, "-to", end, "-c:v", "libx264", "-c:a", "aac", &out_str])
                        .map_err(|e| AppError::ProcessingFailed(e))?;
                }
                outputs.push(out_str);
            }
            Ok(outputs)
        });

        // ---- 视频转码 ----
        registry.register("video_transcode", |files, params, out_dir| {
            let codec = params.get("videoCodec").and_then(|v| v.as_str()).unwrap_or("h264");
            let crf = params.get("crf").and_then(|v| v.as_u64()).unwrap_or(23).to_string();
            let encoder = params.get("encoder").and_then(|v| v.as_str()).unwrap_or("");
            let mut outputs = Vec::new();
            for file in &files {
                let stem = std::path::Path::new(file).file_stem().and_then(|n| n.to_str()).unwrap_or("out");
                let out = std::path::PathBuf::from(&out_dir).join(format!("transcode_output.mp4"));
                let out_str = out.to_string_lossy().to_string();
                let vcodec = if !encoder.is_empty() { encoder } else {
                    match codec { "h265" => "libx265", "vp9" => "libvpx-vp9", _ => "libx264" }
                };
                run_ff(&["-i", file, "-c:v", vcodec, "-crf", &crf, "-preset", "medium", "-c:a", "aac", &out_str])
                    .map_err(|e| AppError::ProcessingFailed(e))?;
                outputs.push(out_str);
            }
            Ok(outputs)
        });

        // ---- 视频转 GIF ----
        registry.register("video_to_gif", |files, params, out_dir| {
            let start = params.get("start").and_then(|v| v.as_str()).unwrap_or("00:00:00");
            let dur = params.get("duration").and_then(|v| v.as_u64()).unwrap_or(5).to_string();
            let fps = params.get("fps").and_then(|v| v.as_u64()).unwrap_or(10).to_string();
            let w = params.get("width").and_then(|v| v.as_u64()).unwrap_or(480).to_string();
            let mut outputs = Vec::new();
            for file in &files {
                let out = std::path::PathBuf::from(&out_dir).join("output.gif");
                let out_str = out.to_string_lossy().to_string();
                let pal = std::path::PathBuf::from(&out_dir).join("palette.png");
                let pal_str = pal.to_string_lossy().to_string();
                run_ff(&["-ss", &start, "-t", &dur, "-i", file,
                         "-vf", &format!("fps={},scale={}:-1:flags=lanczos,palettegen", fps, w), &pal_str])
                    .map_err(|e| AppError::ProcessingFailed(e))?;
                run_ff(&["-ss", &start, "-t", &dur, "-i", file, "-i", &pal_str,
                         "-lavfi", &format!("fps={},scale={}:-1:flags=lanczos[x];[x][1:v]paletteuse", fps, w), &out_str])
                    .map_err(|e| AppError::ProcessingFailed(e))?;
                let _ = std::fs::remove_file(&pal);
                outputs.push(out_str);
            }
            Ok(outputs)
        });

        // ---- 音频提取/转换 ----
        registry.register("extract_audio", |files, params, out_dir| {
            let fmt = params.get("format").and_then(|v| v.as_str()).unwrap_or("mp3");
            let br = params.get("bitrate").and_then(|v| v.as_str()).unwrap_or("192k");
            let mut outputs = Vec::new();
            for file in &files {
                let out = std::path::PathBuf::from(&out_dir).join(format!("audio_output.{}", fmt));
                let out_str = out.to_string_lossy().to_string();
                let (codec, _) = match fmt { "aac" => ("aac", "aac"), "flac" => ("flac", "flac"),
                    "wav" => ("pcm_s16le", "wav"), "ogg" => ("libvorbis", "ogg"), _ => ("libmp3lame", "mp3") };
                run_ff(&["-i", file, "-vn", "-c:a", codec, "-b:a", br, &out_str])
                    .map_err(|e| AppError::ProcessingFailed(e))?;
                outputs.push(out_str);
            }
            Ok(outputs)
        });

        registry.register("audio_convert", |files, params, out_dir| {
            // 与 extract_audio 相同逻辑
            let fmt = params.get("format").and_then(|v| v.as_str()).unwrap_or("mp3");
            let br = params.get("bitrate").and_then(|v| v.as_str()).unwrap_or("192k");
            let mut outputs = Vec::new();
            for file in &files {
                let out = std::path::PathBuf::from(&out_dir).join(format!("audio_output.{}", fmt));
                let out_str = out.to_string_lossy().to_string();
                let (codec, _) = match fmt { "aac" => ("aac", "aac"), "flac" => ("flac", "flac"),
                    "wav" => ("pcm_s16le", "wav"), "ogg" => ("libvorbis", "ogg"), _ => ("libmp3lame", "mp3") };
                run_ff(&["-i", file, "-vn", "-c:a", codec, "-b:a", br, &out_str])
                    .map_err(|e| AppError::ProcessingFailed(e))?;
                outputs.push(out_str);
            }
            Ok(outputs)
        });

        // ---- PDF拆分: 调用 commands/pdf.rs 的真实逻辑 ----
        registry.register("pdf_split", |files, params, out_dir| {
            let ranges_str: String = params.get("ranges").and_then(|v| v.as_str()).unwrap_or("1").to_string();
            let ranges: Vec<String> = ranges_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            let mut outputs = Vec::new();
            for file in &files {
                let out_paths = do_split_pdf(file, &ranges, &out_dir)
                    .map_err(|e| AppError::ProcessingFailed(e))?;
                outputs.extend(out_paths);
            }
            Ok(outputs)
        });

        // ---- 查重: blake3 哈希去重，保留第一个 ----
        registry.register("dedup", |files, _params, out_dir| {
            use std::collections::HashMap;
            let mut hash_map: HashMap<String, String> = HashMap::new();
            for file in &files {
                let data = std::fs::read(file)
                    .map_err(|e| AppError::Io(format!("读取失败 {}: {}", file, e)))?;
                let hash = blake3::hash(&data).to_hex().to_string();
                hash_map.entry(hash).or_insert_with(|| file.clone());
            }
            let mut outputs = Vec::new();
            for (_, kept) in &hash_map {
                let name = std::path::Path::new(kept).file_name().and_then(|n| n.to_str()).unwrap_or("file");
                let dest = std::path::PathBuf::from(&out_dir).join(name);
                if std::path::Path::new(kept) != dest {
                    std::fs::copy(kept, &dest)
                        .map_err(|e| AppError::Io(format!("复制失败 {} → {}: {}", kept, dest.display(), e)))?;
                }
                outputs.push(dest.display().to_string());
            }
            Ok(outputs)
        });

        // ---- 其余节点:默认复制 ----
        let default_nodes = vec![
            "office_convert", "archive_extract",
        ];
        for nt in default_nodes {
            registry.register(nt, |files, _params, out_dir| {
                let od = out_dir;  // 捕获所有权
                let mut outputs = Vec::new();
                for file in &files {
                    let name = std::path::Path::new(file)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "output".to_string());
                    let out = std::path::PathBuf::from(&od).join(&name);

                    // 源和目标相同 → 不复制
                    if std::path::Path::new(file) == out {
                        outputs.push(file.clone());
                        continue;
                    }

                    // 目标已存在 → 加后缀避免冲突/文件锁
                    let final_out = if out.exists() {
                        let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
                        let ext = out.extension().and_then(|e| e.to_str()).unwrap_or("");
                        let alt = if ext.is_empty() {
                            std::path::PathBuf::from(&od).join(format!("{}_step", stem))
                        } else {
                            std::path::PathBuf::from(&od).join(format!("{}_step.{}", stem, ext))
                        };
                        alt
                    } else {
                        out
                    };

                    std::fs::copy(file, &final_out)
                        .map_err(|e| AppError::Io(format!("复制失败 {} → {}: {}", file, final_out.display(), e)))?;
                    outputs.push(final_out.display().to_string());
                }
                Ok(outputs)
            });
        }

        registry
    }
}

// ============================================================
// 执行引擎
// ============================================================

/// 真正执行流水线。
///
/// # 参数
/// - `pipeline`:前端传来的 Pipeline JSON
/// - `input_files`:用户初始选择的文件
/// - `output_dir`:最终输出目录
/// - `cancel_flag`:取消标志
pub fn execute_pipeline(
    app: &AppHandle,
    pipeline: &Pipeline,
    input_files: &[String],
    output_dir: &str,
    cancel_flag: Arc<AtomicBool>,
) -> ExecutionResult {
    let registry = HandlerRegistry::default();
    let total_steps = pipeline.nodes.len() as u32;

    // 1. 拓扑排序
    let order = match crate::pipeline::preview::topological_sort(pipeline) {
        Ok(o) => o,
        Err(e) => {
            return ExecutionResult {
                success: false,
                steps: vec![StepResult {
                    node_id: "".into(), node_label: "流水线错误".into(),
                    success: false, output_files: vec![], error: Some(e),
                }],
            };
        }
    };

    // 2. 创建输出目录
    if let Err(e) = std::fs::create_dir_all(output_dir) {
        return ExecutionResult {
            success: false,
            steps: vec![StepResult {
                node_id: "".into(), node_label: "系统错误".into(),
                success: false, output_files: vec![],
                error: Some(format!("无法创建输出目录 {}: {}", output_dir, e)),
            }],
        };
    }

    // 3. 建立索引
    let nodes: HashMap<&str, &PipelineNode> = pipeline.nodes.iter()
        .map(|n| (n.id.as_str(), n)).collect();

    // 4. 逐节点执行
    let mut outputs: HashMap<String, Vec<String>> = HashMap::new();
    let mut steps = Vec::new();

    for (step_idx, node_id) in order.iter().enumerate() {
        // 取消检查
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = app.emit("pipeline-progress", PipelineProgress {
                pipeline_id: pipeline.name.clone(),
                current_step: step_idx as u32, total_steps,
                current_node_id: node_id.clone(),
                current_node_label: "".into(),
                status: PipelineStatus::Cancelled,
            });
            steps.push(StepResult {
                node_id: node_id.clone(), node_label: "已取消".into(),
                success: false, output_files: vec![],
                error: Some("流水线已被用户取消".into()),
            });
            return ExecutionResult { success: false, steps };
        }

        let node = match nodes.get(node_id.as_str()) {
            Some(n) => n,
            None => {
                steps.push(StepResult {
                    node_id: node_id.clone(), node_label: "未知节点".into(),
                    success: false, output_files: vec![],
                    error: Some(format!("节点 {} 不存在", node_id)),
                });
                continue;
            }
        };

        // 获取输入:来自上游或原始输入
        let input: Vec<String> = pipeline.edges.iter()
            .find(|e| e.target == *node_id)
            .and_then(|e| outputs.get(&e.source))
            .cloned()
            .unwrap_or_else(|| input_files.to_vec());

        // 提交进度
        let _ = app.emit("pipeline-progress", PipelineProgress {
            pipeline_id: pipeline.name.clone(),
            current_step: step_idx as u32, total_steps,
            current_node_id: node.id.clone(),
            current_node_label: node.label.clone(),
            status: PipelineStatus::Running,
        });

        // 执行节点
        let handler = registry.get(&node.node_type);
        match handler {
            Some(h) => match h(input.clone(), node.params.clone(), output_dir.to_string()) {
                Ok(files) => {
                    steps.push(StepResult {
                        node_id: node.id.clone(), node_label: node.label.clone(),
                        success: true, output_files: files.clone(), error: None,
                    });
                    outputs.insert(node.id.clone(), files);
                }
                Err(e) => {
                    let _ = app.emit("pipeline-progress", PipelineProgress {
                        pipeline_id: pipeline.name.clone(),
                        current_step: step_idx as u32, total_steps,
                        current_node_id: node.id.clone(),
                        current_node_label: node.label.clone(),
                        status: PipelineStatus::Failed(e.to_string()),
                    });
                    steps.push(StepResult {
                        node_id: node.id.clone(), node_label: node.label.clone(),
                        success: false, output_files: vec![],
                        error: Some(e.to_string()),
                    });
                    return ExecutionResult { success: false, steps };
                }
            },
            None => {
                steps.push(StepResult {
                    node_id: node.id.clone(), node_label: node.label.clone(),
                    success: false, output_files: vec![],
                    error: Some(format!("未注册的节点类型: {}", node.node_type)),
                });
            }
        }
    }

    // 完成
    let _ = app.emit("pipeline-progress", PipelineProgress {
        pipeline_id: pipeline.name.clone(),
        current_step: total_steps, total_steps,
        current_node_id: "".into(),
        current_node_label: "完成".into(),
        status: PipelineStatus::Completed,
    });

    ExecutionResult { success: true, steps }
}

// ============================================================
// Tauri 命令
// ============================================================

/// 执行流水线。
/// D 的编辑器在用户点[执行]按钮时调用。
#[tauri::command]
pub async fn run_pipeline(
    app: AppHandle,
    pipeline_json: String,
    input_files: Vec<String>,
    output_dir: String,
) -> Result<ExecutionResult, String> {
    let pipeline: Pipeline = serde_json::from_str(&pipeline_json)
        .map_err(|e| format!("JSON解析失败: {}", e))?;

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let app_clone = app.clone();
    let pipeline_clone = pipeline.clone();

    tokio::task::spawn_blocking(move || {
        execute_pipeline(&app_clone, &pipeline_clone, &input_files, &output_dir, cancel_flag)
    })
    .await
    .map_err(|e| format!("执行失败: {}", e))
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::model::{NodePosition, Pipeline, PipelineEdge, PipelineNode};

    fn make_rename_pipeline() -> Pipeline {
        Pipeline {
            name: "rename-test".into(),
            description: "".into(),
            nodes: vec![PipelineNode {
                id: "n1".into(),
                node_type: "rename".into(),
                label: "重命名".into(),
                position: NodePosition::default(),
                params: serde_json::json!({"prefix": "test_", "startNum": 1, "suffix": ""}),
            }],
            edges: vec![],
        }
    }

    #[test]
    fn test_handler_registry_has_all_types() {
        let registry = HandlerRegistry::default();
        let types = registry::get_all_node_types();
        for nt in &types {
            assert!(registry.get(&nt.id).is_some(), "缺少处理器: {}", nt.id);
        }
    }

    #[test]
    fn test_rename_handler() {
        let registry = HandlerRegistry::default();
        let handler = registry.get("rename").unwrap();

        let tmp = std::env::temp_dir().join("filetoolkit-test-rename");
        let _ = std::fs::create_dir_all(&tmp);

        let src = tmp.join("src.jpg");
        std::fs::write(&src, b"test").unwrap();

        let result = handler(
            vec![src.display().to_string()],
            serde_json::json!({"prefix": "img_", "startNum": 1, "suffix": ""}),
            tmp.display().to_string(),
        );
        assert!(result.is_ok());
        assert!(result.unwrap()[0].ends_with("img_1.jpg"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_archive_handler() {
        let registry = HandlerRegistry::default();
        let handler = registry.get("archive_compress").unwrap();

        let tmp = std::env::temp_dir().join("filetoolkit-test-archive");
        let _ = std::fs::create_dir_all(&tmp);

        let f1 = tmp.join("a.txt");
        std::fs::write(&f1, b"hello").unwrap();

        let result = handler(
            vec![f1.display().to_string()],
            serde_json::json!({}),
            tmp.display().to_string(),
        );
        assert!(result.is_ok());
        let zip_path = &result.unwrap()[0];
        assert!(std::path::Path::new(zip_path).exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
