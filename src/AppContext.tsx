import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getSettings, listProjects, listUnityEditors, updateSettings as saveSettings } from "./api";
import type { AppSettings, ProjectInfo, SettingsPatch, UnityEditorInfo } from "./api";
import { errorMessage } from "./format";
import { applyTheme, loadTheme, loadView, saveTheme, saveView } from "./theme";
import type { ThemeChoice, ViewMode } from "./theme";

export type Route =
	| { page: "projects" }
	| { page: "presets" }
	| { page: "backups" }
	| { page: "new" }
	| { page: "manage"; path: string }
	| { page: "learn" }
	| { page: "tools" }
	| { page: "logs" }
	| { page: "settings" };

export type NoticeKind = "info" | "success" | "warning" | "error";

export interface LogEntry {
	id: number;
	time: number;
	kind: NoticeKind;
	text: string;
}

export interface Toast {
	id: number;
	kind: NoticeKind;
	text: string;
}

export interface ImportProgress {
	done: number;
	total: number;
}

interface AppContextValue {
	route: Route;
	navigate: (route: Route) => void;
	projects: ProjectInfo[];
	projectsLoading: boolean;
	refreshProjects: () => Promise<void>;
	editors: UnityEditorInfo[];
	editorsLoading: boolean;
	editorsError: string | null;
	refreshEditors: () => Promise<void>;
	settings: AppSettings;
	updateSettings: (patch: SettingsPatch) => Promise<boolean>;
	packageOperation: boolean;
	setPackageOperation: (active: boolean) => void;
	packageImport: ImportProgress | null;
	setPackageImport: (progress: ImportProgress | null) => void;
	repositoriesVersion: number;
	bumpRepositories: () => void;
	notify: (kind: NoticeKind, text: string) => void;
	toasts: Toast[];
	dismissToast: (id: number) => void;
	log: LogEntry[];
	clearLog: () => void;
	theme: ThemeChoice;
	setTheme: (theme: ThemeChoice) => void;
	view: ViewMode;
	setView: (view: ViewMode) => void;
}

const AppContext = createContext<AppContextValue | null>(null);

export function useApp(): AppContextValue {
	const value = useContext(AppContext);
	if (!value) {
		throw new Error("useApp must be used inside AppProvider");
	}
	return value;
}

export function AppProvider({ children }: { children: ReactNode }) {
	const [route, setRoute] = useState<Route>({ page: "projects" });
	const [projects, setProjects] = useState<ProjectInfo[]>([]);
	const [projectsLoading, setProjectsLoading] = useState(true);
	const [editors, setEditors] = useState<UnityEditorInfo[]>([]);
	const [editorsLoading, setEditorsLoading] = useState(true);
	const [editorsError, setEditorsError] = useState<string | null>(null);
	const [toasts, setToasts] = useState<Toast[]>([]);
	const [log, setLog] = useState<LogEntry[]>([]);
	const [theme, setThemeState] = useState<ThemeChoice>(loadTheme);
	const [view, setViewState] = useState<ViewMode>(loadView);
	const [settings, setSettings] = useState<AppSettings>({
		defaultProjectLocation: null,
		backupLocation: null,
		showPrerelease: false,
		defaultUnityArgs: "",
	});
	const [packageOperation, setPackageOperation] = useState(false);
	const [packageImport, setPackageImport] = useState<ImportProgress | null>(null);
	const [repositoriesVersion, setRepositoriesVersion] = useState(0);
	const nextId = useRef(1);

	const dismissToast = useCallback((id: number) => {
		setToasts((current) => current.filter((toast) => toast.id !== id));
	}, []);

	const notify = useCallback(
		(kind: NoticeKind, text: string) => {
			const id = nextId.current++;
			setToasts((current) => [...current, { id, kind, text }]);
			setLog((current) => [{ id, time: Date.now(), kind, text }, ...current].slice(0, 200));
			window.setTimeout(() => dismissToast(id), kind === "error" ? 7000 : 4000);
		},
		[dismissToast],
	);

	const refreshProjects = useCallback(async () => {
		setProjectsLoading(true);
		try {
			setProjects(await listProjects());
		} catch (error) {
			notify("error", errorMessage(error));
		} finally {
			setProjectsLoading(false);
		}
	}, [notify]);

	const refreshEditors = useCallback(async () => {
		setEditorsLoading(true);
		try {
			setEditors(await listUnityEditors());
			setEditorsError(null);
		} catch (error) {
			setEditorsError(errorMessage(error));
		} finally {
			setEditorsLoading(false);
		}
	}, []);

	useEffect(() => {
		void refreshProjects();
		void refreshEditors();
		getSettings()
			.then(setSettings)
			.catch((error) => notify("error", errorMessage(error)));
	}, [refreshProjects, refreshEditors, notify]);

	const updateSettings = useCallback(
		async (patch: SettingsPatch) => {
			try {
				setSettings(await saveSettings(patch));
				return true;
			} catch (error) {
				notify("error", errorMessage(error));
				return false;
			}
		},
		[notify],
	);

	useEffect(() => {
		let unlisten: (() => void) | null = null;
		let cancelled = false;
		listen("close-blocked", () => notify("info", "Package changes are in progress. Close the app again once they finish."))
			.then((stop) => {
				if (cancelled) {
					stop();
				} else {
					unlisten = stop;
				}
			})
			.catch(() => undefined);
		return () => {
			cancelled = true;
			unlisten?.();
		};
	}, [notify]);

	useEffect(() => {
		applyTheme(theme);
		if (theme !== "system") {
			return;
		}
		const query = window.matchMedia("(prefers-color-scheme: light)");
		const listener = () => applyTheme("system");
		query.addEventListener("change", listener);
		return () => query.removeEventListener("change", listener);
	}, [theme]);

	const setTheme = useCallback((next: ThemeChoice) => {
		saveTheme(next);
		setThemeState(next);
	}, []);

	const setView = useCallback((next: ViewMode) => {
		saveView(next);
		setViewState(next);
	}, []);

	const value = useMemo<AppContextValue>(
		() => ({
			route,
			navigate: setRoute,
			projects,
			projectsLoading,
			refreshProjects,
			editors,
			editorsLoading,
			editorsError,
			refreshEditors,
			settings,
			updateSettings,
			packageOperation,
			setPackageOperation,
			packageImport,
			setPackageImport,
			repositoriesVersion,
			bumpRepositories: () => setRepositoriesVersion((value) => value + 1),
			notify,
			toasts,
			dismissToast,
			log,
			clearLog: () => setLog([]),
			theme,
			setTheme,
			view,
			setView,
		}),
		[
			route,
			projects,
			projectsLoading,
			refreshProjects,
			editors,
			editorsLoading,
			editorsError,
			refreshEditors,
			settings,
			updateSettings,
			packageOperation,
			packageImport,
			repositoriesVersion,
			notify,
			toasts,
			dismissToast,
			log,
			theme,
			setTheme,
			view,
			setView,
		],
	);

	return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}
