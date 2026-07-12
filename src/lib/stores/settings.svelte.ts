import { load, type Store } from "@tauri-apps/plugin-store";
import * as api from "../api";

export type PlaylistPreset = "best" | "1080p" | "720p" | "480p" | "audio";

class SettingsStore {
  defaultFolder = $state("");
  maxConcurrent = $state(2);
  audioFormat = $state<"mp3" | "m4a">("mp3");
  playlistPreset = $state<PlaylistPreset>("1080p");
  loaded = $state(false);

  #store: Store | null = null;

  async init() {
    if (this.loaded) return;
    this.#store = await load("settings.json", { autoSave: true, defaults: {} });
    this.defaultFolder = (await this.#store.get<string>("defaultFolder")) ?? "";
    this.maxConcurrent = (await this.#store.get<number>("maxConcurrent")) ?? 2;
    this.audioFormat = (await this.#store.get<"mp3" | "m4a">("audioFormat")) ?? "mp3";
    this.playlistPreset =
      (await this.#store.get<PlaylistPreset>("playlistPreset")) ?? "1080p";

    if (!this.defaultFolder) {
      try {
        this.defaultFolder = await api.suggestDownloadDir();
        await this.#store.set("defaultFolder", this.defaultFolder);
      } catch {
        // leave empty; the format picker forces a choice
      }
    }
    await api.setMaxConcurrent(this.maxConcurrent);
    this.loaded = true;
  }

  async setDefaultFolder(folder: string) {
    this.defaultFolder = folder;
    await this.#store?.set("defaultFolder", folder);
  }

  async setMaxConcurrent(n: number) {
    this.maxConcurrent = n;
    await this.#store?.set("maxConcurrent", n);
    await api.setMaxConcurrent(n);
  }

  async setAudioFormat(fmt: "mp3" | "m4a") {
    this.audioFormat = fmt;
    await this.#store?.set("audioFormat", fmt);
  }

  async setPlaylistPreset(preset: PlaylistPreset) {
    this.playlistPreset = preset;
    await this.#store?.set("playlistPreset", preset);
  }
}

export const settings = new SettingsStore();
