//! Detection rules: which marker files identify a project, and which
//! directories within that project are safely removable (reinstallable
//! dependencies, build output, caches).

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Reinstallable dependencies (node_modules, vendor, .venv, deps...)
    Dependencies,
    /// Compiler / bundler output (target, build, bin, obj, dist...)
    Build,
    /// Tool caches (__pycache__, .pytest_cache, .gradle...)
    Cache,
    /// Installed SDKs (JDKs, Android SDK components...)
    Sdk,
    /// Mobile simulators / emulators (iOS Simulator devices, Android AVDs)
    Simulator,
    /// Language toolchains installed by version managers (rustup, nvm, pyenv...)
    Toolchain,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Dependencies => "dependencies",
            Kind::Build => "build",
            Kind::Cache => "cache",
            Kind::Sdk => "sdk",
            Kind::Simulator => "simulator",
            Kind::Toolchain => "toolchain",
        }
    }
}

pub struct Artifact {
    /// Directory name to clean. A leading `*` means suffix match (e.g. `*.egg-info`).
    pub name: &'static str,
    pub kind: Kind,
}

pub struct Rule {
    pub language: &'static str,
    /// Any of these filenames present in a directory identifies it as a project root.
    /// A leading `*` means suffix match (e.g. `*.csproj`).
    pub markers: &'static [&'static str],
    pub artifacts: &'static [Artifact],
}

macro_rules! art {
    ($name:expr, $kind:ident) => {
        Artifact {
            name: $name,
            kind: Kind::$kind,
        }
    };
}

pub static RULES: &[Rule] = &[
    Rule {
        language: "Rust",
        markers: &["Cargo.toml"],
        artifacts: &[art!("target", Build)],
    },
    Rule {
        language: "Node.js",
        markers: &["package.json"],
        artifacts: &[
            art!("node_modules", Dependencies),
            art!(".next", Build),
            art!(".nuxt", Build),
            art!(".svelte-kit", Build),
            art!(".turbo", Cache),
            art!(".angular", Cache),
            art!(".parcel-cache", Cache),
            art!(".vite", Cache),
            art!("dist", Build),
            art!("build", Build),
            art!("out", Build),
            art!(".output", Build),
        ],
    },
    Rule {
        language: "Python",
        markers: &[
            "requirements.txt",
            "pyproject.toml",
            "setup.py",
            "setup.cfg",
            "Pipfile",
        ],
        artifacts: &[
            art!(".venv", Dependencies),
            art!("venv", Dependencies),
            art!("env", Dependencies),
            art!("__pycache__", Cache),
            art!(".pytest_cache", Cache),
            art!(".mypy_cache", Cache),
            art!(".ruff_cache", Cache),
            art!(".tox", Cache),
            art!("*.egg-info", Build),
            art!("build", Build),
            art!("dist", Build),
        ],
    },
    Rule {
        language: "Java (Maven)",
        markers: &["pom.xml"],
        artifacts: &[art!("target", Build)],
    },
    Rule {
        language: "Java (Gradle)",
        markers: &[
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
        ],
        artifacts: &[art!("build", Build), art!(".gradle", Cache)],
    },
    Rule {
        language: ".NET",
        markers: &["*.csproj", "*.fsproj", "*.vbproj", "*.sln"],
        artifacts: &[art!("bin", Build), art!("obj", Build)],
    },
    Rule {
        language: "Go",
        markers: &["go.mod"],
        artifacts: &[art!("vendor", Dependencies)],
    },
    Rule {
        language: "PHP",
        markers: &["composer.json"],
        artifacts: &[art!("vendor", Dependencies)],
    },
    Rule {
        language: "Dart / Flutter",
        markers: &["pubspec.yaml"],
        artifacts: &[art!(".dart_tool", Cache), art!("build", Build)],
    },
    Rule {
        language: "Elixir",
        markers: &["mix.exs"],
        artifacts: &[art!("_build", Build), art!("deps", Dependencies)],
    },
    Rule {
        language: "Swift",
        markers: &["Package.swift"],
        artifacts: &[art!(".build", Build)],
    },
    Rule {
        language: "C / C++ (CMake)",
        markers: &["CMakeLists.txt"],
        artifacts: &[art!("build", Build)],
    },
    Rule {
        language: "Zig",
        markers: &["build.zig"],
        artifacts: &[
            art!("zig-cache", Cache),
            art!(".zig-cache", Cache),
            art!("zig-out", Build),
        ],
    },
    Rule {
        language: "Haskell",
        markers: &["stack.yaml", "*.cabal"],
        artifacts: &[art!(".stack-work", Build), art!("dist-newstyle", Build)],
    },
];

/// Returns true if `name` matches `pattern` (supporting a single leading `*` suffix wildcard).
pub fn matches(pattern: &str, name: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix('*') {
        name.ends_with(suffix)
    } else {
        pattern.eq_ignore_ascii_case(name) || pattern == name
    }
}

/// Directory names that are always pruned from traversal (VCS metadata and
/// unambiguous heavy artifact dirs) regardless of project context. Ambiguous
/// names like `build`, `dist`, `bin`, `obj`, `vendor`, `deps`, `env` are pruned
/// only when matched as an artifact inside an identified project.
pub static ALWAYS_PRUNE: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    ".venv",
    "venv",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".angular",
    ".parcel-cache",
    ".vite",
    ".output",
    ".gradle",
    ".dart_tool",
    ".build",
    "_build",
    ".stack-work",
    "dist-newstyle",
    "zig-cache",
    ".zig-cache",
    "zig-out",
];

/// Whether a directory basename is a recognized removable artifact name.
/// Used as a safety guard before deletion.
pub fn is_known_artifact(name: &str) -> bool {
    for rule in RULES {
        for a in rule.artifacts {
            if matches(a.name, name) {
                return true;
            }
        }
    }
    false
}
