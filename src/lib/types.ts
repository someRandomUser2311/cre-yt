export interface Format {
  formatId: string;
  ext: string;
  resolution: string | null;
  height: number | null;
  fps: number | null;
  vcodec: string | null;
  acodec: string | null;
  filesize: number | null;
  filesizeApprox: number | null;
  tbr: number | null;
  formatNote: string | null;
}

export interface PlaylistEntry {
  id: string | null;
  title: string | null;
  url: string | null;
  duration: number | null;
}

export interface MediaInfo {
  isPlaylist: boolean;
  title: string;
  webpageUrl: string;
  thumbnail: string | null;
  duration: number | null;
  uploader: string | null;
  formats: Format[];
  entries: PlaylistEntry[];
}

export type JobStatus =
  | "queued"
  | "downloading"
  | "processing"
  | "paused"
  | "completed"
  | "cancelled"
  | "error";

export interface DownloadRequest {
  url: string;
  destDir: string;
  formatSelector?: string;
  audioOnly?: boolean;
  audioFormat?: "mp3" | "m4a";
  title?: string;
  thumbnail?: string;
}

export interface JobSnapshot {
  id: string;
  url: string;
  title: string | null;
  thumbnail: string | null;
  destDir: string;
  audioOnly: boolean;
  status: JobStatus;
  filepath: string | null;
  error: string | null;
}

export interface ProgressPayload {
  id: string;
  stage: "downloading" | "processing";
  downloaded: number | null;
  total: number | null;
  percent: number | null;
  speed: number | null;
  eta: number | null;
}

export interface StatusPayload {
  id: string;
  status: JobStatus;
}

export interface CompletePayload {
  id: string;
  filepath: string | null;
}

export interface ErrorPayload {
  id: string;
  message: string;
}

export interface HistoryEntry {
  id: string;
  url: string;
  title: string;
  filepath: string | null;
  audioOnly: boolean;
  completedAt: number;
}

export interface QueueItem extends JobSnapshot {
  progress: ProgressPayload | null;
}
