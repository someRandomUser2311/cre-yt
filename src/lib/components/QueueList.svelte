<script lang="ts">
  import { queue } from "../stores/queue.svelte";
  import QueueItemRow from "./QueueItemRow.svelte";

  const hasFinished = $derived(
    queue.items.some((i) => ["completed", "cancelled", "error"].includes(i.status)),
  );
</script>

<div class="list">
  {#if queue.items.length === 0}
    <p class="empty-state">No downloads yet. Paste a URL in the Download tab to get started.</p>
  {:else}
    {#if hasFinished}
      <div class="toolbar">
        <button onclick={() => queue.clearFinished()}>Clear finished</button>
      </div>
    {/if}
    {#each queue.items as item (item.id)}
      <QueueItemRow {item} />
    {/each}
  {/if}
</div>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .toolbar {
    display: flex;
    justify-content: flex-end;
  }
</style>
