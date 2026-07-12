<script lang="ts">
  import * as api from "../api";
  import type { MediaInfo } from "../types";

  let { onResult }: { onResult: (info: MediaInfo) => void } = $props();

  let url = $state("");
  let loading = $state(false);
  let error = $state("");

  async function fetchInfo() {
    const trimmed = url.trim();
    if (!trimmed || loading) return;
    loading = true;
    error = "";
    try {
      onResult(await api.fetchVideoInfo(trimmed));
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function pasteFromClipboard() {
    try {
      const text = await navigator.clipboard.readText();
      if (text?.trim().startsWith("http")) {
        url = text.trim();
        await fetchInfo();
      }
    } catch {
      // clipboard unavailable — ignore
    }
  }
</script>

<div class="url-row">
  <input
    type="url"
    placeholder="Paste a video or playlist URL (YouTube, Vimeo, …)"
    bind:value={url}
    onkeydown={(e) => e.key === "Enter" && fetchInfo()}
    disabled={loading}
  />
  <button onclick={pasteFromClipboard} disabled={loading} title="Paste from clipboard">📋 Paste</button>
  <button class="primary" onclick={fetchInfo} disabled={loading || !url.trim()}>
    {loading ? "Fetching…" : "Fetch"}
  </button>
</div>

{#if loading}
  <p class="dim">Looking up available formats — this can take a few seconds…</p>
{/if}
{#if error}
  <p class="error-text">
    {error}
    {#if error.includes("Unsupported") || error.toLowerCase().includes("unable to")}
      <br /><span class="dim">If this site used to work, try updating yt-dlp in Settings.</span>
    {/if}
  </p>
{/if}

<style>
  .url-row {
    display: flex;
    gap: 10px;
  }

  .url-row input {
    flex: 1;
  }
</style>
