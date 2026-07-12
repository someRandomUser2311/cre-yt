import * as api from "../api";
import type {
  CompletePayload,
  DownloadRequest,
  ErrorPayload,
  JobStatus,
  ProgressPayload,
  QueueItem,
  StatusPayload,
} from "../types";

class QueueStore {
  items = $state<QueueItem[]>([]);

  get activeCount(): number {
    return this.items.filter((i) =>
      ["queued", "downloading", "processing"].includes(i.status),
    ).length;
  }

  add(id: string, request: DownloadRequest) {
    this.items.unshift({
      id,
      url: request.url,
      title: request.title ?? null,
      thumbnail: request.thumbnail ?? null,
      destDir: request.destDir,
      audioOnly: request.audioOnly ?? false,
      status: "queued",
      filepath: null,
      error: null,
      progress: null,
    });
  }

  private find(id: string): QueueItem | undefined {
    return this.items.find((i) => i.id === id);
  }

  applyStatus(p: StatusPayload) {
    const item = this.find(p.id);
    if (!item) {
      // job started outside this UI instance (e.g. dev reload) — resync
      void this.sync();
      return;
    }
    item.status = p.status;
    if (p.status !== "downloading" && p.status !== "processing") {
      item.progress = null;
    }
  }

  applyProgress(p: ProgressPayload) {
    const item = this.find(p.id);
    if (item) item.progress = p;
  }

  markComplete(p: CompletePayload) {
    const item = this.find(p.id);
    if (item) {
      item.status = "completed";
      item.filepath = p.filepath;
      item.progress = null;
    }
  }

  markError(p: ErrorPayload) {
    const item = this.find(p.id);
    if (item) {
      item.status = "error";
      item.error = p.message;
      item.progress = null;
    }
  }

  remove(id: string) {
    this.items = this.items.filter((i) => i.id !== id);
  }

  /** Pull the backend's job table (used at startup / after dev reloads). */
  async sync() {
    const jobs = await api.getQueue();
    const known = new Set(this.items.map((i) => i.id));
    for (const job of jobs) {
      if (known.has(job.id)) {
        const item = this.find(job.id)!;
        item.status = job.status;
        item.filepath = job.filepath;
        item.error = job.error;
      } else {
        this.items.push({ ...job, progress: null });
      }
    }
  }

  clearFinished() {
    const finished: JobStatus[] = ["completed", "cancelled", "error"];
    for (const item of this.items.filter((i) => finished.includes(i.status))) {
      void api.removeJob(item.id);
    }
    this.items = this.items.filter((i) => !finished.includes(i.status));
  }
}

export const queue = new QueueStore();
