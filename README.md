<div align="center">

<img src="static/logo.svg" width="120" alt="DevSweep logo" />

# DevSweep

### Reclaim gigabytes of disk space in one click.

DevSweep scans any folder, finds the **reinstallable** dependencies and
build artifacts of every project inside it, and lets you wipe them — safely,
across **15+ languages**.

[![CI](https://github.com/USER/devsweep/actions/workflows/ci.yml/badge.svg)](https://github.com/USER/devsweep/actions/workflows/ci.yml)
[![Release](https://github.com/USER/devsweep/actions/workflows/release.yml/badge.svg)](https://github.com/USER/devsweep/actions/workflows/release.yml)
![Platforms](https://img.shields.io/badge/macOS%20%7C%20Windows%20%7C%20Linux-informational)

</div>

---

`node_modules`, `target`, `.venv`, `build`, `bin/obj`, `vendor`… every project
you've ever touched is quietly hoarding gigabytes of files you can regenerate in
seconds. DevSweep finds them all and gives them back to you.

> ⚡ A native desktop app built with **Rust + Tauri 2**. Tiny (~3 MB), fast, and
> private — nothing ever leaves your machine.

## ✨ Features

- **🔍 Recursive scan** — point it at `~/dev` (or your whole home) and it finds
  every project, including monorepos and deeply nested folders.
- **🌍 15+ ecosystems** — Rust, Node.js, Python, Java, .NET, Go, PHP, Dart/Flutter,
  Elixir, Swift, C/C++, Zig, Haskell, and more.
- **📊 See before you delete** — every removable folder shown with its exact size,
  grouped by project, sorted biggest-first.
- **🎯 Pick exactly what you want** — filter by language or by category
  (Dependencies / Build / Cache), search, and select per-project or per-folder.
- **🛡️ Safe by design** — the backend refuses to delete anything that isn't a
  recognized artifact directory, and every cleanup is confirmed first.
- **🚀 Blazing fast** — parallel sizing with Rust + Rayon; heavy folders are
  pruned from the walk so a scan of thousands of projects takes seconds.

## 📦 Install

Grab the latest installer for your platform from the
[**Releases**](https://github.com/USER/devsweep/releases) page:

| Platform | File |
|---|---|
| macOS (Intel + Apple Silicon) | `.dmg` |
| Windows | `.msi` or setup `.exe` |
| Linux | `.AppImage`, `.deb`, or `.rpm` |

## 🗂️ Supported languages

| Language | Detected by | Cleaned |
|---|---|---|
| Rust | `Cargo.toml` | `target` |
| Node.js | `package.json` | `node_modules`, `dist`, `build`, `.next`, `.nuxt`, `.svelte-kit`, `.turbo`, `.angular`, `.parcel-cache`, `.vite`, `out`, `.output` |
| Python | `requirements.txt`, `pyproject.toml`, `setup.py`, `Pipfile`… | `.venv`, `venv`, `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.tox`, `*.egg-info`, `build`, `dist` |
| Java (Maven) | `pom.xml` | `target` |
| Java (Gradle) | `build.gradle(.kts)` | `build`, `.gradle` |
| .NET | `*.csproj`, `*.sln`… | `bin`, `obj` |
| Go | `go.mod` | `vendor` |
| PHP | `composer.json` | `vendor` |
| Dart / Flutter | `pubspec.yaml` | `.dart_tool`, `build` |
| Elixir | `mix.exs` | `_build`, `deps` |
| Swift | `Package.swift` | `.build` |
| C / C++ (CMake) | `CMakeLists.txt` | `build` |
| Zig | `build.zig` | `zig-cache`, `zig-out` |
| Haskell | `stack.yaml`, `*.cabal` | `.stack-work`, `dist-newstyle` |

Each target is tagged **Dependencies**, **Build**, or **Cache** so you stay in control.

## 🛡️ Is it safe?

Yes. Everything DevSweep removes is **regenerable** — reinstall your
dependencies (`npm install`, `cargo build`, `pip install`…) or rebuild the
project and it comes right back. On top of that:

- The Rust backend validates every path before deletion: it will only remove a
  folder whose name is a **known artifact** and that lives inside a project — never
  a source folder, never a filesystem root.
- Nothing is deleted without an explicit confirmation showing exactly how much
  space you'll free.

## 🛠️ Build from source

Prerequisites: Rust (stable), Node ≥ 18, pnpm, and the
[Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
pnpm install
pnpm tauri dev      # run in development
pnpm tauri build    # build a native installer for the current platform
```

Tests:

```bash
cd src-tauri && cargo test   # detection-logic unit tests
pnpm check                   # Svelte / TypeScript type check
```

## 🤖 Tech stack

- **Backend:** Rust — recursive scanner with artifact pruning, parallel sizing
  ([rayon](https://github.com/rayon-rs/rayon)), safe deletion.
- **Frontend:** [Svelte 5](https://svelte.dev) + TypeScript.
- **Shell:** [Tauri 2](https://v2.tauri.app) — small, secure, cross-platform.

Cross-platform installers are built automatically by GitHub Actions on every
tagged release (`git tag v0.1.0 && git push --tags`). See
[`.github/workflows`](.github/workflows).

## 📄 License

MIT
