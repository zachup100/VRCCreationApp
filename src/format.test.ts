import { describe, expect, it } from "vitest";
import { errorMessage, formatBytes, timeAgo } from "./format";

const NOW = Date.UTC(2026, 9, 3, 12, 0, 0);
const MINUTE = 60 * 1000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

describe("timeAgo", () => {
	it("says just now for very recent times", () => {
		expect(timeAgo(NOW - 20 * 1000, NOW)).toBe("just now");
		expect(timeAgo(NOW, NOW)).toBe("just now");
	});

	it("uses minutes, hours and days", () => {
		expect(timeAgo(NOW - 5 * MINUTE, NOW)).toBe("5 minutes ago");
		expect(timeAgo(NOW - 3 * HOUR, NOW)).toBe("3 hours ago");
		expect(timeAgo(NOW - 2 * DAY, NOW)).toBe("2 days ago");
	});

	it("uses weeks, months and years for older times", () => {
		expect(timeAgo(NOW - 14 * DAY, NOW)).toBe("2 weeks ago");
		expect(timeAgo(NOW - 60 * DAY, NOW)).toBe("2 months ago");
		expect(timeAgo(NOW - 800 * DAY, NOW)).toBe("2 years ago");
	});

	it("prefers natural words for one unit", () => {
		expect(timeAgo(NOW - DAY, NOW)).toBe("yesterday");
	});
});

describe("formatBytes", () => {
	it("shows kilobytes below one megabyte, never zero", () => {
		expect(formatBytes(1)).toBe("1 KB");
		expect(formatBytes(2048)).toBe("2 KB");
	});

	it("shows megabytes and gigabytes with decimals", () => {
		expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
		expect(formatBytes(1.5 * 1024 * 1024)).toBe("1.5 MB");
		expect(formatBytes(2 * 1024 * 1024 * 1024)).toBe("2.00 GB");
	});
});

describe("errorMessage", () => {
	it("returns strings as they are", () => {
		expect(errorMessage("plain message")).toBe("plain message");
	});

	it("reads the message of an Error", () => {
		expect(errorMessage(new Error("boom"))).toBe("boom");
	});

	it("falls back to a string for anything else", () => {
		expect(errorMessage(42)).toBe("42");
		expect(errorMessage({ toString: () => "custom" })).toBe("custom");
	});
});
