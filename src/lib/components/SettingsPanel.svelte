<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../api";
  import { settings } from "../stores/settings.svelte";
  import FolderRow from "./FolderRow.svelte";

  let folder = $state(settings.defaultFolder);
  let ytdlpVersion = $state("checking…");
  let updating = $state(false);
  let updateMsg = $state("");

  $effect(() => {
    if (folder && folder !== settings.defaultFolder) {
      void settings.setDefaultFolder(folder);
    }
  });

  onMount(async () => {
    try {
      ytdlpVersion = await api.getYtdlpVersion();
    } catch (e) {
      ytdlpVersion = `unavailable (${e})`;
    }
  });

  async function update() {
    updating = true;
    updateMsg = "";
    try {
      const version = await api.updateYtdlp();
      ytdlpVersion = version;
      updateMsg = `Updated to ${version}`;
    } catch (e) {
      updateMsg = String(e);
    } finally {
      updating = false;
    }
  }
</script>

<div class="panel">
  <div class="card section">
    <h3>Downloads</h3>
    <FolderRow bind:folder label="Default folder" />
    <div class="setting-row">
      <span class="dim">Simultaneous downloads</span>
      <select
        value={settings.maxConcurrent}
        onchange={(e) => settings.setMaxConcurrent(Number(e.currentTarget.value))}
      >
        {#each [1, 2, 3, 4, 5] as n (n)}
          <option value={n}>{n}</option>
        {/each}
      </select>
    </div>
    <div class="setting-row">
      <span class="dim">Preferred audio format</span>
      <select
        value={settings.audioFormat}
        onchange={(e) => settings.setAudioFormat(e.currentTarget.value as "mp3" | "m4a")}
      >
        <option value="mp3">MP3</option>
        <option value="m4a">M4A</option>
      </select>
    </div>
  </div>

  <div class="card section">
    <h3>Download engine</h3>
    <div class="setting-row">
      <span class="dim">yt-dlp version</span>
      <span>{ytdlpVersion}</span>
      <button onclick={update} disabled={updating}>
        {updating ? "Updating…" : "Update yt-dlp"}
      </button>
    </div>
    {#if updateMsg}
      <p class:error-text={!updateMsg.startsWith("Updated")} class="dim msg">{updateMsg}</p>
    {/if}
    <p class="dim msg">
      Sites change frequently — if downloads suddenly fail, updating yt-dlp usually fixes it.
    </p>
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .setting-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .setting-row .dim:first-child {
    min-width: 190px;
  }

  .msg {
    margin: 0;
    font-size: 13px;
  }
</style>
