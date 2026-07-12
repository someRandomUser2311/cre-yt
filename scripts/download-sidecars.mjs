#!/usr/bin/env node
// Fetches yt-dlp and ffmpeg binaries for a given Rust target triple and
// places them in src-tauri/binaries/ with the triple-suffixed names that
// Tauri's bundle.externalBin expects.
//
// Usage:
//   node scripts/download-sidecars.mjs                 # auto-detect host triple
//   node scripts/download-sidecars.mjs --triple x86_64-pc-windows-msvc

import { execFileSync } from "node:child_process";
import { createWriteStream } from "node:fs";
import { chmod, mkdir, mkdtemp, readdir, rename, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const BIN_DIR = join(ROOT, "src-tauri", "binaries");

const YTDLP_BASE = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const BTBN_BASE = "https://github.com/BtbN/FFmpeg-Builds/releases/latest/download";

// yt-dlp ships no native macOS arm64 binary; yt-dlp_macos (x86_64) runs under
// Rosetta 2. evermeet.cx ffmpeg is likewise x86_64-only, used for both mac triples.
const TARGETS = {
  "x86_64-unknown-linux-gnu": {
    ytdlp: `${YTDLP_BASE}/yt-dlp_linux`,
    ffmpeg: { url: `${BTBN_BASE}/ffmpeg-master-latest-linux64-gpl.tar.xz`, archived: true },
  },
  "x86_64-pc-windows-msvc": {
    exe: ".exe",
    ytdlp: `${YTDLP_BASE}/yt-dlp.exe`,
    ffmpeg: { url: `${BTBN_BASE}/ffmpeg-master-latest-win64-gpl.zip`, archived: true },
  },
  "x86_64-apple-darwin": {
    ytdlp: `${YTDLP_BASE}/yt-dlp_macos`,
    ffmpeg: { url: "https://evermeet.cx/ffmpeg/getrelease/zip", archived: true },
  },
  "aarch64-apple-darwin": {
    ytdlp: `${YTDLP_BASE}/yt-dlp_macos`,
    ffmpeg: { url: "https://evermeet.cx/ffmpeg/getrelease/zip", archived: true },
  },
};

function hostTriple() {
  const out = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  return /host: (\S+)/.exec(out)[1];
}

async function download(url, dest) {
  console.log(`  ↓ ${url}`);
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) throw new Error(`GET ${url} → HTTP ${res.status}`);
  await pipeline(Readable.fromWeb(res.body), createWriteStream(dest));
}

async function findFile(dir, name) {
  for (const entry of await readdir(dir, { withFileTypes: true, recursive: true })) {
    if (entry.isFile() && entry.name === name) return join(entry.parentPath ?? entry.path, entry.name);
  }
  throw new Error(`${name} not found in extracted archive at ${dir}`);
}

async function main() {
  const tripleArg = process.argv.indexOf("--triple");
  const triple = tripleArg !== -1 ? process.argv[tripleArg + 1] : hostTriple();
  const target = TARGETS[triple];
  if (!target) throw new Error(`Unsupported triple ${triple}. Known: ${Object.keys(TARGETS).join(", ")}`);
  const exe = target.exe ?? "";

  await mkdir(BIN_DIR, { recursive: true });
  console.log(`Fetching sidecars for ${triple} → ${BIN_DIR}`);

  const ytdlpDest = join(BIN_DIR, `yt-dlp-${triple}${exe}`);
  await download(target.ytdlp, ytdlpDest);
  await chmod(ytdlpDest, 0o755);

  const ffmpegDest = join(BIN_DIR, `ffmpeg-${triple}${exe}`);
  const work = await mkdtemp(join(tmpdir(), "sidecar-"));
  try {
    const archive = join(work, "ffmpeg-archive");
    await download(target.ffmpeg.url, archive);
    // tar handles .tar.xz everywhere; bsdtar (macOS, Windows runners) also handles .zip
    execFileSync("tar", ["-xf", archive, "-C", work]);
    const ffmpegBin = await findFile(work, `ffmpeg${exe}`);
    await rename(ffmpegBin, ffmpegDest).catch(async () => {
      // cross-device fallback
      const { copyFile } = await import("node:fs/promises");
      await copyFile(ffmpegBin, ffmpegDest);
    });
    await chmod(ffmpegDest, 0o755);
  } finally {
    await rm(work, { recursive: true, force: true });
  }

  for (const f of [ytdlpDest, ffmpegDest]) {
    const { size } = await stat(f);
    console.log(`  ✔ ${f} (${(size / 1024 / 1024).toFixed(1)} MB)`);
  }
}

main().catch((err) => {
  console.error(err.message ?? err);
  process.exit(1);
});
