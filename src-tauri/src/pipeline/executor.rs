// ★ 流水线执行引擎(A-6 核心,A-8 配套)
//
// 职责:接收一个 Pipeline + 输入文件 → 按拓扑序逐节点执行 → 回传进度。
// 与 preview.rs 的区别:preview 只预测不改文件,executor 真正调用各命令。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter};

use crate::common::error::{AppError, AppResult};
use crate::common::types::Progress;
use crate::pipeline::model::{Pipeline, PipelineNode, PipelineProgress, PipelineStatus};
use crate::pipeline::preview::dry_run; // 复用拓扑排序
use crate::pipeline::registry;

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
                let img = image::open(file)
                    .map_err(|e| AppError::ProcessingFailed(format!("打开图片失败: {}", e)))?;
                let img = if max_w.is_some() || max_h.is_some() {
                    img.resize(max_w.unwrap_or(u32::MAX), max_h.unwrap_or(u32::MAX),
                               image::imageops::FilterType::Lanczos3)
                } else { img };

                let stem = std::path::Path::new(file).file_stem()
                    .and_then(|s| s.to_str()).unwrap_or("output");
                let ext = format;
                let out_path = format!("{}/{}.{}", out_dir, stem, ext);
                img.save(&out_path)
                    .map_err(|e| AppError::ProcessingFailed(format!("保存失败: {}", e)))?;
                outputs.push(out_path);
            }
            Ok(outputs)
        });

        // ---- 重命名 ----
        registry.register("rename", |files, params, out_dir| {
            let prefix = params.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
            let start = params.get("startNum").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let suffix = params.get("suffix").and_then(|v| v.as_str()).unwrap_or("");

            let mut outputs = Vec::new();
            for (i, file) in files.iter().enumerate() {
                let ext = std::path::Path::new(file).extension()
                    .and_then(|e| e.to_str()).unwrap_or("");
                let new_name = if ext.is_empty() {
                    format!("{}/{}{}{}", out_dir, prefix, start + i, suffix)
                } else {
                    format!("{}/{}{}{}.{}", out_dir, prefix, start + i, suffix, ext)
                };
                std::fs::copy(file, &new_name)
                    .map_err(|e| AppError::Io(format!("重命名失败 {}: {}", file, e)))?;
                outputs.push(new_name);
            }
            Ok(outputs)
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

        // ---- PDF合并(委托给 pdf_merge 命令) ----
        registry.register("pdf_merge", |files, params, out_dir| {
            let name = params.get("outputName").and_then(|v| v.as_str()).unwrap_or("merged");
            let out_path = format!("{}/{}.pdf", out_dir, name);

            let mut doc = lopdf::Document::new();
            for file in &files {
                let src = lopdf::Document::load(file)
                    .map_err(|e| AppError::ProcessingFailed(format!("PDF加载失败: {}", e)))?;
                for (_, page) in src.get_pages() {
                    doc.push_page(page.clone());
                }
            }
            doc.save(&out_path)
                .map_err(|e| AppError::ProcessingFailed(format!("PDF保存失败: {}", e)))?;
            Ok(vec![out_path])
        });

        // ---- 其余节点:默认复制(占位,后续补) ----
        let default_nodes = vec![
            "pdf_split", "pdf_compress", "dedup", "video_cut",
            "video_transcode", "video_to_gif", "extract_audio",
            "audio_convert", "office_convert", "archive_extract",
        ];
        for nt in default_nodes {
            registry.register(nt, |files, _params, out_dir| {
                let mut outputs = Vec::new();
                for file in &files {
                    let name = std::path::Path::new(file).file_name()
                        .and_then(|n| n.to_str()).unwrap_or("output");
                    let out = format!("{}/{}", out_dir, name);
                    std::fs::copy(file, &out)
                        .map_err(|e| AppError::Io(e.to_string()))?;
                    outputs.push(out);
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

    // 2. 建立索引
    let nodes: HashMap<&str, &PipelineNode> = pipeline.nodes.iter()
        .map(|n| (n.id.as_str(), n)).collect();

    // 3. 逐节点执行
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
