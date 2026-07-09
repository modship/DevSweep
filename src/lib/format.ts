import type { ArtifactKind } from "./types";

/** Human-readable byte size (binary units). */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, i);
  const decimals = value >= 100 || i === 0 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(decimals)} ${units[i]}`;
}

/** Accent color per language, used for the project badge. */
export function languageColor(language: string): string {
  const map: Record<string, string> = {
    Rust: "#dea584",
    "Node.js": "#8cc84b",
    Python: "#4b8bbe",
    "Java (Maven)": "#e76f00",
    "Java (Gradle)": "#02303a",
    ".NET": "#9b4f96",
    Go: "#00add8",
    PHP: "#777bb4",
    "Dart / Flutter": "#42a5f5",
    Elixir: "#9b59b6",
    Swift: "#f05138",
    "C / C++ (CMake)": "#649ad2",
    Zig: "#f7a41d",
    Haskell: "#5e5086",
    Java: "#e76f00",
    Android: "#3ddc84",
    iOS: "#0a84ff",
    Homebrew: "#f9d094",
    asdf: "#9d7bd8",
  };
  return map[language] ?? "#7c8aa5";
}

export function kindLabel(kind: ArtifactKind): string {
  switch (kind) {
    case "dependencies":
      return "Dependencies";
    case "build":
      return "Build";
    case "cache":
      return "Cache";
    case "sdk":
      return "SDK";
    case "simulator":
      return "Simulator";
    case "toolchain":
      return "Toolchain";
  }
}

export function kindColor(kind: ArtifactKind): string {
  switch (kind) {
    case "dependencies":
      return "#38bdf8";
    case "build":
      return "#fbbf24";
    case "cache":
      return "#a78bfa";
    case "sdk":
      return "#34d399";
    case "simulator":
      return "#f472b6";
    case "toolchain":
      return "#fb923c";
  }
}
