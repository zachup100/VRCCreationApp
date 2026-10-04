export type ThemeChoice = "dark" | "light" | "system";
export type ViewMode = "list" | "grid";

const THEME_KEY = "theme";
const VIEW_KEY = "projectView";

function read(key: string): string | null {
	try {
		return localStorage.getItem(key);
	} catch {
		return null;
	}
}

function write(key: string, value: string) {
	try {
		localStorage.setItem(key, value);
	} catch {
		return;
	}
}

export function loadTheme(): ThemeChoice {
	const stored = read(THEME_KEY);
	return stored === "dark" || stored === "light" || stored === "system" ? stored : "dark";
}

export function saveTheme(choice: ThemeChoice) {
	write(THEME_KEY, choice);
}

export function loadView(): ViewMode {
	return read(VIEW_KEY) === "grid" ? "grid" : "list";
}

export function saveView(view: ViewMode) {
	write(VIEW_KEY, view);
}

export function applyTheme(choice: ThemeChoice) {
	const resolved =
		choice === "system"
			? window.matchMedia("(prefers-color-scheme: light)").matches
				? "light"
				: "dark"
			: choice;
	document.documentElement.dataset.theme = resolved;
}

const LOCATION_KEY = "lastProjectLocation";

export function loadLastLocation(): string {
	return read(LOCATION_KEY) ?? "";
}

export function saveLastLocation(location: string) {
	write(LOCATION_KEY, location);
}
