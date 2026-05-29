<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { SvelteSet } from "svelte/reactivity";
  import { onDestroy } from "svelte";

  import type { CleanResult, DiskInfo, ProjectInfo, ScanProgress, ArtifactKind } from "$lib/types";
  import { formatBytes, kindLabel } from "$lib/format";
  import ProjectCard from "$lib/ProjectCard.svelte";

  type View = "idle" | "scanning" | "results" | "cleaning";

  let view = $state<View>("idle");
  let rootPath = $state<string | null>(null);
  let progress = $state<ScanProgress | null>(null);
  let projects = $state<ProjectInfo[]>([]);
  let disk = $state<DiskInfo | null>(null);
  let error = $state<string | null>(null);

  const selected = new SvelteSet<string>();

  // Filters
  let search = $state("");
  const activeLanguages = new SvelteSet<string>();
  const activeKinds = new SvelteSet<string>();

  // Clean flow
  let showConfirm = $state(false);
  let cleanSummary = $state<{ freed: number; failed: number } | null>(null);

  let unlisten: UnlistenFn | null = null;

  // ---- derived ----
  let allLanguages = $derived(
    [...new Set(projects.flatMap((p) => p.languages))].sort(),
  );

  let filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return projects
      .map((p) => {
        const langOk =
          activeLanguages.size === 0 ||
          p.languages.some((l) => activeLanguages.has(l));
        const searchOk =
          q === "" ||
          p.name.toLowerCase().includes(q) ||
          p.path.toLowerCase().includes(q);
        if (!langOk || !searchOk) return null;
        const artifacts =
          activeKinds.size === 0
            ? p.artifacts
            : p.artifacts.filter((a) => activeKinds.has(a.kind));
        if (artifacts.length === 0) return null;
        const totalSize = artifacts.reduce((s, a) => s + a.size, 0);
        return { ...p, artifacts, totalSize };
      })
      .filter((p): p is ProjectInfo => p !== null);
  });

  let totalReclaimable = $derived(
    projects.reduce((s, p) => s + p.totalSize, 0),
  );
  let totalArtifacts = $derived(
    projects.reduce((s, p) => s + p.artifacts.length, 0),
  );

  let allArtifactPaths = $derived(
    new Map(projects.flatMap((p) => p.artifacts.map((a) => [a.path, a.size]))),
  );
  let selectedSize = $derived(
    [...selected].reduce((s, path) => s + (allArtifactPaths.get(path) ?? 0), 0),
  );
  let visiblePaths = $derived(
    filtered.flatMap((p) => p.artifacts.map((a) => a.path)),
  );

  // ---- disk context ----
  let diskUsed = $derived(disk ? Math.max(0, disk.total - disk.free) : 0);
  // Reclaimable space as a share of the whole disk, and of the used space.
  let pctOfDisk = $derived(
    disk && disk.total > 0 ? (totalReclaimable / disk.total) * 100 : 0,
  );
  let pctOfUsed = $derived(
    diskUsed > 0 ? (Math.min(totalReclaimable, diskUsed) / diskUsed) * 100 : 0,
  );
  // Bar segments (percent of total disk), clamped so they never overflow.
  let segReclaim = $derived(disk && disk.total > 0 ? Math.min(totalReclaimable, diskUsed) / disk.total * 100 : 0);
  let segOther = $derived(disk && disk.total > 0 ? (diskUsed / disk.total) * 100 - segReclaim : 0);
  let freeAfter = $derived(disk ? disk.free + totalReclaimable : 0);

  function fmtPct(p: number): string {
    if (p > 0 && p < 1) return "<1%";
    return `${Math.round(p)}%`;
  }

  // ---- actions ----
  async function pickFolder() {
    const dir = await open({ directory: true, multiple: false, title: "Choose a folder to scan" });
    if (typeof dir === "string") {
      rootPath = dir;
      await runScan();
    }
  }

  async function runScan() {
    if (!rootPath) return;
    error = null;
    cleanSummary = null;
    projects = [];
    selected.clear();
    activeLanguages.clear();
    activeKinds.clear();
    progress = { scanned: 0, current: "", phase: "scanning" };
    view = "scanning";
    disk = null;

    invoke<DiskInfo>("disk_usage", { path: rootPath })
      .then((d) => (disk = d))
      .catch(() => (disk = null));

    unlisten?.();
    unlisten = await listen<ScanProgress>("scan-progress", (e) => {
      progress = e.payload;
    });

    try {
      projects = await invoke<ProjectInfo[]>("scan_directory", { path: rootPath });
      view = "results";
    } catch (e) {
      error = String(e);
      view = "idle";
    } finally {
      unlisten?.();
      unlisten = null;
    }
  }

  function toggleLang(lang: string) {
    if (activeLanguages.has(lang)) activeLanguages.delete(lang);
    else activeLanguages.add(lang);
  }
  function toggleKind(kind: string) {
    if (activeKinds.has(kind)) activeKinds.delete(kind);
    else activeKinds.add(kind);
  }

  function selectAllVisible() {
    for (const path of visiblePaths) selected.add(path);
  }
  function selectNone() {
    selected.clear();
  }

  async function confirmClean() {
    showConfirm = false;
    const paths = [...selected];
    if (paths.length === 0) return;
    view = "cleaning";
    let results: CleanResult[] = [];
    try {
      results = await invoke<CleanResult[]>("clean_paths", { paths });
    } catch (e) {
      error = String(e);
      view = "results";
      return;
    }

    const okPaths = new Set(results.filter((r) => r.success).map((r) => r.path));
    const freed = [...okPaths].reduce((s, p) => s + (allArtifactPaths.get(p) ?? 0), 0);
    const failed = results.filter((r) => !r.success).length;

    // Remove cleaned artifacts from the model.
    projects = projects
      .map((p) => {
        const artifacts = p.artifacts.filter((a) => !okPaths.has(a.path));
        return { ...p, artifacts, totalSize: artifacts.reduce((s, a) => s + a.size, 0) };
      })
      .filter((p) => p.artifacts.length > 0);

    for (const p of okPaths) selected.delete(p);
    cleanSummary = { freed, failed };
    view = "results";
  }

  onDestroy(() => unlisten?.());
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && showConfirm && (showConfirm = false)} />

<div class="app">
  <!-- Header -->
  <header class="topbar">
    <div class="brand">
      <img class="logo" src="/logo.svg" alt="DevSweep logo" />
      <div>
        <div class="app-name">DevSweep</div>
        <div class="app-sub">Reclaim your disk space</div>
      </div>
    </div>

    <div class="topbar-right">
      {#if rootPath}
        <div class="rootpath" title={rootPath}>{rootPath}</div>
      {/if}
      <button class="btn primary" onclick={pickFolder} disabled={view === "scanning" || view === "cleaning"}>
        {rootPath ? "Change folder" : "Choose folder"}
      </button>
      {#if rootPath && view === "results"}
        <button class="btn ghost" onclick={runScan}>↻ Rescan</button>
      {/if}
    </div>
  </header>

  {#if cleanSummary}
    <div class="toast success">
      ✓ Freed {formatBytes(cleanSummary.freed)}
      {#if cleanSummary.failed > 0}· {cleanSummary.failed} failed{/if}
      <button class="toast-x" onclick={() => (cleanSummary = null)}>✕</button>
    </div>
  {/if}
  {#if error}
    <div class="toast error">
      ⚠ {error}
      <button class="toast-x" onclick={() => (error = null)}>✕</button>
    </div>
  {/if}

  <!-- IDLE -->
  {#if view === "idle"}
    <div class="center">
      <div class="hero">
        <img class="hero-logo" src="/logo.svg" alt="DevSweep logo" />
        <h1>Reclaim your disk space</h1>
        <p>
          Scan a folder to find the reinstallable dependencies and build
          artifacts of all your projects — Rust, Node.js, Python, Java, Go,
          .NET, PHP, and many more.
        </p>
        <button class="btn primary big" onclick={pickFolder}>Choose a folder to scan</button>
      </div>
    </div>

  <!-- SCANNING -->
  {:else if view === "scanning"}
    <div class="center">
      <div class="scanning">
        <div class="spinner"></div>
        <h2>{progress?.phase === "sizing" ? "Measuring sizes…" : "Scanning…"}</h2>
        {#if progress}
          <div class="scan-count">
            {progress.phase === "sizing"
              ? `${progress.scanned} target(s) measured`
              : `${progress.scanned} folders scanned`}
          </div>
          <div class="scan-path">{progress.current}</div>
        {/if}
      </div>
    </div>

  <!-- CLEANING -->
  {:else if view === "cleaning"}
    <div class="center">
      <div class="scanning">
        <div class="spinner"></div>
        <h2>Cleaning…</h2>
        <div class="scan-count">{selected.size} target(s)</div>
      </div>
    </div>

  <!-- RESULTS -->
  {:else if view === "results"}
    <!-- Stats -->
    <section class="stats">
      <div class="stat">
        <div class="stat-val accent">{formatBytes(totalReclaimable)}</div>
        <div class="stat-lbl">
          Reclaimable space{#if disk && pctOfDisk > 0} · {fmtPct(pctOfDisk)} of your disk{/if}
        </div>
      </div>
      <div class="stat">
        <div class="stat-val">{projects.length}</div>
        <div class="stat-lbl">Projects</div>
      </div>
      <div class="stat">
        <div class="stat-val">{totalArtifacts}</div>
        <div class="stat-lbl">Targets</div>
      </div>
      <div class="stat highlight">
        <div class="stat-val">{formatBytes(selectedSize)}</div>
        <div class="stat-lbl">Selected</div>
      </div>
    </section>

    {#if disk && projects.length > 0}
      <section class="diskbar">
        <div class="diskbar-head">
          <span class="diskbar-title">
            DevSweep can free <strong class="accent">{formatBytes(totalReclaimable)}</strong>
            — <strong class="accent">{fmtPct(pctOfDisk)}</strong> of your
            {formatBytes(disk.total)} disk
            <span class="dim">({fmtPct(pctOfUsed)} of used space)</span>
          </span>
          <span class="diskbar-after">
            {formatBytes(disk.free)} free → <strong>{formatBytes(freeAfter)}</strong>
          </span>
        </div>
        <div class="diskbar-track" title="{formatBytes(diskUsed)} used of {formatBytes(disk.total)}">
          <div class="seg other" style="width: {segOther}%"></div>
          <div class="seg reclaim" style="width: {segReclaim}%"></div>
        </div>
        <div class="diskbar-legend">
          <span><i class="dot other"></i> Used by other files</span>
          <span><i class="dot reclaim"></i> Reclaimable</span>
          <span><i class="dot free"></i> Free</span>
        </div>
      </section>
    {/if}

    {#if projects.length === 0}
      <div class="center">
        <div class="empty">
          <div class="empty-icon">✨</div>
          <h2>Nothing to clean</h2>
          <p>No reinstallable artifacts found in this folder.</p>
        </div>
      </div>
    {:else}
      <!-- Toolbar -->
      <div class="toolbar">
        <div class="searchbox">
          <span class="search-icon">⌕</span>
          <input type="text" placeholder="Search a project…" bind:value={search} />
        </div>

        <div class="chips">
          {#each ["dependencies", "build", "cache"] as kind}
            <button
              class="chip"
              class:on={activeKinds.has(kind)}
              onclick={() => toggleKind(kind)}
            >
              {kindLabel(kind as ArtifactKind)}
            </button>
          {/each}
        </div>

        <div class="chips langs">
          {#each allLanguages as lang}
            <button
              class="chip"
              class:on={activeLanguages.has(lang)}
              onclick={() => toggleLang(lang)}
            >
              {lang}
            </button>
          {/each}
        </div>

        <div class="select-actions">
          <button class="btn ghost sm" onclick={selectAllVisible}>Select all</button>
          <button class="btn ghost sm" onclick={selectNone} disabled={selected.size === 0}>None</button>
        </div>
      </div>

      <!-- List -->
      <div class="list">
        {#if filtered.length === 0}
          <div class="no-match">No project matches the filters.</div>
        {/if}
        {#each filtered as project (project.path)}
          <ProjectCard {project} {selected} />
        {/each}
      </div>
    {/if}

    <!-- Action bar -->
    {#if selected.size > 0}
      <div class="actionbar">
        <div class="actionbar-info">
          <strong>{selected.size}</strong> target(s) selected ·
          <strong class="accent">{formatBytes(selectedSize)}</strong> to free
        </div>
        <button class="btn danger" onclick={() => (showConfirm = true)}>
          🗑 Clean selection
        </button>
      </div>
    {/if}
  {/if}

  <!-- Confirm modal -->
  {#if showConfirm}
    <div
      class="modal-backdrop"
      onclick={(e) => e.target === e.currentTarget && (showConfirm = false)}
      role="presentation"
    >
      <div class="modal" role="dialog" aria-modal="true" tabindex="-1">
        <div class="modal-icon">🗑</div>
        <h2>Confirm deletion</h2>
        <p>
          You are about to delete <strong>{selected.size}</strong> artifact folder(s),
          freeing <strong class="accent">{formatBytes(selectedSize)}</strong>.
        </p>
        <p class="warn">
          These folders will be permanently deleted. They are regenerable
          (reinstall the dependencies or rebuild the project).
        </p>
        <div class="modal-actions">
          <button class="btn ghost" onclick={() => (showConfirm = false)}>Cancel</button>
          <button class="btn danger" onclick={confirmClean}>Delete permanently</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
  }

  /* Topbar */
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elev);
    flex-shrink: 0;
    gap: 16px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .logo {
    width: 44px;
    height: 44px;
    border-radius: 11px;
    display: block;
  }
  .app-name {
    font-weight: 700;
    font-size: 16px;
  }
  .app-sub {
    font-size: 12px;
    color: var(--text-faint);
  }
  .topbar-right {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .rootpath {
    font-size: 12px;
    color: var(--text-dim);
    max-width: 360px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    background: var(--bg);
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }

  /* Buttons */
  .btn {
    padding: 9px 16px;
    border-radius: var(--radius-sm);
    font-weight: 600;
    font-size: 13px;
    white-space: nowrap;
    transition: all 0.13s;
  }
  .btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .btn.primary {
    background: var(--accent);
    color: white;
  }
  .btn.primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }
  .btn.ghost {
    background: var(--bg-elev-2);
    color: var(--text-dim);
    border: 1px solid var(--border);
  }
  .btn.ghost:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text);
  }
  .btn.danger {
    background: var(--danger);
    color: white;
  }
  .btn.danger:hover:not(:disabled) {
    background: var(--danger-hover);
  }
  .btn.big {
    padding: 14px 28px;
    font-size: 15px;
  }
  .btn.sm {
    padding: 6px 12px;
    font-size: 12px;
  }

  /* Centered states */
  .center {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 40px;
    overflow: auto;
  }
  .hero {
    text-align: center;
    max-width: 520px;
  }
  .hero-logo {
    width: 96px;
    height: 96px;
    border-radius: 22px;
    margin-bottom: 8px;
  }
  .hero h1 {
    font-size: 28px;
    margin: 8px 0 12px;
  }
  .hero p {
    color: var(--text-dim);
    font-size: 15px;
    margin-bottom: 28px;
  }

  .scanning {
    text-align: center;
  }
  .scanning h2 {
    margin: 18px 0 6px;
  }
  .scan-count {
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
  .scan-path {
    margin-top: 10px;
    font-size: 12px;
    color: var(--text-faint);
    font-family: ui-monospace, monospace;
    max-width: 560px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
  }
  .spinner {
    width: 46px;
    height: 46px;
    border: 4px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    margin: 0 auto;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty,
  .no-match {
    text-align: center;
    color: var(--text-dim);
  }
  .empty-icon {
    font-size: 56px;
  }
  .no-match {
    padding: 40px;
  }

  /* Stats */
  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    padding: 16px 20px 4px;
    flex-shrink: 0;
  }
  .stat {
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 18px;
  }
  .stat.highlight {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .stat-val {
    font-size: 24px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .stat-val.accent {
    color: var(--accent);
  }
  .stat-lbl {
    font-size: 12px;
    color: var(--text-faint);
    margin-top: 2px;
  }

  /* Disk context bar */
  .diskbar {
    margin: 12px 20px 0;
    padding: 14px 18px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    flex-shrink: 0;
  }
  .diskbar-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 10px;
    flex-wrap: wrap;
  }
  .diskbar-title {
    font-size: 14px;
    color: var(--text);
  }
  .diskbar-title .dim {
    color: var(--text-faint);
    font-size: 13px;
  }
  .diskbar-after {
    font-size: 13px;
    color: var(--text-dim);
    white-space: nowrap;
  }
  .diskbar-track {
    display: flex;
    height: 12px;
    border-radius: 7px;
    overflow: hidden;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
  }
  .seg {
    height: 100%;
    transition: width 0.4s ease;
  }
  .seg.other {
    background: var(--border-strong);
  }
  .seg.reclaim {
    background: linear-gradient(90deg, var(--accent), var(--accent-hover));
    box-shadow: 0 0 12px var(--accent);
  }
  .diskbar-legend {
    display: flex;
    gap: 18px;
    margin-top: 9px;
    font-size: 12px;
    color: var(--text-faint);
  }
  .diskbar-legend i {
    display: inline-block;
    width: 9px;
    height: 9px;
    border-radius: 3px;
    margin-right: 5px;
    vertical-align: baseline;
  }
  .dot.other {
    background: var(--border-strong);
  }
  .dot.reclaim {
    background: var(--accent);
  }
  .dot.free {
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
  }

  /* Toolbar */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 20px;
    flex-wrap: wrap;
    flex-shrink: 0;
  }
  .searchbox {
    position: relative;
    flex: 0 0 240px;
  }
  .search-icon {
    position: absolute;
    left: 11px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-faint);
    font-size: 16px;
  }
  .searchbox input {
    width: 100%;
    padding: 9px 12px 9px 32px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: 13px;
  }
  .searchbox input::placeholder {
    color: var(--text-faint);
  }
  .chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .chips.langs {
    flex: 1;
  }
  .chip {
    padding: 6px 12px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 600;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    color: var(--text-dim);
    transition: all 0.12s;
  }
  .chip:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }
  .chip.on {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
  .select-actions {
    display: flex;
    gap: 6px;
    margin-left: auto;
  }

  /* List */
  .list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  /* Action bar */
  .actionbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    background: var(--bg-elev);
    border-top: 1px solid var(--border-strong);
    box-shadow: 0 -6px 20px rgba(0, 0, 0, 0.25);
    flex-shrink: 0;
  }
  .actionbar-info {
    font-size: 14px;
    color: var(--text-dim);
  }
  .accent {
    color: var(--accent);
  }

  /* Toast */
  .toast {
    margin: 12px 20px 0;
    padding: 11px 16px;
    border-radius: var(--radius-sm);
    font-weight: 600;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .toast.success {
    background: color-mix(in srgb, var(--success) 16%, transparent);
    color: var(--success);
    border: 1px solid color-mix(in srgb, var(--success) 35%, transparent);
  }
  .toast.error {
    background: color-mix(in srgb, var(--danger) 16%, transparent);
    color: var(--danger);
    border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent);
  }
  .toast-x {
    margin-left: auto;
    color: inherit;
    opacity: 0.7;
  }
  .toast-x:hover {
    opacity: 1;
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    display: grid;
    place-items: center;
    z-index: 100;
    backdrop-filter: blur(2px);
  }
  .modal {
    background: var(--bg-elev);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 28px;
    max-width: 440px;
    text-align: center;
    box-shadow: var(--shadow);
  }
  .modal-icon {
    font-size: 40px;
  }
  .modal h2 {
    margin: 8px 0 12px;
  }
  .modal p {
    color: var(--text-dim);
    margin: 8px 0;
  }
  .modal .warn {
    font-size: 13px;
    color: var(--text-faint);
  }
  .modal-actions {
    display: flex;
    gap: 10px;
    justify-content: center;
    margin-top: 22px;
  }
</style>
