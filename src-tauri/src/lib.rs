mod cleaner;
mod rules;
mod scanner;
mod tools;

use std::path::{Path, PathBuf};

use cleaner::CleanResult;
use scanner::ProjectInfo;
use serde::Serialize;
use tauri::AppHandle;

/// Capacity of the volume that holds a given path.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DiskInfo {
    /// Total size of the volume, in bytes.
    total: u64,
    /// Space currently available to the user, in bytes.
    free: u64,
}

#[tauri::command]
fn disk_usage(path: String) -> Result<DiskInfo, String> {
    let p = Path::new(&path);
    let total = fs2::total_space(p).map_err(|e| e.to_string())?;
    let free = fs2::available_space(p).map_err(|e| e.to_string())?;
    Ok(DiskInfo { total, free })
}

#[tauri::command]
async fn scan_directory(app: AppHandle, path: String) -> Result<Vec<ProjectInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || scanner::scan(&PathBuf::from(path), &app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn scan_tools(app: AppHandle) -> Result<Vec<ProjectInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || tools::scan(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn clean_paths(app: AppHandle, paths: Vec<String>) -> Result<Vec<CleanResult>, String> {
    tauri::async_runtime::spawn_blocking(move || cleaner::clean(paths, &app))
        .await
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_directory,
            scan_tools,
            clean_paths,
            disk_usage
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
