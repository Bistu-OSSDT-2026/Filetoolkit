// commands 模块:暴露给前端调用的 Tauri 命令。
// 每个功能独立成子模块。

pub mod image;   // 图片处理(B)
pub mod pdf;     // PDF 处理(B)
pub mod video;   // 视频处理(A)
pub mod rename;  // 批量重命名(C)
pub mod dedup;   // 查重(C)
pub mod audio;   // 音频处理(C)
pub mod checksum; // 文件校验(C)
pub mod ocr;     // PDF OCR(C)
pub mod diskusage; // 磁盘可视化(D)
pub mod office;  // Office 转换(B)
pub mod archive; // 解压/压缩(B)

/// 调通命令(保留)
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("来自 Rust 后端的问候:你好,{}!FileToolkit 已就绪。", name)
}
