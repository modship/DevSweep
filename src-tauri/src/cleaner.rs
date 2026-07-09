//! Safe deletion of selected artifact directories and global dev tools.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use rayon::prelude::*;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::rules::is_known_artifact;
use crate::scanner::ScanProgress;
use crate::tools;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
}

pub fn clean(paths: Vec<String>, app: &AppHandle) -> Vec<CleanResult> {
    let done = AtomicU64::new(0);
    paths
        .into_par_iter()
        .map(|p| {
            let _ = app.emit(
                "scan-progress",
                ScanProgress {
                    scanned: done.load(Ordering::Relaxed),
                    current: p.clone(),
                    phase: "cleaning".into(),
                },
            );
            let res = validate(Path::new(&p)).and_then(|()| remove(Path::new(&p)));
            let _ = app.emit(
                "scan-progress",
                ScanProgress {
                    scanned: done.fetch_add(1, Ordering::Relaxed) + 1,
                    current: p.clone(),
                    phase: "cleaning".into(),
                },
            );
            CleanResult {
                path: p,
                success: res.is_ok(),
                error: res.err(),
            }
        })
        .collect()
}

/// Guard against deleting anything that is not a recognized artifact
/// directory or a cataloged global tool location.
fn validate(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err("Path does not exist or is not a directory".into());
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid directory name".to_string())?;

    if !is_known_artifact(name) && !tools::is_tool_path(path) {
        return Err(format!(
            "Safety refusal: \"{}\" is not a recognized artifact directory",
            name
        ));
    }
    // Never delete a filesystem root or a top-level directory.
    if path.parent().map(|p| p.parent().is_none()).unwrap_or(true) {
        return Err("Safety refusal: path too close to the filesystem root".into());
    }
    Ok(())
}

fn remove(path: &Path) -> Result<(), String> {
    if tools::is_simulator_device(path) {
        // Proper uninstall: unregister from CoreSimulator, which also deletes
        // the data directory. Fall through to plain removal for orphaned
        // devices that simctl no longer knows about.
        if let Some(udid) = path.file_name().map(|n| n.to_string_lossy().to_string()) {
            let ok = Command::new("xcrun")
                .args(["simctl", "delete", &udid])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok && !path.exists() {
                return Ok(());
            }
        }
    }

    remove_dir_all_force(path)?;

    if tools::is_avd(path) {
        // The AVD manager keeps a sibling <name>.ini pointing at the deleted
        // directory; drop it too so the emulator list stays consistent.
        let _ = fs::remove_file(path.with_extension("ini"));
    }
    Ok(())
}

/// `remove_dir_all` that also handles read-only trees (Go module cache) and,
/// on macOS, system-owned SDKs (/Library JDKs) via an admin prompt.
fn remove_dir_all_force(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            let _ = Command::new("chmod").args(["-R", "u+w"]).arg(path).status();
            if fs::remove_dir_all(path).is_ok() {
                return Ok(());
            }
            #[cfg(target_os = "macos")]
            {
                // Root-owned (e.g. /Library/Java): one admin-password prompt.
                // The path travels as an argv item, never spliced into shell.
                let ok = Command::new("osascript")
                    .args([
                        "-e",
                        "on run argv",
                        "-e",
                        "do shell script \"rm -rf \" & quoted form of item 1 of argv with administrator privileges",
                        "-e",
                        "end run",
                    ])
                    .arg(path)
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if ok && !path.exists() {
                    return Ok(());
                }
            }
            Err(e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}
