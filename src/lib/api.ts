import { invoke } from "@tauri-apps/api/core";
import type { DownloadRequest, JobSnapshot, MediaInfo } from "./types";

export const fetchVideoInfo = (url: string) =>
  invoke<MediaInfo>("fetch_video_info", { url });

export const startDownload = (request: DownloadRequest) =>
  invoke<string>("start_download", { request });

export const pauseDownload = (id: string) => invoke<void>("pause_download", { id });
export const resumeDownload = (id: string) => invoke<void>("resume_download", { id });
export const cancelDownload = (id: string) => invoke<void>("cancel_download", { id });
export const removeJob = (id: string) => invoke<void>("remove_job", { id });
export const getQueue = () => invoke<JobSnapshot[]>("get_queue");
export const setMaxConcurrent = (max: number) =>
  invoke<void>("set_max_concurrent", { max });

export const getYtdlpVersion = () => invoke<string>("get_ytdlp_version");
export const updateYtdlp = () => invoke<string>("update_ytdlp");
export const suggestDownloadDir = () => invoke<string>("suggest_download_dir");
