<script lang="ts">
  import type { SvelteSet } from "svelte/reactivity";
  import type { ProjectInfo } from "./types";
  import { formatBytes, kindColor, kindLabel, languageColor } from "./format";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  interface Props {
    project: ProjectInfo;
    selected: SvelteSet<string>;
  }
  let { project, selected }: Props = $props();

  let expanded = $state(false);

  let selectedCount = $derived(
    project.artifacts.filter((a) => selected.has(a.path)).length,
  );
  let selectedSize = $derived(
    project.artifacts
      .filter((a) => selected.has(a.path))
      .reduce((s, a) => s + a.size, 0),
  );
  let allSelected = $derived(selectedCount === project.artifacts.length);
  let someSelected = $derived(selectedCount > 0 && !allSelected);

  function toggleProject() {
    if (allSelected) {
      for (const a of project.artifacts) selected.delete(a.path);
    } else {
      for (const a of project.artifacts) selected.add(a.path);
    }
  }

  function toggleArtifact(path: string) {
    if (selected.has(path)) selected.delete(path);
    else selected.add(path);
  }

  async function reveal(path: string, e: Event) {
    e.stopPropagation();
    try {
      await revealItemInDir(path);
    } catch (_) {
      /* ignore */
    }
  }
</script>

<div class="card" class:active={selectedCount > 0}>
  <div
    class="head"
    role="button"
    tabindex="0"
    onclick={() => (expanded = !expanded)}
    onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), (expanded = !expanded))}
  >
    <label class="checkbox">
      <input
        type="checkbox"
        checked={allSelected}
        indeterminate={someSelected}
        onclick={(e) => e.stopPropagation()}
        onchange={toggleProject}
      />
      <span class="box"></span>
    </label>

    <div class="info">
      <div class="title-row">
        <span class="name">{project.name}</span>
        {#each project.languages as lang}
          <span class="lang" style="--c: {languageColor(lang)}">{lang}</span>
        {/each}
      </div>
      <div class="path" title={project.path}>{project.path}</div>
    </div>

    <div class="meta">
      <span class="count">{project.artifacts.length} target{project.artifacts.length > 1 ? "s" : ""}</span>
      <span class="size">{formatBytes(project.totalSize)}</span>
    </div>

    <span class="chev" class:open={expanded}>›</span>
  </div>

  {#if expanded}
    <div class="artifacts">
      {#each project.artifacts as a (a.path)}
        <div class="artifact" class:sel={selected.has(a.path)}>
          <label class="checkbox">
            <input
              type="checkbox"
              checked={selected.has(a.path)}
              onchange={() => toggleArtifact(a.path)}
            />
            <span class="box"></span>
          </label>
          <span class="kind" style="--k: {kindColor(a.kind)}">{kindLabel(a.kind)}</span>
          <span class="aname">{a.name}</span>
          <span class="apath" title={a.path}>{a.path}</span>
          <span class="asize">{formatBytes(a.size)}</span>
          <button class="reveal" title="Show in file manager" onclick={(e) => reveal(a.path, e)}>
            ⤴
          </button>
        </div>
      {/each}
    </div>
  {/if}

  {#if selectedCount > 0}
    <div class="selbar">
      {selectedCount} / {project.artifacts.length} selected · {formatBytes(selectedSize)}
    </div>
  {/if}
</div>

<style>
  .card {
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    transition: border-color 0.15s;
    flex-shrink: 0;
  }
  .card.active {
    border-color: var(--border-strong);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    cursor: pointer;
  }
  .head:hover {
    background: var(--bg-hover);
  }
  .info {
    flex: 1;
    min-width: 0;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .name {
    font-weight: 600;
    font-size: 15px;
  }
  .lang {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 20px;
    color: var(--c);
    background: color-mix(in srgb, var(--c) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--c) 35%, transparent);
    white-space: nowrap;
  }
  .path {
    font-size: 12px;
    color: var(--text-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 2px;
  }
  .meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    white-space: nowrap;
  }
  .count {
    font-size: 11px;
    color: var(--text-faint);
  }
  .size {
    font-weight: 700;
    font-size: 15px;
    font-variant-numeric: tabular-nums;
  }
  .chev {
    font-size: 22px;
    color: var(--text-faint);
    transition: transform 0.18s;
    width: 14px;
    text-align: center;
  }
  .chev.open {
    transform: rotate(90deg);
  }

  .artifacts {
    border-top: 1px solid var(--border);
    background: var(--bg);
  }
  .artifact {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 16px 9px 18px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }
  .artifact:last-child {
    border-bottom: none;
  }
  .artifact:hover {
    background: var(--bg-elev-2);
  }
  .artifact.sel {
    background: var(--accent-soft);
  }
  .kind {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 2px 7px;
    border-radius: 5px;
    color: var(--k);
    background: color-mix(in srgb, var(--k) 15%, transparent);
    width: 92px;
    text-align: center;
    flex-shrink: 0;
  }
  .aname {
    font-weight: 600;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    flex-shrink: 0;
  }
  .apath {
    flex: 1;
    min-width: 0;
    color: var(--text-faint);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl;
    text-align: left;
  }
  .asize {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--text-dim);
    flex-shrink: 0;
  }
  .reveal {
    color: var(--text-faint);
    font-size: 15px;
    padding: 2px 6px;
    border-radius: 5px;
    flex-shrink: 0;
  }
  .reveal:hover {
    color: var(--text);
    background: var(--bg-hover);
  }

  .selbar {
    padding: 7px 16px;
    font-size: 12px;
    color: var(--accent);
    background: var(--accent-soft);
    border-top: 1px solid var(--border);
    font-weight: 600;
  }

  /* custom checkbox */
  .checkbox {
    position: relative;
    display: inline-flex;
    flex-shrink: 0;
    cursor: pointer;
  }
  .checkbox input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }
  .box {
    width: 18px;
    height: 18px;
    border-radius: 5px;
    border: 1.5px solid var(--border-strong);
    background: var(--bg-elev-2);
    display: inline-block;
    transition: all 0.12s;
    position: relative;
  }
  .checkbox input:checked + .box {
    background: var(--accent);
    border-color: var(--accent);
  }
  .checkbox input:checked + .box::after {
    content: "";
    position: absolute;
    left: 5px;
    top: 1px;
    width: 5px;
    height: 10px;
    border: solid white;
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }
  .checkbox input:indeterminate + .box {
    background: var(--accent);
    border-color: var(--accent);
  }
  .checkbox input:indeterminate + .box::after {
    content: "";
    position: absolute;
    left: 3px;
    top: 7px;
    width: 10px;
    height: 2px;
    background: white;
  }
</style>
