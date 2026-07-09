// 应用库入口:声明各功能模块,并组装 Tauri 应用。
// 模块职责详见 docs/design/m0-skeleton.md §3。

mod common;
mod commands;
mod pipeline;
mod worker;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::rename::rename_files,
            commands::dedup::scan_duplicate_files,
            commands::dedup::delete_duplicate,
            commands::video::check_ffmpeg,
            commands::video::cut_video,
            commands::video::transcode_video,
            commands::video::video_to_gif,
            commands::video::detect_gpu_encoders,
            commands::video::extract_audio,
            pipeline::registry::get_node_types,
            pipeline::registry::get_node_types_grouped,
            pipeline::preview::preview_pipeline,
            commands::diskusage::scan_directory,
            commands::image::compress_images,
            commands::pdf::merge_pdfs,
            commands::pdf::split_pdf,
            commands::pdf::compress_pdf,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
