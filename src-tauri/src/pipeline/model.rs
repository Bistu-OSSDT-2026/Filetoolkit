// ★ 流水线数据模型 —— 这是 A(后端引擎)与 D(前端编辑器)的接口约定。
//
// 这些类型是前后端共享的 schema。D 在 Vue Flow 编辑器中构建的 JSON
// 必须符合此结构,A 的 executor 才能正确解析和执行。
// 修改此文件前,必须与 D 同步。

use serde::{Deserialize, Serialize};

// ============================================================
// 顶层结构
// ============================================================

/// 一条流水线:包含若干节点和它们之间的连线。
/// D 的编辑器导出的 JSON 就是这个结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    /// 流水线名称(用户自定义)
    pub name: String,
    /// 描述
    #[serde(default)]
    pub description: String,
    /// 所有节点
    pub nodes: Vec<PipelineNode>,
    /// 所有连线
    pub edges: Vec<PipelineEdge>,
}

// ============================================================
// 节点
// ============================================================

/// 流水线中的一个操作节点。
/// 每个节点对应一个原子功能(图片压缩、PDF合并、重命名……)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineNode {
    /// 节点唯一 ID(编辑器生成,如 "node-1")
    pub id: String,
    /// 节点类型 ID(对应 NodeTypeRegistry 中的 key,如 "image_compress")
    pub node_type: String,
    /// 显示标签
    pub label: String,
    /// 节点在画布上的位置(前端编辑器用,后端忽略)
    #[serde(default)]
    pub position: NodePosition,
    /// 节点参数(JSON,结构由对应 NodeType 的 input_schema 定义)
    #[serde(default)]
    pub params: serde_json::Value,
}

/// 节点在画布上的坐标(纯前端数据,后端执行时忽略)。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

// ============================================================
// 连线
// ============================================================

/// 一条连线:从 source 节点的输出 → target 节点的输入。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineEdge {
    /// 连线唯一 ID(如 "edge-1")
    pub id: String,
    /// 源节点 ID
    pub source: String,
    /// 源节点的输出端口名(默认为 "output")
    #[serde(default = "default_port")]
    pub source_port: String,
    /// 目标节点 ID
    pub target: String,
    /// 目标节点的输入端口名(默认为 "input")
    #[serde(default = "default_port")]
    pub target_port: String,
}

fn default_port() -> String {
    "default".into()
}

// ============================================================
// 节点类型注册表(NodeType)
// ============================================================

/// 节点类型定义 —— 描述一种可用的操作类型。
/// D 的编辑器根据此信息渲染节点面板和参数表单。
/// A-7 的 registry.rs 会维护所有已注册 NodeType 的列表。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeType {
    /// 类型标识(如 "image_compress", "pdf_merge")
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 分类(前端按分类分组显示)
    pub category: String,
    /// 描述
    pub description: String,
    /// 图标名(前端图标库的 key,如 "Picture", "Document")
    #[serde(default)]
    pub icon: String,
    /// 输入端口
    #[serde(default)]
    pub inputs: Vec<PortDef>,
    /// 输出端口
    #[serde(default)]
    pub outputs: Vec<PortDef>,
    /// 参数定义列表
    #[serde(default)]
    pub params: Vec<ParamDef>,
}

/// 端口定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortDef {
    pub id: String,
    pub label: String,
    /// 端口类型: "file" | "file[]" | "string" | "number"
    #[serde(default = "default_port_type")]
    pub port_type: String,
}

fn default_port_type() -> String {
    "file".into()
}

/// 参数定义 —— 描述节点的一个可配置参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamDef {
    /// 参数 key(如 "quality", "format")
    pub key: String,
    /// 显示标签
    pub label: String,
    /// 参数类型
    pub param_type: ParamType,
    /// 默认值(JSON)
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// 是否必填
    #[serde(default)]
    pub required: bool,
    /// 帮助提示
    #[serde(default)]
    pub help: String,
    /// 选项列表(当 param_type 为 Select 时)
    #[serde(default)]
    pub options: Vec<SelectOption>,
}

/// 参数类型枚举。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParamType {
    /// 文本输入
    String,
    /// 数字输入
    Number,
    /// 开关
    Boolean,
    /// 下拉选择
    Select,
    /// 文件选择
    File,
    /// 目录选择
    Directory,
    /// 整数滑块
    Slider { min: f64, max: f64, step: f64 },
}

/// 下拉选项。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

// ============================================================
// 运行时状态
// ============================================================

/// 流水线执行状态(后端 → 前端的事件负载)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineProgress {
    /// 流水线 ID
    pub pipeline_id: String,
    /// 当前执行到第几步
    pub current_step: u32,
    /// 总步数
    pub total_steps: u32,
    /// 当前执行的节点 ID
    #[serde(default)]
    pub current_node_id: String,
    /// 当前节点名称
    #[serde(default)]
    pub current_node_label: String,
    /// 状态
    pub status: PipelineStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PipelineStatus {
    Ready,
    Running,
    Paused,
    Completed,
    Failed(String),
    Cancelled,
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_serialization() {
        let pipeline = Pipeline {
            name: "测试流水线".into(),
            description: "".into(),
            nodes: vec![
                PipelineNode {
                    id: "n1".into(),
                    node_type: "image_compress".into(),
                    label: "压缩图片".into(),
                    position: NodePosition { x: 100.0, y: 200.0 },
                    params: serde_json::json!({"quality": 80, "format": "webp"}),
                },
            ],
            edges: vec![
                PipelineEdge {
                    id: "e1".into(),
                    source: "n1".into(),
                    source_port: "output".into(),
                    target: "n2".into(),
                    target_port: "input".into(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&pipeline).unwrap();
        assert!(json.contains("image_compress"));
        assert!(json.contains("webp"));

        let parsed: Pipeline = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(parsed.edges.len(), 1);
    }

    #[test]
    fn test_node_type_example() {
        let nt = NodeType {
            id: "image_compress".into(),
            name: "图片压缩".into(),
            category: "图片".into(),
            description: "批量压缩图片并转换格式".into(),
            icon: "Picture".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "输入文件".into(),
                port_type: "file[]".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "输出文件".into(),
                port_type: "file[]".into(),
            }],
            params: vec![
                ParamDef {
                    key: "quality".into(),
                    label: "质量".into(),
                    param_type: ParamType::Slider { min: 1.0, max: 100.0, step: 1.0 },
                    default: Some(serde_json::json!(80)),
                    required: false,
                    help: "输出质量(1-100)".into(),
                    options: vec![],
                },
                ParamDef {
                    key: "format".into(),
                    label: "输出格式".into(),
                    param_type: ParamType::Select,
                    default: Some(serde_json::json!("webp")),
                    required: true,
                    help: "".into(),
                    options: vec![
                        SelectOption { value: "jpg".into(), label: "JPG".into() },
                        SelectOption { value: "png".into(), label: "PNG".into() },
                        SelectOption { value: "webp".into(), label: "WebP".into() },
                    ],
                },
            ],
        };

        let json = serde_json::to_string_pretty(&nt).unwrap();
        assert!(json.contains("image_compress"));
        // 验证可以反序列化回来
        let back: NodeType = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "image_compress");
        assert_eq!(back.params.len(), 2);
    }
}
