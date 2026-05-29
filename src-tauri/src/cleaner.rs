//! Safe deletion of selected artifact directories.

use std::fs;
use std::path::Path;

use rayon::prelude::*;
use serde::Serialize;

use crate::rules::is_known_artifact;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
}

pub fn clean(paths: Vec<String>) -> Vec<CleanResult> {
    paths
        .into_par_iter()
        .map(|p| {
            let path = Path::new(&p);
            match validate(path) {
                Err(e) => CleanResult {
                    path: p,
                    success: false,
                    error: Some(e),
                },
                Ok(()) => match fs::remove_dir_all(path) {
                    Ok(()) => CleanResult {
                        path: p,
                        success: true,
                        error: None,
                    },
                    Err(e) => CleanResult {
                        path: p,
                        success: false,
                        error: Some(e.to_string()),
                    },
                },
            }
        })
        .collect()
}

/// Guard against deleting anything that is not a recognized artifact directory.
fn validate(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err("Path does not exist or is not a directory".into());
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid directory name".to_string())?;

    if !is_known_artifact(name) {
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
