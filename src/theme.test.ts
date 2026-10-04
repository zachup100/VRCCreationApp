import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { applyTheme, loadLastLocation, loadTheme, loadView, saveLastLocation, saveTheme, saveView } from "./theme";

beforeEach(() => {
	localStorage.clear();
	document.documentElement.removeAttribute("data-theme");
});

afterEach(() => {
	vi.restoreAllMocks();
});

describe("saved preferences", () => {
	it("defaults to dark, list view and no location", () => {
		expect(loadTheme()).toBe("dark");
		expect(loadView()).toBe("list");
		expect(loadLastLocation()).toBe("");
	});

	it("restores what was saved", () => {
		saveTheme("oled");
		saveView("grid");
		saveLastLocation("/home/user/Projects");
		expect(loadTheme()).toBe("oled");
		expect(loadView()).toBe("grid");
		expect(loadLastLocation()).toBe("/home/user/Projects");
	});

	it("accepts every known theme and ignores unknown values", () => {
		for (const theme of ["dark", "oled", "light", "system"] as const) {
			saveTheme(theme);
			expect(loadTheme()).toBe(theme);
		}
		localStorage.setItem("theme", "neon");
		expect(loadTheme()).toBe("dark");
		localStorage.setItem("projectView", "table");
		expect(loadView()).toBe("list");
	});

	it("still works when browser storage throws", () => {
		vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
			throw new Error("blocked");
		});
		vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
			throw new Error("blocked");
		});
		expect(loadTheme()).toBe("dark");
		expect(loadView()).toBe("list");
		expect(() => saveTheme("light")).not.toThrow();
		expect(() => saveLastLocation("/x")).not.toThrow();
	});
});

describe("applyTheme", () => {
	it("sets the theme on the document element", () => {
		applyTheme("oled");
		expect(document.documentElement.dataset.theme).toBe("oled");
		applyTheme("light");
		expect(document.documentElement.dataset.theme).toBe("light");
		applyTheme("dark");
		expect(document.documentElement.dataset.theme).toBe("dark");
	});

	it("follows the system preference for system", () => {
		const setPreference = (light: boolean) =>
			Object.defineProperty(window, "matchMedia", {
				configurable: true,
				writable: true,
				value: (query: string) => ({ matches: light && query.includes("light"), media: query }),
			});
		setPreference(true);
		applyTheme("system");
		expect(document.documentElement.dataset.theme).toBe("light");
		setPreference(false);
		applyTheme("system");
		expect(document.documentElement.dataset.theme).toBe("dark");
	});
});
