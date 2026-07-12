import { load, type Store } from "@tauri-apps/plugin-store";
import type { HistoryEntry } from "../types";

class HistoryStore {
  entries = $state<HistoryEntry[]>([]);

  #store: Store | null = null;

  async init() {
    // Rust writes entries to this same store on download completion; the
    // plugin shares one instance per file, so reload() sees Rust's writes.
    this.#store = await load("history.json");
    await this.reload();
  }

  async reload() {
    if (!this.#store) return;
    this.entries = (await this.#store.get<HistoryEntry[]>("entries")) ?? [];
  }

  async clear() {
    this.entries = [];
    await this.#store?.set("entries", []);
    await this.#store?.save();
  }
}

export const history = new HistoryStore();
