<script lang="ts">
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { formatTimestamp } from "../format";
  import { history } from "../stores/history.svelte";
</script>

<div class="list">
  {#if history.entries.length === 0}
    <p class="empty-state">Completed downloads will show up here.</p>
  {:else}
    <div class="toolbar">
      <button class="danger" onclick={() => history.clear()}>Clear history</button>
    </div>
    {#each history.entries as entry (entry.id + entry.completedAt)}
      <div class="card row">
        <div class="body">
          <span class="title" title={entry.title}>{entry.title}</span>
          <span class="dim sub">
            {formatTimestamp(entry.completedAt)}
            {#if entry.audioOnly}· audio{/if}
          </span>
        </div>
        {#if entry.filepath}
          <button class="icon" title="Open file" onclick={() => openPath(entry.filepath!)}>▶</button>
          <button class="icon" title="Show in folder" onclick={() => revealItemInDir(entry.filepath!)}>📁</button>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .toolbar {
    display: flex;
    justify-content: flex-end;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
  }

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .sub {
    font-size: 13px;
  }
</style>
