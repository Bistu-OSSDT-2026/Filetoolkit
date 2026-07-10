mod commands;
mod common;
mod pipeline;
mod worker;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::video::detect_gpu_encoders,
            commands::video::cut_video,
            commands::video::transcode_video,
            commands::video::video_to_gif,
            commands::video::extract_audio,
            common::dependency::check_dependency,
            common::dependency::check_all_dependencies,
            common::dependency::clear_dependency_cache,
            pipeline::registry::get_node_types,
            pipeline::registry::get_node_types_grouped,
            pipeline::preview::preview_pipeline,
            pipeline::executor::run_pipeline,
            worker::cancel_batch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
