<script lang="ts">
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import * as api from "../api";
  import { formatBytes, formatEta, formatSpeed } from "../format";
  import { queue } from "../stores/queue.svelte";
  import type { QueueItem } from "../types";

  let { item }: { item: QueueItem } = $props();

  const STATUS_LABEL: Record<string, string> = {
    queued: "Queued",
    downloading: "Downloading",
    processing: "Processing / merging",
    paused: "Paused",
    completed: "Completed",
    cancelled: "Cancelled",
    error: "Failed",
  };

  const active = $derived(item.status === "downloading" || item.status === "processing");
  const finished = $derived(["completed", "cancelled", "error"].includes(item.status));

  async function act(fn: () => Promise<void>) {
    try {
      await fn();
    } catch (e) {
      console.error(e);
    }
  }
</script>

<div class="card row">
  {#if item.thumbnail}
    <img src={item.thumbnail} alt="" class="thumb" />
  {/if}
  <div class="body">
    <div class="title-line">
      <span class="title" title={item.title ?? item.url}>{item.title ?? item.url}</span>
      {#if item.audioOnly}<span class="badge">audio</span>{/if}
      <span class="status status-{item.status}">{STATUS_LABEL[item.status]}</span>
    </div>

    {#if active}
      <div class="bar" class:indeterminate={item.status === "processing" || item.progress?.percent == null}>
        <div class="fill" style:width={`${item.progress?.percent ?? 100}%`}></div>
      </div>
      <div class="stats dim">
        {#if item.status === "downloading" && item.progress}
          <span>{formatBytes(item.progress.downloaded)} / {formatBytes(item.progress.total)}</span>
          <span>{formatSpeed(item.progress.speed)}</span>
          <span>ETA {formatEta(item.progress.eta)}</span>
        {:else if item.status === "processing"}
          <span>Merging / converting with ffmpeg…</span>
        {/if}
      </div>
    {:else if item.status === "error" && item.error}
      <p class="error-text msg">{item.error}</p>
    {:else if item.status === "completed" && item.filepath}
      <p class="dim msg" title={item.filepath}>{item.filepath}</p>
    {/if}
  </div>

  <div class="buttons">
    {#if item.status === "downloading" || item.status === "queued"}
      <button class="icon" title="Pause" onclick={() => act(() => api.pauseDownload(item.id))}>⏸</button>
    {/if}
    {#if item.status === "paused" || item.status === "error"}
      <button class="icon" title="Resume / retry" onclick={() => act(() => api.resumeDownload(item.id))}>▶</button>
    {/if}
    {#if item.status === "queued" || item.status === "downloading" || item.status === "paused"}
      <button class="icon danger" title="Cancel" onclick={() => act(() => api.cancelDownload(item.id))}>✕</button>
    {/if}
    {#if item.status === "completed" && item.filepath}
      <button class="icon" title="Open file" onclick={() => act(() => openPath(item.filepath!))}>▶</button>
      <button class="icon" title="Show in folder" onclick={() => act(() => revealItemInDir(item.filepath!))}>📁</button>
    {/if}
    {#if finished || item.status === "paused"}
      <button
        class="icon"
        title="Remove from list"
        onclick={() => act(async () => {
          await api.removeJob(item.id);
          queue.remove(item.id);
        })}>🗑</button>
    {/if}
  </div>
</div>

<style>
  .row {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .thumb {
    width: 90px;
    border-radius: 6px;
    object-fit: cover;
  }

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .title-line {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .badge {
    font-size: 12px;
    padding: 1px 7px;
    border-radius: 10px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    color: var(--text-dim);
  }

  .status {
    font-size: 13px;
    white-space: nowrap;
  }

  .status-downloading,
  .status-processing {
    color: var(--accent);
  }

  .status-completed {
    color: var(--green);
  }

  .status-error,
  .status-cancelled {
    color: var(--red);
  }

  .status-paused,
  .status-queued {
    color: var(--amber);
  }

  .bar {
    height: 6px;
    border-radius: 3px;
    background: var(--bg-hover);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
    transition: width 0.25s linear;
  }

  .indeterminate .fill {
    animation: slide 1.2s infinite ease-in-out;
    width: 40% !important;
  }

  @keyframes slide {
    0% {
      margin-left: -40%;
    }
    100% {
      margin-left: 100%;
    }
  }

  .stats {
    display: flex;
    gap: 16px;
    font-size: 13px;
  }

  .msg {
    margin: 0;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .buttons {
    display: flex;
    gap: 6px;
  }
</style>
