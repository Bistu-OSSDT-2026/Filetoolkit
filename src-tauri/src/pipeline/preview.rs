// ★ Dry-run 预览引擎(A-8)
//
// 用户点"预览"按钮后,后端根据 Pipeline 描述模拟每个节点的输出。
// 不实际执行任何文件操作,仅预测文件变换链。
// D 的编辑器在用户点[预览]时调用 preview_pipeline 命令。

use std::collections::{HashMap, HashSet};

use crate::pipeline::model::*;

// ============================================================
// 输出结构
// ============================================================

/// 一次 dry-run 的完整结果。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    /// 流水线名称
    pub pipeline_name: String,
    /// 节点执行顺序(拓扑排序后)
    pub execution_order: Vec<String>,
    /// 每个节点的预览
    pub steps: Vec<StepPreview>,
    /// 是否有环(无效流水线)
    pub has_cycle: bool,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 流水线中一个步骤的预览。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepPreview {
    /// 节点 ID
    pub node_id: String,
    /// 节点类型 ID
    pub node_type: String,
    /// 节点名称
    pub node_label: String,
    /// 输入文件列表
    pub input_files: Vec<String>,
    /// 预测的输出文件列表
    pub output_files: Vec<String>,
    /// 输入来自哪个上游节点
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_from: Option<String>,
}

// ============================================================
// 核心逻辑
// ============================================================

/// 执行 dry-run:根据 Pipeline 和输入文件,预测每一步的输出。
pub fn dry_run(pipeline: &Pipeline, input_files: Vec<String>) -> PreviewResult {
    // ============================================================
    // 1. 拓扑排序(检测环)
    // ============================================================
    let order_result = topological_sort(pipeline);
    if let Err(cycle_error) = order_result {
        return PreviewResult {
            pipeline_name: pipeline.name.clone(),
            execution_order: vec![],
            steps: vec![],
            has_cycle: true,
            error: Some(cycle_error),
        };
    }
    let execution_order = order_result.unwrap();

    // ============================================================
    // 2. 建立节点索引
    // ============================================================
    let nodes: HashMap<&str, &PipelineNode> = pipeline.nodes.iter()
        .map(|n| (n.id.as_str(), n))
        .collect();

    // ============================================================
    // 3. 逐节点模拟
    // ============================================================
    let mut steps = Vec::new();
    // node_id → 它产出的文件列表
    let mut outputs: HashMap<String, Vec<String>> = HashMap::new();

    for node_id in &execution_order {
        let node = match nodes.get(node_id.as_str()) {
            Some(n) => n,
            None => continue,
        };

        // 找出输入来源:谁把文件传给这个节点
        let input_from = pipeline.edges.iter()
            .find(|e| e.target == *node_id)
            .map(|e| e.source.clone());

        // 获取输入文件:要么来自上游,要么就是原始输入文件
        let input_files: Vec<String> = if let Some(ref upstream) = input_from {
            outputs.get(upstream).cloned().unwrap_or_default()
        } else {
            input_files.clone()
        };

        if input_files.is_empty() {
            steps.push(StepPreview {
                node_id: node.id.clone(),
                node_type: node.node_type.clone(),
                node_label: node.label.clone(),
                input_files: vec![],
                output_files: vec![],
                input_from: input_from.clone(),
            });
            continue;
        }

        // 预测输出
        let predicted = predict_outputs(&node.node_type, &input_files, &node.params);

        steps.push(StepPreview {
            node_id: node.id.clone(),
            node_type: node.node_type.clone(),
            node_label: node.label.clone(),
            input_files: input_files.clone(),
            output_files: predicted.clone(),
            input_from,
        });

        outputs.insert(node.id.clone(), predicted);
    }

    PreviewResult {
        pipeline_name: pipeline.name.clone(),
        execution_order,
        steps,
        has_cycle: false,
        error: None,
    }
}

// ============================================================
// 拓扑排序(Kahn 算法)
// ============================================================

pub fn topological_sort(pipeline: &Pipeline) -> Result<Vec<String>, String> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

    for node in &pipeline.nodes {
        in_degree.entry(node.id.as_str()).or_insert(0);
        graph.entry(node.id.as_str()).or_default();
    }

    for edge in &pipeline.edges {
        graph.entry(edge.source.as_str()).or_default().push(edge.target.as_str());
        *in_degree.entry(edge.target.as_str()).or_insert(0) += 1;
    }

    let mut queue: Vec<&str> = in_degree.iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut result = Vec::new();
    while let Some(node) = queue.pop() {
        result.push(node.to_string());
        if let Some(neighbors) = graph.get(node) {
            for &neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor);
                    }
                }
            }
        }
    }

    if result.len() != pipeline.nodes.len() {
        Err("流水线中存在环路,请检查连线".into())
    } else {
        Ok(result)
    }
}

// ============================================================
// 输出预测(按节点类型)
// ============================================================

/// 根据节点类型和输入文件,预测输出文件列表。
fn predict_outputs(
    node_type: &str,
    inputs: &[String],
    params: &serde_json::Value,
) -> Vec<String> {
    match node_type {
        // ========== 图片 ==========
        "image_compress" => {
            let format = params.get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("jpg");
            let ext = match format {
                "jpg" | "jpeg" => "jpg",
                "png" => "png",
                "webp" => "webp",
                "avif" => "avif",
                _ => "jpg",
            };
            inputs.iter().map(|f| change_ext(f, ext)).collect()
        }

        // ========== PDF ==========
        "pdf_merge" => {
            let name = params.get("outputName").and_then(|v| v.as_str()).unwrap_or("merged");
            vec![format!("{}.pdf", name)]
        }
        "pdf_split" => {
            let ranges = params.get("ranges").and_then(|v| v.as_str()).unwrap_or("1");
            let count = ranges.split(',').count();
            (0..count).map(|i| {
                let base = strip_ext(&inputs[0]);
                format!("{}_split_{}-{}.pdf", base, i + 1, i + 1)
            }).collect()
        }
        "pdf_compress" => { inputs.to_vec() }

        // ========== 重命名 ==========
        "rename" => {
            let prefix = params.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
            let start_num = params.get("startNum").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
            let suffix = params.get("suffix").and_then(|v| v.as_str()).unwrap_or("");
            inputs.iter().enumerate().map(|(i, f)| {
                let ext = get_ext(f);
                format!("{}{}{}.{}", prefix, start_num + i, suffix, ext)
            }).collect()
        }

        // ========== 视频 ==========
        "video_cut" => {
            let ext = get_ext(&inputs[0]);
            vec![format!("cut_output.{}", if ext.is_empty() { "mp4".to_string() } else { ext })]
        }
        "video_transcode" => {
            vec!["transcode_output.mp4".to_string()]
        }
        "video_to_gif" => {
            vec!["output.gif".to_string()]
        }

        // ========== 音频 ==========
        "extract_audio" | "audio_convert" => {
            let format = params.get("format").and_then(|v| v.as_str()).unwrap_or("mp3");
            vec![format!("audio_output.{}", format)]
        }

        // ========== 文档 ==========
        "office_convert" => {
            let target = params.get("targetFormat")
                .and_then(|v| v.as_str())
                .unwrap_or("pdf");
            inputs.iter().map(|f| change_ext(f, target)).collect()
        }

        // ========== 解压/打包 ==========
        "archive_extract" => {
            inputs.iter().map(|f| {
                let name = strip_ext(f);
                format!("{}/", name) // 解压到同名目录
            }).collect()
        }
        "archive_compress" => {
            vec!["archive.zip".to_string()]
        }

        // ========== 查重 ==========
        "dedup" => {
            // 查重不改变文件名,输出原始文件列表
            inputs.to_vec()
        }

        // ========== 未知 ==========
        _ => inputs.to_vec(),
    }
}

// ============================================================
// 路径工具函数
// ============================================================

fn get_ext(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string()
}

fn strip_ext(path: &str) -> String {
    let p = std::path::Path::new(path);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(path);
    let parent = p.parent().and_then(|par| {
        let s = par.to_str().unwrap_or("");
        if s.is_empty() { None } else { Some(s) }
    });
    if let Some(parent) = parent {
        format!("{}/{}", parent, stem)
    } else {
        stem.to_string()
    }
}

fn change_ext(path: &str, new_ext: &str) -> String {
    format!("{}.{}", strip_ext(path), new_ext)
}

fn add_suffix(path: &str, suffix: &str) -> String {
    let ext = get_ext(path);
    let base = strip_ext(path);
    if ext.is_empty() {
        format!("{}{}", base, suffix)
    } else {
        format!("{}{}.{}", base, suffix, ext)
    }
}

// ============================================================
// Tauri 命令
// ============================================================

/// 预览流水线执行计划。
/// D 的编辑器在用户点[预览]按钮时调用。
/// 返回每一步的输入/输出预测,不实际执行任何文件操作。
#[tauri::command]
pub fn preview_pipeline(
    pipeline_json: String,
    input_files: Vec<String>,
) -> Result<PreviewResult, String> {
    let pipeline: Pipeline = serde_json::from_str(&pipeline_json)
        .map_err(|e| format!("流水线 JSON 解析失败: {}", e))?;
    Ok(dry_run(&pipeline, input_files))
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_pipeline() -> Pipeline {
        Pipeline {
            name: "测试流水线".into(),
            description: "".into(),
            nodes: vec![
                PipelineNode {
                    id: "n1".into(),
                    node_type: "rename".into(),
                    label: "重命名".into(),
                    position: NodePosition::default(),
                    params: serde_json::json!({"prefix": "img_", "startNum": 1, "suffix": ""}),
                },
                PipelineNode {
                    id: "n2".into(),
                    node_type: "image_compress".into(),
                    label: "压缩为WebP".into(),
                    position: NodePosition::default(),
                    params: serde_json::json!({"format": "webp", "quality": 80}),
                },
            ],
            edges: vec![
                PipelineEdge {
                    id: "e1".into(),
                    source: "n1".into(),
                    source_port: "default".into(),
                    target: "n2".into(),
                    target_port: "default".into(),
                },
            ],
        }
    }

    #[test]
    fn test_topological_sort() {
        let pipeline = make_test_pipeline();
        let order = topological_sort(&pipeline).unwrap();
        assert_eq!(order, vec!["n1", "n2"]);
    }

    #[test]
    fn test_dry_run() {
        let pipeline = make_test_pipeline();
        let inputs = vec!["/photos/a.jpg".into(), "/photos/b.png".into()];
        let result = dry_run(&pipeline, inputs);

        assert!(!result.has_cycle);
        assert_eq!(result.steps.len(), 2);
        assert_eq!(result.steps[0].node_type, "rename");
        assert_eq!(result.steps[1].node_type, "image_compress");
        // 重命名后: img_1.jpg, img_2.png
        assert_eq!(result.steps[0].output_files[0], "img_1.jpg");
        assert_eq!(result.steps[0].output_files[1], "img_2.png");
        // 压缩后: img_1.webp, img_2.webp
        assert_eq!(result.steps[1].output_files[0], "img_1.webp");
        assert_eq!(result.steps[1].output_files[1], "img_2.webp");
    }

    #[test]
    fn test_cycle_detection() {
        let mut pipeline = make_test_pipeline();
        pipeline.edges.push(PipelineEdge {
            id: "e2".into(),
            source: "n2".into(),
            source_port: "default".into(),
            target: "n1".into(),
            target_port: "default".into(),
        });
        let result = dry_run(&pipeline, vec![]);
        assert!(result.has_cycle);
    }

    #[test]
    fn test_preview_rename() {
        let result = dry_run(
            &Pipeline {
                name: "test".into(),
                description: "".into(),
                nodes: vec![PipelineNode {
                    id: "n1".into(),
                    node_type: "rename".into(),
                    label: "重命名".into(),
                    position: NodePosition::default(),
                    params: serde_json::json!({"prefix": "photo_", "startNum": 1, "suffix": "_v2"}),
                }],
                edges: vec![],
            },
            vec!["/a/1.jpg".into(), "/a/2.jpg".into()],
        );
        assert_eq!(result.steps[0].output_files[0], "photo_1_v2.jpg");
        assert_eq!(result.steps[0].output_files[1], "photo_2_v2.jpg");
    }
}
