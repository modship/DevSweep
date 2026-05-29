export type ArtifactKind = "dependencies" | "build" | "cache";

export interface ArtifactInfo {
  path: string;
  name: string;
  kind: ArtifactKind;
  size: number;
}

export interface ProjectInfo {
  path: string;
  name: string;
  languages: string[];
  artifacts: ArtifactInfo[];
  totalSize: number;
}

export interface ScanProgress {
  scanned: number;
  current: string;
  phase: "scanning" | "sizing";
}

export interface CleanResult {
  path: string;
  success: boolean;
  error: string | null;
}

export interface DiskInfo {
  /** Total size of the volume, in bytes. */
  total: number;
  /** Space currently available to the user, in bytes. */
  free: number;
}
