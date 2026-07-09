//! Global development tools living outside project folders: JDKs, SDKs,
//! mobile simulators/emulators, version-manager toolchains and
//! package-manager caches. Everything listed here is re-downloadable through
//! its manager (Xcode, sdkmanager, rustup, nvm, brew...).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use rayon::prelude::*;
use tauri::{AppHandle, Emitter};

use crate::rules::Kind;
use crate::scanner::{dir_size, ArtifactInfo, ProjectInfo, ScanProgress};

pub struct ToolGroup {
    pub name: &'static str,
    /// Ecosystem chip shown in the UI (reuses the "language" badge).
    pub language: &'static str,
    pub kind: Kind,
    /// Absolute path, or `~/`-relative to the user's home directory.
    pub root: &'static str,
    /// true: each child directory is a separate removable artifact;
    /// false: the root directory itself is the single artifact.
    pub per_child: bool,
}

const SIM_DEVICES: &str = "~/Library/Developer/CoreSimulator/Devices";
const AVD_ROOT: &str = "~/.android/avd";

pub static TOOL_GROUPS: &[ToolGroup] = &[
    // SDKs
    ToolGroup {
        name: "Java JDKs (system)",
        language: "Java",
        kind: Kind::Sdk,
        root: "/Library/Java/JavaVirtualMachines",
        per_child: true,
    },
    ToolGroup {
        name: "SDKMAN candidates",
        language: "Java",
        kind: Kind::Sdk,
        root: "~/.sdkman/candidates",
        per_child: true,
    },
    ToolGroup {
        name: "Android SDK",
        language: "Android",
        kind: Kind::Sdk,
        root: "~/Library/Android/sdk",
        per_child: true,
    },
    // Simulators / emulators
    ToolGroup {
        name: "Android emulators (AVD)",
        language: "Android",
        kind: Kind::Simulator,
        root: AVD_ROOT,
        per_child: true,
    },
    ToolGroup {
        name: "iOS Simulators",
        language: "iOS",
        kind: Kind::Simulator,
        root: SIM_DEVICES,
        per_child: true,
    },
    // Xcode
    ToolGroup {
        name: "Xcode DerivedData",
        language: "iOS",
        kind: Kind::Cache,
        root: "~/Library/Developer/Xcode/DerivedData",
        per_child: true,
    },
    ToolGroup {
        name: "Xcode device support",
        language: "iOS",
        kind: Kind::Cache,
        root: "~/Library/Developer/Xcode/iOS DeviceSupport",
        per_child: true,
    },
    ToolGroup {
        name: "Xcode archives",
        language: "iOS",
        kind: Kind::Build,
        root: "~/Library/Developer/Xcode/Archives",
        per_child: false,
    },
    ToolGroup {
        name: "CoreSimulator caches",
        language: "iOS",
        kind: Kind::Cache,
        root: "~/Library/Developer/CoreSimulator/Caches",
        per_child: false,
    },
    // Toolchains installed by version managers
    ToolGroup {
        name: "Rust toolchains",
        language: "Rust",
        kind: Kind::Toolchain,
        root: "~/.rustup/toolchains",
        per_child: true,
    },
    ToolGroup {
        name: "Node versions (nvm)",
        language: "Node.js",
        kind: Kind::Toolchain,
        root: "~/.nvm/versions/node",
        per_child: true,
    },
    ToolGroup {
        name: "Python versions (pyenv)",
        language: "Python",
        kind: Kind::Toolchain,
        root: "~/.pyenv/versions",
        per_child: true,
    },
    ToolGroup {
        name: "asdf installs",
        language: "asdf",
        kind: Kind::Toolchain,
        root: "~/.asdf/installs",
        per_child: true,
    },
    // Package-manager caches
    ToolGroup {
        name: "Gradle cache",
        language: "Java",
        kind: Kind::Cache,
        root: "~/.gradle/caches",
        per_child: false,
    },
    ToolGroup {
        name: "Maven repository",
        language: "Java",
        kind: Kind::Cache,
        root: "~/.m2/repository",
        per_child: false,
    },
    ToolGroup {
        name: "Cargo registry",
        language: "Rust",
        kind: Kind::Cache,
        root: "~/.cargo/registry",
        per_child: false,
    },
    ToolGroup {
        name: "Cargo git checkouts",
        language: "Rust",
        kind: Kind::Cache,
        root: "~/.cargo/git",
        per_child: false,
    },
    ToolGroup {
        name: "npm cache",
        language: "Node.js",
        kind: Kind::Cache,
        root: "~/.npm/_cacache",
        per_child: false,
    },
    ToolGroup {
        name: "pnpm store",
        language: "Node.js",
        kind: Kind::Cache,
        root: "~/Library/pnpm/store",
        per_child: false,
    },
    ToolGroup {
        name: "Yarn cache",
        language: "Node.js",
        kind: Kind::Cache,
        root: "~/Library/Caches/Yarn",
        per_child: false,
    },
    ToolGroup {
        name: "Bun cache",
        language: "Node.js",
        kind: Kind::Cache,
        root: "~/.bun/install/cache",
        per_child: false,
    },
    ToolGroup {
        name: "pip cache",
        language: "Python",
        kind: Kind::Cache,
        root: "~/Library/Caches/pip",
        per_child: false,
    },
    ToolGroup {
        name: "Poetry cache",
        language: "Python",
        kind: Kind::Cache,
        root: "~/Library/Caches/pypoetry",
        per_child: false,
    },
    ToolGroup {
        name: "uv cache",
        language: "Python",
        kind: Kind::Cache,
        root: "~/Library/Caches/uv",
        per_child: false,
    },
    ToolGroup {
        name: "Go module cache",
        language: "Go",
        kind: Kind::Cache,
        root: "~/go/pkg/mod",
        per_child: false,
    },
    ToolGroup {
        name: "Go build cache",
        language: "Go",
        kind: Kind::Cache,
        root: "~/Library/Caches/go-build",
        per_child: false,
    },
    ToolGroup {
        name: "Homebrew cache",
        language: "Homebrew",
        kind: Kind::Cache,
        root: "~/Library/Caches/Homebrew",
        per_child: false,
    },
    ToolGroup {
        name: "CocoaPods cache",
        language: "iOS",
        kind: Kind::Cache,
        root: "~/Library/Caches/CocoaPods",
        per_child: false,
    },
    ToolGroup {
        name: "CocoaPods repos",
        language: "iOS",
        kind: Kind::Cache,
        root: "~/.cocoapods",
        per_child: false,
    },
    ToolGroup {
        name: "Composer cache",
        language: "PHP",
        kind: Kind::Cache,
        root: "~/.composer/cache",
        per_child: false,
    },
];

/// Expand a catalog root to an absolute path (`~/` -> $HOME).
fn expand(root: &str) -> Option<PathBuf> {
    if let Some(rest) = root.strip_prefix("~/") {
        std::env::var_os("HOME").map(|h| Path::new(&h).join(rest))
    } else {
        Some(PathBuf::from(root))
    }
}

/// Safety guard for deletion: `path` must be exactly a catalog root, or a
/// direct child of a per-child catalog root.
pub fn is_tool_path(path: &Path) -> bool {
    TOOL_GROUPS.iter().any(|g| match expand(g.root) {
        Some(root) if g.per_child => path.parent() == Some(root.as_path()),
        Some(root) => path == root,
        None => false,
    })
}

/// Whether `path` is an iOS Simulator device directory (named by UDID).
pub fn is_simulator_device(path: &Path) -> bool {
    expand(SIM_DEVICES).is_some_and(|r| path.parent() == Some(r.as_path()))
}

/// Whether `path` is an Android AVD data directory (`<name>.avd`).
pub fn is_avd(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "avd")
        && expand(AVD_ROOT).is_some_and(|r| path.parent() == Some(r.as_path()))
}

/// UDID -> "iPhone 15 (iOS 17.5)" from `xcrun simctl`, empty map if unavailable.
fn simulator_names() -> HashMap<String, String> {
    let out = match Command::new("xcrun")
        .args(["simctl", "list", "devices", "-j"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return HashMap::new(),
    };
    let json: serde_json::Value = match serde_json::from_slice(&out.stdout) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };
    let mut map = HashMap::new();
    let Some(devices) = json.get("devices").and_then(|d| d.as_object()) else {
        return map;
    };
    for (runtime, list) in devices {
        // "com.apple.CoreSimulator.SimRuntime.iOS-17-5" -> "iOS 17.5"
        let rt = runtime.rsplit('.').next().unwrap_or(runtime);
        let mut parts = rt.split('-');
        let os = parts.next().unwrap_or(rt);
        let ver: Vec<&str> = parts.collect();
        let rt_label = if ver.is_empty() {
            os.to_string()
        } else {
            format!("{} {}", os, ver.join("."))
        };
        for d in list.as_array().into_iter().flatten() {
            let udid = d.get("udid").and_then(|v| v.as_str());
            let name = d.get("name").and_then(|v| v.as_str());
            if let (Some(udid), Some(name)) = (udid, name) {
                map.insert(udid.to_string(), format!("{name} ({rt_label})"));
            }
        }
    }
    map
}

/// Scan the catalog and return one group per tool family that has something
/// to reclaim, shaped like a project so the UI needs no new component.
pub fn scan(app: &AppHandle) -> Vec<ProjectInfo> {
    // (group, expanded root, artifact dirs)
    let raw: Vec<(&ToolGroup, PathBuf, Vec<PathBuf>)> = TOOL_GROUPS
        .iter()
        .filter_map(|g| {
            let root = expand(g.root)?;
            if !root.is_dir() {
                return None;
            }
            let paths = if g.per_child {
                let mut v: Vec<PathBuf> = fs::read_dir(&root)
                    .ok()?
                    .flatten()
                    .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                    .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                    .map(|e| e.path())
                    .collect();
                v.sort();
                v
            } else {
                vec![root.clone()]
            };
            (!paths.is_empty()).then_some((g, root, paths))
        })
        .collect();

    let sim_names = if raw.iter().any(|(g, _, _)| g.root == SIM_DEVICES) {
        simulator_names()
    } else {
        HashMap::new()
    };

    let total: usize = raw.iter().map(|(_, _, p)| p.len()).sum();
    let sized = AtomicU64::new(0);
    let _ = app.emit(
        "scan-progress",
        ScanProgress {
            scanned: 0,
            current: format!("{} tool locations to measure", total),
            phase: "sizing".into(),
        },
    );

    let mut groups: Vec<ProjectInfo> = raw
        .into_par_iter()
        .filter_map(|(g, root, paths)| {
            let mut artifacts: Vec<ArtifactInfo> = paths
                .par_iter()
                .map(|p| {
                    let size = dir_size(p);
                    let done = sized.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ = app.emit(
                        "scan-progress",
                        ScanProgress {
                            scanned: done,
                            current: p.to_string_lossy().to_string(),
                            phase: "sizing".into(),
                        },
                    );
                    let base = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let name = if g.root == SIM_DEVICES {
                        sim_names
                            .get(&base)
                            .cloned()
                            .unwrap_or_else(|| format!("{base} (orphaned)"))
                    } else {
                        base
                    };
                    ArtifactInfo {
                        path: p.to_string_lossy().to_string(),
                        name,
                        kind: g.kind.as_str().to_string(),
                        size,
                    }
                })
                .filter(|a| a.size > 0)
                .collect();

            if artifacts.is_empty() {
                return None;
            }
            artifacts.sort_by_key(|a| std::cmp::Reverse(a.size));
            let total_size = artifacts.iter().map(|a| a.size).sum();
            Some(ProjectInfo {
                path: root.to_string_lossy().to_string(),
                name: g.name.to_string(),
                languages: vec![g.language.to_string()],
                artifacts,
                total_size,
            })
        })
        .collect();

    groups.sort_by_key(|p| std::cmp::Reverse(p.total_size));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_path_guard() {
        let home = std::env::var("HOME").unwrap();
        // Direct child of a per-child root is deletable.
        assert!(is_tool_path(Path::new(&format!(
            "{home}/.rustup/toolchains/stable-aarch64-apple-darwin"
        ))));
        // A per-child root itself is not (would wipe all toolchains at once).
        assert!(!is_tool_path(Path::new(&format!(
            "{home}/.rustup/toolchains"
        ))));
        // Exact-root artifact.
        assert!(is_tool_path(Path::new(&format!("{home}/.m2/repository"))));
        // Anything else is refused.
        assert!(!is_tool_path(Path::new("/tmp/whatever")));
        assert!(!is_tool_path(Path::new(&home)));

        assert!(is_avd(Path::new(&format!(
            "{home}/.android/avd/Pixel_8.avd"
        ))));
        assert!(!is_avd(Path::new(&format!(
            "{home}/.android/avd/Pixel_8.ini"
        ))));
    }
}
