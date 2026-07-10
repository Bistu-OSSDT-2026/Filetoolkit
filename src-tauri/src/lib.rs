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
            // -- M1: rename & dedup (C + D 实现) --
            commands::rename::rename_files,
            commands::dedup::scan_duplicate_files,
            commands::dedup::delete_duplicate,
            // -- M2: video/audio (A + D 实现) --
            commands::video::check_ffmpeg,
            commands::video::cut_video,
            commands::video::transcode_video,
            commands::video::video_to_gif,
            commands::video::detect_gpu_encoders,
            commands::video::extract_audio,
            // -- M3: pipeline engine (A) --
            pipeline::registry::get_node_types,
            pipeline::registry::get_node_types_grouped,
            pipeline::preview::preview_pipeline,
            pipeline::executor::run_pipeline,
            // -- common --
            common::dependency::check_dependency,
            common::dependency::check_all_dependencies,
            common::dependency::clear_dependency_cache,
            // -- M4: disk usage (D) --
            commands::diskusage::scan_directory,
            // -- M1: image (B) --
            commands::image::compress_images,
            // -- M1: PDF (B) --
            commands::pdf::merge_pdfs,
            commands::pdf::split_pdf,
            commands::pdf::compress_pdf,
            // -- worker --
            worker::cancel_batch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
