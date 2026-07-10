// ★ 节点类型注册表(A-7)
//
// 把 M1/M2 所有原子功能注册为流水线节点。
// D 的前端编辑器调用 get_node_types() 获取此列表来渲染节点面板。

use crate::pipeline::model::*;

/// 返回所有已注册的节点类型。
/// D 的编辑器初始化时调用此命令,获取可拖拽的节点列表。
pub fn get_all_node_types() -> Vec<NodeType> {
    vec![
        // ============================================================
        // 图片处理(B 实现)
        // ============================================================
        NodeType {
            id: "image_compress".into(),
            name: "图片压缩".into(),
            category: "图片".into(),
            description: "批量压缩图片并转换格式,支持调整尺寸".into(),
            icon: "Picture".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "图片文件".into(),
                port_type: "file[]".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "压缩后图片".into(),
                port_type: "file[]".into(),
            }],
            params: vec![
                slider_param("quality", "质量", 1.0, 100.0, 1.0, 80.0, false, "数值越大质量越高"),
                select_param("format", "输出格式", &[
                    ("jpg", "JPG"), ("png", "PNG"), ("webp", "WebP"), ("avif", "AVIF"),
                ], "webp", true, ""),
                number_param("maxWidth", "最大宽度(px)", None, false, "超过此宽度会等比缩放"),
                number_param("maxHeight", "最大高度(px)", None, false, "超过此高度会等比缩放"),
            ],
        },
        // ============================================================
        // PDF 处理(B 实现)
        // ============================================================
        NodeType {
            id: "pdf_merge".into(),
            name: "PDF 合并".into(),
            category: "文档".into(),
            description: "将多个 PDF 合并为一个文件".into(),
            icon: "Document".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "PDF 文件(多个)".into(),
                port_type: "file[]".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "合并后 PDF".into(),
                port_type: "file".into(),
            }],
            params: vec![
                string_param("outputName", "输出文件名", "merged.pdf", false, ""),
            ],
        },
        NodeType {
            id: "pdf_split".into(),
            name: "PDF 拆分".into(),
            category: "文档".into(),
            description: "按页码范围将 PDF 拆分为多个文件".into(),
            icon: "Document".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "PDF 文件".into(),
                port_type: "file".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "拆分后 PDF".into(),
                port_type: "file[]".into(),
            }],
            params: vec![
                string_param("ranges", "页码范围", "1-5,6-10", true, "每行一个范围,如 1-5"),
            ],
        },
        NodeType {
            id: "pdf_compress".into(),
            name: "PDF 压缩".into(),
            category: "文档".into(),
            description: "压缩 PDF 文件大小".into(),
            icon: "Document".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "PDF 文件".into(),
                port_type: "file".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "压缩后 PDF".into(),
                port_type: "file".into(),
            }],
            params: vec![],
        },
        // ============================================================
        // 文件重命名(C 实现) —— 适配 C 的 rename_files API
        // ============================================================
        NodeType {
            id: "rename".into(),
            name: "批量重命名".into(),
            category: "文件".into(),
            description: "为文件批量添加前缀+序号+后缀".into(),
            icon: "Edit".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "文件".into(),
                port_type: "file[]".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "重命名后文件".into(),
                port_type: "file[]".into(),
            }],
            params: vec![
                string_param("prefix", "前缀", "img_", false, "文件名前缀,如 img_"),
                number_param("startNum", "起始数字", Some(1.0), false, "序号起始值,如 1 或 001"),
                string_param("suffix", "后缀", "", false, "文件名后缀(可选),如 _v2"),
            ],
        },
        // ============================================================
        // 查重(C 实现) —— 适配 C 的 scan_duplicate_files + delete_duplicate API
        // ============================================================
        NodeType {
            id: "dedup".into(),
            name: "查重清理".into(),
            category: "文件".into(),
            description: "扫描目录找出重复文件并批量删除".into(),
            icon: "Search".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "扫描目录".into(),
                port_type: "directory".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "去重后文件列表".into(),
                port_type: "file[]".into(),
            }],
            params: vec![
                select_param("keepStrategy", "保留策略", &[
                    ("newest", "保留最新"), ("largest", "保留最大"), ("first", "保留第一个"),
                ], "newest", false, "每组重复文件保留哪一个,其余删除"),
            ],
        },
        // ============================================================
        // 视频处理(A 实现 ✓)
        // ============================================================
        NodeType {
            id: "video_cut".into(),
            name: "视频剪切".into(),
            category: "视频".into(),
            description: "从视频中截取指定时间段".into(),
            icon: "VideoCamera".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "视频文件".into(),
                port_type: "file".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "剪切后视频".into(),
                port_type: "file".into(),
            }],
            params: vec![
                string_param("start", "开始时间", "00:00:00", true, "HH:MM:SS 或秒数"),
                string_param("end", "结束时间", "00:01:00", true, "HH:MM:SS 或秒数"),
                select_param("mode", "剪切模式", &[
                    ("fast", "快速(-c copy)"), ("accurate", "精确(重编码)"),
                ], "fast", false, "快速模式秒级完成但不精确到帧"),
            ],
        },
        NodeType {
            id: "video_transcode".into(),
            name: "视频转码".into(),
            category: "视频".into(),
            description: "转换视频格式与编码".into(),
            icon: "VideoCamera".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "视频文件".into(),
                port_type: "file".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "转码后视频".into(),
                port_type: "file".into(),
            }],
            params: vec![
                select_param("videoCodec", "视频编码", &[
                    ("h264", "H.264"), ("h265", "H.265/HEVC"), ("vp9", "VP9"), ("av1", "AV1"),
                ], "h264", false, ""),
                slider_param("crf", "质量(CRF)", 0.0, 51.0, 1.0, 23.0, false, "越小质量越高,推荐 18-28"),
                string_param("encoder", "硬件编码器", "", false, "留空自动选择,如 h264_nvenc"),
            ],
        },
        NodeType {
            id: "video_to_gif".into(),
            name: "视频转 GIF".into(),
            category: "视频".into(),
            description: "截取视频片段生成 GIF 动图".into(),
            icon: "VideoCamera".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "视频文件".into(),
                port_type: "file".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "GIF 文件".into(),
                port_type: "file".into(),
            }],
            params: vec![
                string_param("start", "开始时间", "00:00:00", true, ""),
                number_param("duration", "持续时长(秒)", Some(3.0), true, ""),
                number_param("fps", "帧率", Some(10.0), false, "默认 10fps"),
                number_param("width", "输出宽度(px)", Some(480.0), false, ""),
            ],
        },
        NodeType {
            id: "extract_audio".into(),
            name: "提取音频".into(),
            category: "音频".into(),
            description: "从视频中提取音频轨道".into(),
            icon: "Headset".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "视频文件".into(),
                port_type: "file".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "音频文件".into(),
                port_type: "file".into(),
            }],
            params: vec![
                select_param("format", "音频格式", &[
                    ("mp3", "MP3"), ("aac", "AAC"), ("flac", "FLAC"), ("wav", "WAV"), ("ogg", "OGG"),
                ], "mp3", false, ""),
                select_param("bitrate", "码率", &[
                    ("128k", "128kbps"), ("192k", "192kbps"), ("256k", "256kbps"), ("320k", "320kbps"),
                ], "192k", false, ""),
            ],
        },
        // ============================================================
        // 音频处理(C 实现)
        // ============================================================
        NodeType {
            id: "audio_convert".into(),
            name: "音频转换".into(),
            category: "音频".into(),
            description: "音频格式互转(MP3/AAC/FLAC/WAV/OGG)".into(),
            icon: "Headset".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "音频文件".into(),
                port_type: "file".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "转换后音频".into(),
                port_type: "file".into(),
            }],
            params: vec![
                select_param("format", "目标格式", &[
                    ("mp3", "MP3"), ("aac", "AAC"), ("flac", "FLAC"), ("wav", "WAV"), ("ogg", "OGG"),
                ], "mp3", true, ""),
                select_param("bitrate", "码率", &[
                    ("128k", "128kbps"), ("192k", "192kbps"), ("256k", "256kbps"), ("320k", "320kbps"),
                ], "192k", false, ""),
            ],
        },
        // ============================================================
        // Office 转换(B 实现)
        // ============================================================
        NodeType {
            id: "office_convert".into(),
            name: "文档转换".into(),
            category: "文档".into(),
            description: "Office 文档互转(Word/Excel/PPT ↔ PDF)".into(),
            icon: "Document".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "Office 文档".into(),
                port_type: "file".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "转换后文档".into(),
                port_type: "file".into(),
            }],
            params: vec![
                select_param("targetFormat", "目标格式", &[
                    ("pdf", "PDF"), ("docx", "Word"), ("html", "HTML"),
                ], "pdf", true, ""),
            ],
        },
        // ============================================================
        // 解压/压缩(B 实现)
        // ============================================================
        NodeType {
            id: "archive_extract".into(),
            name: "解压".into(),
            category: "文件".into(),
            description: "批量解压 zip/7z/rar/tar.gz 文件".into(),
            icon: "FolderOpened".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "压缩包文件".into(),
                port_type: "file[]".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "解压后目录".into(),
                port_type: "directory".into(),
            }],
            params: vec![],
        },
        NodeType {
            id: "archive_compress".into(),
            name: "打包".into(),
            category: "文件".into(),
            description: "将文件批量打包为 zip".into(),
            icon: "Folder".into(),
            inputs: vec![PortDef {
                id: "input".into(),
                label: "待打包文件".into(),
                port_type: "file[]".into(),
            }],
            outputs: vec![PortDef {
                id: "output".into(),
                label: "压缩包".into(),
                port_type: "file".into(),
            }],
            params: vec![
                select_param("format", "压缩格式", &[
                    ("zip", "ZIP"),
                ], "zip", false, ""),
                string_param("password", "密码(可选)", "", false, "留空则不加密"),
            ],
        },
    ]
}

// ============================================================
// 辅助:快速构建 ParamDef
// ============================================================

fn slider_param(key: &str, label: &str, min: f64, max: f64, step: f64, default: f64, required: bool, help: &str) -> ParamDef {
    ParamDef {
        key: key.into(), label: label.into(),
        param_type: ParamType::Slider { min, max, step },
        default: Some(serde_json::json!(default)),
        required, help: help.into(),
        options: vec![],
    }
}

fn select_param(key: &str, label: &str, options: &[(&str, &str)], default: &str, required: bool, help: &str) -> ParamDef {
    ParamDef {
        key: key.into(), label: label.into(),
        param_type: ParamType::Select,
        default: Some(serde_json::json!(default)),
        required, help: help.into(),
        options: options.iter().map(|(v, l)| SelectOption { value: v.to_string(), label: l.to_string() }).collect(),
    }
}

fn string_param(key: &str, label: &str, default: &str, required: bool, help: &str) -> ParamDef {
    ParamDef {
        key: key.into(), label: label.into(),
        param_type: ParamType::String,
        default: Some(serde_json::json!(default)),
        required, help: help.into(),
        options: vec![],
    }
}

fn number_param(key: &str, label: &str, default: Option<f64>, required: bool, help: &str) -> ParamDef {
    ParamDef {
        key: key.into(), label: label.into(),
        param_type: ParamType::Number,
        default: default.map(|v| serde_json::json!(v)),
        required, help: help.into(),
        options: vec![],
    }

}

// ============================================================
// Tauri 命令:提供给 D
// ============================================================

/// 获取所有可用的流水线节点类型。
/// D 的前端编辑器在初始化时调用此命令来渲染左侧节点面板。
#[tauri::command]
pub fn get_node_types() -> Vec<NodeType> {
    get_all_node_types()
}

/// 获取节点类型按分类分组(方便 D 渲染分类面板)。
#[tauri::command]
pub fn get_node_types_grouped() -> Vec<NodeTypeGroup> {
    let types = get_all_node_types();
    let mut groups: std::collections::HashMap<String, Vec<NodeType>> = std::collections::HashMap::new();
    for nt in types {
        groups.entry(nt.category.clone()).or_default().push(nt);
    }
    groups.into_iter().map(|(category, items)| NodeTypeGroup { category, items }).collect()
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeTypeGroup {
    pub category: String,
    pub items: Vec<NodeType>,
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_not_empty() {
        let types = get_all_node_types();
        assert!(!types.is_empty(), "注册表不应为空");
        // 至少包含图片/PDF/视频/音频/文件 5 大类
        let categories: std::collections::HashSet<_> = types.iter().map(|t| &t.category).collect();
        assert!(categories.len() >= 4);
    }

    #[test]
    fn test_node_type_ids_unique() {
        let types = get_all_node_types();
        let mut ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for t in &types {
            assert!(ids.insert(&t.id), "重复的节点 ID: {}", t.id);
        }
    }

    #[test]
    fn test_grouped() {
        let groups = get_node_types_grouped();
        assert!(!groups.is_empty());
    }
}
