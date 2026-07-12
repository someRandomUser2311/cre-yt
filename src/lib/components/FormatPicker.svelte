<script lang="ts">
  import * as api from "../api";
  import { formatBytes, formatDuration } from "../format";
  import { queue } from "../stores/queue.svelte";
  import { settings } from "../stores/settings.svelte";
  import type { Format, MediaInfo } from "../types";
  import FolderRow from "./FolderRow.svelte";

  let {
    info,
    onClose,
    onStarted,
  }: { info: MediaInfo; onClose: () => void; onStarted: () => void } = $props();

  let folder = $state(settings.defaultFolder);
  let mode = $state<"video" | "audio">("video");
  let audioFormat = $state<"mp3" | "m4a">(settings.audioFormat);
  let selectedId = $state<string | null>(null);
  let starting = $state(false);
  let error = $state("");

  const videoFormats = $derived(
    info.formats
      .filter((f) => f.vcodec && f.height)
      .sort((a, b) => (b.height! - a.height!) || ((b.fps ?? 0) - (a.fps ?? 0)) || ((b.tbr ?? 0) - (a.tbr ?? 0))),
  );

  function qualityLabel(f: Format): string {
    const fps = f.fps && f.fps > 30 ? Math.round(f.fps) : "";
    return `${f.height}p${fps}`;
  }

  function sizeOf(f: Format): string {
    return formatBytes(f.filesize ?? f.filesizeApprox);
  }

  function selectorFor(f: Format): string {
    // video-only streams need an audio track merged in
    return f.acodec ? f.formatId : `${f.formatId}+bestaudio/best`;
  }

  async function download() {
    if (starting) return;
    if (!folder) {
      error = "Choose a destination folder first";
      return;
    }
    starting = true;
    error = "";
    try {
      if (folder !== settings.defaultFolder) await settings.setDefaultFolder(folder);
      if (audioFormat !== settings.audioFormat) await settings.setAudioFormat(audioFormat);
      const chosen = videoFormats.find((f) => f.formatId === selectedId);
      const request = {
        url: info.webpageUrl,
        destDir: folder,
        title: info.title,
        thumbnail: info.thumbnail ?? undefined,
        audioOnly: mode === "audio",
        audioFormat: mode === "audio" ? audioFormat : undefined,
        formatSelector:
          mode === "video" && chosen ? selectorFor(chosen) : undefined,
      };
      const id = await api.startDownload(request);
      queue.add(id, request);
      onStarted();
    } catch (e) {
      error = String(e);
    } finally {
      starting = false;
    }
  }
</script>

<div class="card picker">
  <div class="header">
    {#if info.thumbnail}
      <img src={info.thumbnail} alt="" class="thumb" />
    {/if}
    <div class="meta">
      <h3>{info.title}</h3>
      <p class="dim">
        {info.uploader ?? ""}
        {#if info.duration}
          · {formatDuration(info.duration)}
        {/if}
      </p>
    </div>
    <button class="icon" onclick={onClose} title="Close">✕</button>
  </div>

  <div class="mode-tabs">
    <button class:primary={mode === "video"} onclick={() => (mode = "video")}>Video</button>
    <button class:primary={mode === "audio"} onclick={() => (mode = "audio")}>Audio only</button>
  </div>

  {#if mode === "video"}
    <div class="formats">
      <label class="format-row">
        <input type="radio" name="fmt" checked={selectedId === null} onchange={() => (selectedId = null)} />
        <span class="quality">Best available</span>
        <span class="dim">automatically picks the highest quality</span>
      </label>
      {#each videoFormats as f (f.formatId)}
        <label class="format-row">
          <input
            type="radio"
            name="fmt"
            checked={selectedId === f.formatId}
            onchange={() => (selectedId = f.formatId)}
          />
          <span class="quality">{qualityLabel(f)}</span>
          <span>{f.ext}</span>
          <span class="dim codec">{f.vcodec?.split(".")[0]}{f.acodec ? " + audio" : ""}</span>
          <span class="size">{sizeOf(f)}</span>
        </label>
      {:else}
        <p class="dim">No video formats reported — "Best available" will still work for most sites.</p>
      {/each}
    </div>
  {:else}
    <div class="audio-choice">
      <label><input type="radio" name="afmt" checked={audioFormat === "mp3"} onchange={() => (audioFormat = "mp3")} /> MP3</label>
      <label><input type="radio" name="afmt" checked={audioFormat === "m4a"} onchange={() => (audioFormat = "m4a")} /> M4A</label>
      <span class="dim">Best audio track, converted with ffmpeg</span>
    </div>
  {/if}

  <FolderRow bind:folder />

  {#if error}
    <p class="error-text">{error}</p>
  {/if}

  <div class="actions">
    <button class="primary" onclick={download} disabled={starting || !folder}>
      {starting ? "Starting…" : "Download"}
    </button>
  </div>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .header {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .thumb {
    width: 120px;
    border-radius: 8px;
    object-fit: cover;
  }

  .meta {
    flex: 1;
    min-width: 0;
  }

  .meta h3 {
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .mode-tabs {
    display: flex;
    gap: 8px;
  }

  .formats {
    display: flex;
    flex-direction: column;
    max-height: 280px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .format-row {
    display: grid;
    grid-template-columns: 24px 90px 50px 1fr 90px;
    gap: 10px;
    align-items: center;
    padding: 8px 12px;
    cursor: pointer;
  }

  .format-row:hover {
    background: var(--bg-hover);
  }

  .format-row + .format-row {
    border-top: 1px solid var(--border);
  }

  .quality {
    font-weight: 600;
  }

  .size {
    text-align: right;
  }

  .codec {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .audio-choice {
    display: flex;
    gap: 18px;
    align-items: center;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
