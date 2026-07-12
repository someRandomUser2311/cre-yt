# Video Downloader

Cross-platform desktop app (Tauri 2 + Svelte 5) for downloading videos from YouTube, Vimeo, and the 1000+ other sites supported by [yt-dlp](https://github.com/yt-dlp/yt-dlp).

Paste a URL → pick a quality/format (or audio-only MP3/M4A) → pick a folder → download. Includes a concurrent download queue with progress/pause/cancel, persistent history, playlist support with quality presets, and a one-click yt-dlp updater.

## How it works

- **Rust backend** spawns bundled `yt-dlp`/`ffmpeg` sidecar binaries and streams machine-readable progress back to the UI via Tauri events. All process spawning happens in Rust — the frontend can never run arbitrary commands.
- **yt-dlp self-update** downloads the latest release into the app data dir, which shadows the read-only bundled copy (site extractors break frequently; updating usually fixes failures).
- Settings and history persist as JSON via `tauri-plugin-store`.

## Development setup

1. Prerequisites: Rust (stable), Node 20+, pnpm.
   On Ubuntu/Debian, Tauri also needs:

   ```bash
   sudo apt install -y libwebkit2gtk-4.1-dev build-essential pkg-config curl wget file \
     libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
   ```

2. Install dependencies and fetch the sidecar binaries for your platform:

   ```bash
   pnpm install
   node scripts/download-sidecars.mjs
   ```

3. Run the app:

   ```bash
   pnpm tauri dev
   ```

## Building installers

```bash
pnpm tauri build
```

Releases for all platforms are built by `.github/workflows/release.yml` — push a `v*` tag to produce a draft GitHub Release with `.msi`/`.exe`, `.dmg`, `.deb`/`.rpm`/`.AppImage` artifacts.

Caveats:

- Windows/macOS builds are unsigned/un-notarized (SmartScreen / Gatekeeper warnings apply).
- The bundled ffmpeg comes from GPL builds ([BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds), [evermeet.cx](https://evermeet.cx/ffmpeg/)), which makes the distributed bundle GPL-encumbered.
- On Apple Silicon, yt-dlp runs under Rosetta 2 (no official native arm64 build).

## Layout

- `src-tauri/src/download.rs` — download queue, progress parsing, pause/cancel
- `src-tauri/src/info.rs` — URL → formats/playlist metadata (`yt-dlp -J`)
- `src-tauri/src/updater.rs` — yt-dlp self-update
- `src/lib/components/` — Svelte UI (format picker, playlist selector, queue, history, settings)
- `scripts/download-sidecars.mjs` — fetches per-platform yt-dlp/ffmpeg for dev and CI
