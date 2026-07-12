<script lang="ts">
  import * as api from "../api";
  import { formatDuration } from "../format";
  import { queue } from "../stores/queue.svelte";
  import { settings, type PlaylistPreset } from "../stores/settings.svelte";
  import type { MediaInfo } from "../types";
  import FolderRow from "./FolderRow.svelte";

  let {
    info,
    onClose,
    onStarted,
  }: { info: MediaInfo; onClose: () => void; onStarted: () => void } = $props();

  const entries = $derived(info.entries.filter((e) => e.url));

  let folder = $state(settings.defaultFolder);
  let preset = $state<PlaylistPreset>(settings.playlistPreset);
  let checked = $state<boolean[]>([]);
  let starting = $state(false);
  let error = $state("");

  $effect(() => {
    if (checked.length !== entries.length) checked = entries.map(() => true);
  });

  const selectedCount = $derived(checked.filter(Boolean).length);

  const PRESETS: Record<PlaylistPreset, { label: string; selector?: string; audio?: boolean }> = {
    best: { label: "Best quality", selector: "bestvideo+bestaudio/best" },
    "1080p": { label: "1080p", selector: "bestvideo[height<=1080]+bestaudio/best[height<=1080]" },
    "720p": { label: "720p", selector: "bestvideo[height<=720]+bestaudio/best[height<=720]" },
    "480p": { label: "480p", selector: "bestvideo[height<=480]+bestaudio/best[height<=480]" },
    audio: { label: "Audio only (MP3)", audio: true },
  };

  function setAll(value: boolean) {
    checked = entries.map(() => value);
  }

  async function download() {
    if (starting || selectedCount === 0) return;
    if (!folder) {
      error = "Choose a destination folder first";
      return;
    }
    starting = true;
    error = "";
    try {
      if (folder !== settings.defaultFolder) await settings.setDefaultFolder(folder);
      if (preset !== settings.playlistPreset) await settings.setPlaylistPreset(preset);
      const p = PRESETS[preset];
      for (let i = 0; i < entries.length; i++) {
        if (!checked[i]) continue;
        const entry = entries[i];
        const request = {
          url: entry.url!,
          destDir: folder,
          title: entry.title ?? entry.url!,
          audioOnly: p.audio ?? false,
          audioFormat: p.audio ? ("mp3" as const) : undefined,
          formatSelector: p.selector,
        };
        const id = await api.startDownload(request);
        queue.add(id, request);
      }
      onStarted();
    } catch (e) {
      error = String(e);
    } finally {
      starting = false;
    }
  }
</script>

<div class="card selector">
  <div class="header">
    <div>
      <h3>{info.title}</h3>
      <p class="dim">Playlist · {entries.length} videos</p>
    </div>
    <button class="icon" onclick={onClose} title="Close">✕</button>
  </div>

  <div class="controls">
    <button onclick={() => setAll(true)}>Select all</button>
    <button onclick={() => setAll(false)}>Select none</button>
    <span class="spacer"></span>
    <label class="dim" for="preset">Quality</label>
    <select id="preset" bind:value={preset}>
      {#each Object.entries(PRESETS) as [key, p] (key)}
        <option value={key}>{p.label}</option>
      {/each}
    </select>
  </div>

  <div class="entries">
    {#each entries as entry, i (entry.url)}
      <label class="entry-row">
        <input type="checkbox" bind:checked={checked[i]} />
        <span class="index dim">{i + 1}.</span>
        <span class="title">{entry.title ?? entry.url}</span>
        <span class="dim">{formatDuration(entry.duration)}</span>
      </label>
    {/each}
  </div>

  {#if error}
    <p class="error-text">{error}</p>
  {/if}

  <FolderRow bind:folder />

  <div class="actions">
    <button class="primary" onclick={download} disabled={starting || selectedCount === 0 || !folder}>
      {starting ? "Starting…" : `Download ${selectedCount} video${selectedCount === 1 ? "" : "s"}`}
    </button>
  </div>
</div>

<style>
  .selector {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .spacer {
    flex: 1;
  }

  .entries {
    display: flex;
    flex-direction: column;
    max-height: 300px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .entry-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 12px;
    cursor: pointer;
  }

  .entry-row:hover {
    background: var(--bg-hover);
  }

  .entry-row + .entry-row {
    border-top: 1px solid var(--border);
  }

  .index {
    min-width: 28px;
    text-align: right;
  }

  .title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
