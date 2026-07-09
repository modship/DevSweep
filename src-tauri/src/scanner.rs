//! Recursive project scanner. Walks a directory tree, identifies development
//! projects by their marker files, and reports removable artifact directories
//! together with their on-disk size.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rayon::prelude::*;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

use crate::rules::{matches, Kind, ALWAYS_PRUNE, RULES};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactInfo {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub size: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub path: String,
    pub name: String,
    pub languages: Vec<String>,
    pub artifacts: Vec<ArtifactInfo>,
    pub total_size: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanProgress {
    pub scanned: u64,
    pub current: String,
    pub phase: String,
}

/// An artifact directory discovered during the structural walk, before sizing.
struct RawArtifact {
    path: PathBuf,
    name: String,
    kind: Kind,
}

struct RawProject {
    path: PathBuf,
    languages: Vec<String>,
    artifacts: Vec<RawArtifact>,
}

pub fn scan(root: &Path, app: &AppHandle) -> Result<Vec<ProjectInfo>, String> {
    if !root.is_dir() {
        return Err(format!("Not a directory: {}", root.display()));
    }

    let scanned = Arc::new(AtomicU64::new(0));
    let mut projects: Vec<RawProject> = Vec::new();
    {
        let count = scanned.clone();
        let mut emit = |dir: &Path| {
            let c = count.fetch_add(1, Ordering::Relaxed) + 1;
            if c.is_multiple_of(200) {
                let _ = app.emit(
                    "scan-progress",
                    ScanProgress {
                        scanned: c,
                        current: dir.to_string_lossy().to_string(),
                        phase: "scanning".into(),
                    },
                );
            }
        };
        visit(root, &mut projects, &mut emit);
    }

    // Compute artifact sizes in parallel.
    let total_artifacts: usize = projects.iter().map(|p| p.artifacts.len()).sum();
    let sized = Arc::new(AtomicU64::new(0));
    let _ = app.emit(
        "scan-progress",
        ScanProgress {
            scanned: 0,
            current: format!("{} targets to measure", total_artifacts),
            phase: "sizing".into(),
        },
    );

    let result: Vec<ProjectInfo> = projects
        .into_par_iter()
        .map(|p| {
            let mut artifacts: Vec<ArtifactInfo> = p
                .artifacts
                .par_iter()
                .map(|a| {
                    let size = dir_size(&a.path);
                    let done = sized.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ = app.emit(
                        "scan-progress",
                        ScanProgress {
                            scanned: done,
                            current: a.path.to_string_lossy().to_string(),
                            phase: "sizing".into(),
                        },
                    );
                    ArtifactInfo {
                        path: a.path.to_string_lossy().to_string(),
                        name: a.name.clone(),
                        kind: a.kind.as_str().to_string(),
                        size,
                    }
                })
                .filter(|a| a.size > 0)
                .collect();

            artifacts.sort_by_key(|a| std::cmp::Reverse(a.size));
            let total_size = artifacts.iter().map(|a| a.size).sum();

            ProjectInfo {
                name: p
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.path.to_string_lossy().to_string()),
                path: p.path.to_string_lossy().to_string(),
                languages: p.languages,
                artifacts,
                total_size,
            }
        })
        .filter(|p| !p.artifacts.is_empty())
        .collect();

    let mut result = result;
    result.sort_by_key(|p| std::cmp::Reverse(p.total_size));
    Ok(result)
}

fn visit(dir: &Path, projects: &mut Vec<RawProject>, emit: &mut impl FnMut(&Path)) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // permission denied, etc. — skip silently
    };

    emit(dir);

    let mut file_names: Vec<String> = Vec::new();
    let mut subdirs: Vec<(String, PathBuf)> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        match entry.file_type() {
            Ok(ft) if ft.is_dir() => subdirs.push((name, entry.path())),
            Ok(_) => file_names.push(name),
            Err(_) => {}
        }
    }

    // Identify which rules apply to this directory and collect artifacts.
    let mut languages: BTreeSet<&'static str> = BTreeSet::new();
    let mut raw_artifacts: Vec<RawArtifact> = Vec::new();
    let mut pruned_here: BTreeSet<String> = BTreeSet::new();

    for rule in RULES {
        let has_marker = rule
            .markers
            .iter()
            .any(|m| file_names.iter().any(|f| matches(m, f)));
        if !has_marker {
            continue;
        }
        languages.insert(rule.language);
        for artifact in rule.artifacts {
            for (name, path) in &subdirs {
                if matches(artifact.name, name) && !pruned_here.contains(name) {
                    raw_artifacts.push(RawArtifact {
                        path: path.clone(),
                        name: name.clone(),
                        kind: artifact.kind,
                    });
                    pruned_here.insert(name.clone());
                }
            }
        }
    }

    if !raw_artifacts.is_empty() {
        projects.push(RawProject {
            path: dir.to_path_buf(),
            languages: languages.iter().map(|s| s.to_string()).collect(),
            artifacts: raw_artifacts,
        });
    }

    // Recurse, skipping VCS dirs, always-pruned heavy dirs, and artifacts found here.
    for (name, path) in &subdirs {
        if name.starts_with('.') && is_hidden_skip(name) {
            continue;
        }
        if ALWAYS_PRUNE.contains(&name.as_str()) {
            continue;
        }
        if pruned_here.contains(name) {
            continue;
        }
        visit(path, projects, emit);
    }
}

/// Skip well-known hidden directories that never contain projects to clean.
fn is_hidden_skip(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".hg" | ".svn" | ".idea" | ".vscode" | ".Trash" | ".cache"
    )
}

/// Sum of file sizes under `path`, in bytes. Follows no symlinks.
pub(crate) fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::env;

    fn collect(root: &Path) -> Vec<RawProject> {
        let mut projects = Vec::new();
        let mut emit = |_: &Path| {};
        visit(root, &mut projects, &mut emit);
        projects
    }

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }

    /// Map of project dir name -> set of artifact names detected.
    fn summarize(projects: &[RawProject]) -> HashMap<String, Vec<String>> {
        projects
            .iter()
            .map(|p| {
                let name = p.path.file_name().unwrap().to_string_lossy().to_string();
                let mut arts: Vec<String> = p.artifacts.iter().map(|a| a.name.clone()).collect();
                arts.sort();
                (name, arts)
            })
            .collect()
    }

    fn temp_root(tag: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("devsweep-test-{}", tag));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detects_multiple_languages() {
        let root = temp_root("multi");

        // Rust project
        touch(&root.join("rustapp/Cargo.toml"));
        touch(&root.join("rustapp/target/debug/bin"));
        // Node project
        touch(&root.join("webapp/package.json"));
        touch(&root.join("webapp/node_modules/lodash/index.js"));
        touch(&root.join("webapp/dist/bundle.js"));
        // Python project
        touch(&root.join("py/pyproject.toml"));
        touch(&root.join("py/.venv/lib/python"));
        touch(&root.join("py/__pycache__/mod.pyc"));
        // Plain folder, nothing to clean
        touch(&root.join("docs/readme.md"));

        let summary = summarize(&collect(&root));

        assert_eq!(summary.get("rustapp"), Some(&vec!["target".to_string()]));
        assert_eq!(
            summary.get("webapp"),
            Some(&vec!["dist".to_string(), "node_modules".to_string()])
        );
        assert_eq!(
            summary.get("py"),
            Some(&vec![".venv".to_string(), "__pycache__".to_string()])
        );
        assert!(!summary.contains_key("docs"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn does_not_recurse_into_artifacts() {
        // A package.json nested inside node_modules must not be reported as its
        // own project, and node_modules must not be descended into.
        let root = temp_root("prune");
        touch(&root.join("app/package.json"));
        touch(&root.join("app/node_modules/dep/package.json"));
        touch(&root.join("app/node_modules/dep/node_modules/sub/index.js"));

        let projects = collect(&root);
        assert_eq!(projects.len(), 1, "only the top-level app is a project");
        assert_eq!(
            projects[0].path.file_name().unwrap().to_string_lossy(),
            "app"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn finds_nested_monorepo_projects() {
        // Projects in subfolders (not under artifact dirs) are all found.
        let root = temp_root("monorepo");
        touch(&root.join("packages/a/package.json"));
        touch(&root.join("packages/a/node_modules/x/i.js"));
        touch(&root.join("packages/b/Cargo.toml"));
        touch(&root.join("packages/b/target/out"));

        let summary = summarize(&collect(&root));
        assert_eq!(summary.len(), 2);
        assert!(summary.contains_key("a"));
        assert!(summary.contains_key("b"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn glob_markers_and_artifacts() {
        let root = temp_root("glob");
        // .NET: *.csproj marker -> bin/obj
        touch(&root.join("svc/Service.csproj"));
        touch(&root.join("svc/bin/Service.dll"));
        touch(&root.join("svc/obj/x"));
        // Python egg-info (glob artifact)
        touch(&root.join("lib/setup.py"));
        touch(&root.join("lib/mylib.egg-info/PKG-INFO"));

        let summary = summarize(&collect(&root));
        assert_eq!(
            summary.get("svc"),
            Some(&vec!["bin".to_string(), "obj".to_string()])
        );
        assert_eq!(
            summary.get("lib"),
            Some(&vec!["mylib.egg-info".to_string()])
        );

        let _ = fs::remove_dir_all(&root);
    }
}
