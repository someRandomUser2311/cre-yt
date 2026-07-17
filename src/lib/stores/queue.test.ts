import { describe, it, expect, beforeEach, vi } from "vitest";
import type { JobSnapshot } from "../types";

// The store calls into the Tauri IPC layer; stub it so tests stay in-process.
const removeJob = vi.fn(() => Promise.resolve());
const getQueue = vi.fn<() => Promise<JobSnapshot[]>>(() => Promise.resolve([]));
vi.mock("../api", () => ({
  removeJob: (id: string) => removeJob(id),
  getQueue: () => getQueue(),
}));

import { queue } from "./queue.svelte";

const req = {
  url: "https://example.com/v",
  destDir: "/tmp",
  title: "Clip",
  thumbnail: "https://example.com/t.jpg",
  audioOnly: false,
};

beforeEach(() => {
  queue.items = [];
  removeJob.mockClear();
  getQueue.mockClear();
});

describe("add", () => {
  it("prepends a queued item derived from the request", () => {
    queue.add("a", req);
    queue.add("b", req);
    expect(queue.items.map((i) => i.id)).toEqual(["b", "a"]);
    const item = queue.items[1];
    expect(item.status).toBe("queued");
    expect(item.title).toBe("Clip");
    expect(item.progress).toBeNull();
  });

  it("defaults missing optional fields", () => {
    queue.add("a", { url: "u", destDir: "/d" });
    expect(queue.items[0].title).toBeNull();
    expect(queue.items[0].thumbnail).toBeNull();
    expect(queue.items[0].audioOnly).toBe(false);
  });
});

describe("activeCount", () => {
  it("counts only queued/downloading/processing", () => {
    queue.add("a", req);
    queue.add("b", req);
    queue.add("c", req);
    queue.applyStatus({ id: "a", status: "downloading" });
    queue.applyStatus({ id: "b", status: "completed" });
    queue.applyStatus({ id: "c", status: "processing" });
    expect(queue.activeCount).toBe(2);
  });
});

describe("applyStatus", () => {
  it("updates status and clears progress on terminal states", () => {
    queue.add("a", req);
    queue.applyProgress({ id: "a", stage: "downloading", downloaded: 1, total: 2, percent: 50, speed: 1, eta: 1 });
    queue.applyStatus({ id: "a", status: "completed" });
    expect(queue.items[0].status).toBe("completed");
    expect(queue.items[0].progress).toBeNull();
  });

  it("keeps progress while still downloading", () => {
    queue.add("a", req);
    queue.applyProgress({ id: "a", stage: "downloading", downloaded: 1, total: 2, percent: 50, speed: 1, eta: 1 });
    queue.applyStatus({ id: "a", status: "downloading" });
    expect(queue.items[0].progress).not.toBeNull();
  });

  it("resyncs from the backend for an unknown id", () => {
    queue.applyStatus({ id: "ghost", status: "downloading" });
    expect(getQueue).toHaveBeenCalledOnce();
  });
});

describe("markComplete / markError", () => {
  it("marks completion with filepath", () => {
    queue.add("a", req);
    queue.markComplete({ id: "a", filepath: "/tmp/out.mp4" });
    expect(queue.items[0].status).toBe("completed");
    expect(queue.items[0].filepath).toBe("/tmp/out.mp4");
  });

  it("marks error with message", () => {
    queue.add("a", req);
    queue.markError({ id: "a", message: "boom" });
    expect(queue.items[0].status).toBe("error");
    expect(queue.items[0].error).toBe("boom");
  });

  it("ignores unknown ids without throwing", () => {
    expect(() => queue.markComplete({ id: "x", filepath: null })).not.toThrow();
    expect(() => queue.markError({ id: "x", message: "e" })).not.toThrow();
  });
});

describe("remove", () => {
  it("drops the matching item", () => {
    queue.add("a", req);
    queue.add("b", req);
    queue.remove("a");
    expect(queue.items.map((i) => i.id)).toEqual(["b"]);
  });
});

describe("sync", () => {
  it("adds unknown jobs and refreshes known ones", async () => {
    queue.add("a", req);
    getQueue.mockResolvedValueOnce([
      { id: "a", url: "u", title: null, thumbnail: null, destDir: "/d", audioOnly: false, status: "completed", filepath: "/f", error: null },
      { id: "z", url: "u", title: null, thumbnail: null, destDir: "/d", audioOnly: false, status: "downloading", filepath: null, error: null },
    ]);
    await queue.sync();
    expect(queue.items.find((i) => i.id === "a")!.status).toBe("completed");
    expect(queue.items.find((i) => i.id === "z")).toBeDefined();
  });
});

describe("clearFinished", () => {
  it("removes finished items and asks the backend to forget them", () => {
    queue.add("done", req);
    queue.add("busy", req);
    queue.applyStatus({ id: "done", status: "completed" });
    queue.applyStatus({ id: "busy", status: "downloading" });
    queue.clearFinished();
    expect(queue.items.map((i) => i.id)).toEqual(["busy"]);
    expect(removeJob).toHaveBeenCalledWith("done");
    expect(removeJob).toHaveBeenCalledTimes(1);
  });
});
