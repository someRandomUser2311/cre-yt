import { describe, it, expect } from "vitest";
import {
  formatBytes,
  formatSpeed,
  formatEta,
  formatDuration,
  formatTimestamp,
} from "./format";

describe("formatBytes", () => {
  it("returns em dash for missing or non-positive input", () => {
    expect(formatBytes(null)).toBe("—");
    expect(formatBytes(undefined)).toBe("—");
    expect(formatBytes(0)).toBe("—");
    expect(formatBytes(-10)).toBe("—");
  });

  it("formats bytes with no decimals", () => {
    expect(formatBytes(512)).toBe("512 B");
  });

  it("scales up through units", () => {
    expect(formatBytes(1024)).toBe("1.0 KB");
    expect(formatBytes(1024 * 1024)).toBe("1.0 MB");
    expect(formatBytes(1024 * 1024 * 1024)).toBe("1.0 GB");
  });

  it("drops decimals once the value is >= 100", () => {
    expect(formatBytes(150 * 1024)).toBe("150 KB");
    expect(formatBytes(1.5 * 1024 * 1024)).toBe("1.5 MB");
  });

  it("caps at the largest unit (GB)", () => {
    expect(formatBytes(5 * 1024 * 1024 * 1024 * 1024)).toBe("5120 GB");
  });
});

describe("formatSpeed", () => {
  it("returns em dash for missing or non-positive input", () => {
    expect(formatSpeed(null)).toBe("—");
    expect(formatSpeed(0)).toBe("—");
  });

  it("appends /s to a byte figure", () => {
    expect(formatSpeed(1024)).toBe("1.0 KB/s");
  });
});

describe("formatEta", () => {
  it("returns em dash for missing or negative input", () => {
    expect(formatEta(null)).toBe("—");
    expect(formatEta(undefined)).toBe("—");
    expect(formatEta(-1)).toBe("—");
  });

  it("formats sub-minute durations in seconds", () => {
    expect(formatEta(0)).toBe("0s");
    expect(formatEta(45)).toBe("45s");
    expect(formatEta(59.4)).toBe("59s");
  });

  it("formats minutes and seconds", () => {
    expect(formatEta(60)).toBe("1m 0s");
    expect(formatEta(125)).toBe("2m 5s");
  });

  it("formats hours and minutes", () => {
    expect(formatEta(3600)).toBe("1h 0m");
    expect(formatEta(3661)).toBe("1h 1m");
  });
});

describe("formatDuration", () => {
  it("returns empty string for null/undefined", () => {
    expect(formatDuration(null)).toBe("");
    expect(formatDuration(undefined)).toBe("");
  });

  it("formats mm:ss when under an hour", () => {
    expect(formatDuration(0)).toBe("0:00");
    expect(formatDuration(5)).toBe("0:05");
    expect(formatDuration(65)).toBe("1:05");
    expect(formatDuration(600)).toBe("10:00");
  });

  it("formats h:mm:ss and zero-pads minutes when over an hour", () => {
    expect(formatDuration(3600)).toBe("1:00:00");
    expect(formatDuration(3665)).toBe("1:01:05");
    expect(formatDuration(7325)).toBe("2:02:05");
  });
});

describe("formatTimestamp", () => {
  it("converts unix seconds to a locale string for the right instant", () => {
    const unix = 1_700_000_000;
    expect(formatTimestamp(unix)).toBe(new Date(unix * 1000).toLocaleString());
  });
});
