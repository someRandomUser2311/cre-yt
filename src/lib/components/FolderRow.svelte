<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";

  let {
    folder = $bindable(),
    label = "Save to",
  }: { folder: string; label?: string } = $props();

  async function choose() {
    const picked = await open({ directory: true, defaultPath: folder || undefined });
    if (typeof picked === "string") folder = picked;
  }
</script>

<div class="folder-row">
  <span class="dim">{label}</span>
  <span class="path" title={folder}>{folder || "No folder selected"}</span>
  <button onclick={choose}>Change…</button>
</div>

<style>
  .folder-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .path {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
</style>
