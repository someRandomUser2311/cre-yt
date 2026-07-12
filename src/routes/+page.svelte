<script lang="ts">
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import FormatPicker from "$lib/components/FormatPicker.svelte";
  import HistoryList from "$lib/components/HistoryList.svelte";
  import PlaylistSelector from "$lib/components/PlaylistSelector.svelte";
  import QueueList from "$lib/components/QueueList.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import UrlInput from "$lib/components/UrlInput.svelte";
  import { history } from "$lib/stores/history.svelte";
  import { queue } from "$lib/stores/queue.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import type {
    CompletePayload,
    ErrorPayload,
    MediaInfo,
    ProgressPayload,
    StatusPayload,
  } from "$lib/types";

  type Tab = "download" | "queue" | "history" | "settings";
  let tab = $state<Tab>("download");
  let info = $state<MediaInfo | null>(null);

  onMount(() => {
    const unlisteners: UnlistenFn[] = [];
    (async () => {
      await settings.init();
      await history.init();
      await queue.sync();
      unlisteners.push(
        await listen<StatusPayload>("download://status", (e) => queue.applyStatus(e.payload)),
        await listen<ProgressPayload>("download://progress", (e) => queue.applyProgress(e.payload)),
        await listen<CompletePayload>("download://complete", (e) => {
          queue.markComplete(e.payload);
          void history.reload();
        }),
        await listen<ErrorPayload>("download://error", (e) => queue.markError(e.payload)),
      );
    })();
    return () => unlisteners.forEach((u) => u());
  });

  function onStarted() {
    info = null;
    tab = "queue";
  }
</script>

<main>
  <nav>
    <button class:active={tab === "download"} onclick={() => (tab = "download")}>Download</button>
    <button class:active={tab === "queue"} onclick={() => (tab = "queue")}>
      Queue
      {#if queue.activeCount > 0}<span class="count">{queue.activeCount}</span>{/if}
    </button>
    <button class:active={tab === "history"} onclick={() => (tab = "history")}>History</button>
    <button class:active={tab === "settings"} onclick={() => (tab = "settings")}>Settings</button>
  </nav>

  <section>
    {#if tab === "download"}
      <div class="download-view">
        <UrlInput onResult={(result) => (info = result)} />
        {#if info}
          {#if info.isPlaylist}
            <PlaylistSelector {info} onClose={() => (info = null)} {onStarted} />
          {:else}
            <FormatPicker {info} onClose={() => (info = null)} {onStarted} />
          {/if}
        {/if}
      </div>
    {:else if tab === "queue"}
      <QueueList />
    {:else if tab === "history"}
      <HistoryList />
    {:else}
      <SettingsPanel />
    {/if}
  </section>
</main>

<style>
  main {
    max-width: 860px;
    margin: 0 auto;
    padding: 20px 24px 40px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  nav {
    display: flex;
    gap: 6px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 10px;
  }

  nav button {
    background: transparent;
    border: none;
    color: var(--text-dim);
    padding: 8px 14px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    gap: 7px;
  }

  nav button:hover {
    color: var(--text);
    background: var(--bg-raised);
  }

  nav button.active {
    color: var(--text);
    background: var(--bg-raised);
    font-weight: 600;
  }

  .count {
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    min-width: 19px;
    height: 19px;
    border-radius: 10px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 5px;
  }

  .download-view {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
</style>
