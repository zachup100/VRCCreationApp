import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { listProjects, listUnityEditors } from "./api";
import type { ProjectInfo, UnityEditorInfo } from "./api";
import { errorMessage } from "./format";
import { applyTheme, loadTheme, loadView, saveTheme, saveView } from "./theme";
import type { ThemeChoice, ViewMode } from "./theme";

export type Route =
	| { page: "projects" }
	| { page: "new" }
	| { page: "manage"; path: string }
	| { page: "learn" }
	| { page: "tools" }
	| { page: "logs" }
	| { page: "settings" };

export type NoticeKind = "info" | "success" | "error";

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
	}, [refreshProjects, refreshEditors]);

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
