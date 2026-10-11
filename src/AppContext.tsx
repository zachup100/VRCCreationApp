import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { checkForUpdate, deepLinkPendingCount, deepLinkTakeAddRepository, getSettings, listProjects, listUnityEditors, openExternal, refreshProjectInfo, updateSettings as saveSettings } from "./api";
import type { AppSettings, IncomingRepository, ProjectInfo, SettingsPatch, UnityEditorInfo, UnityInstall, UpdateStatus } from "./api";
import { errorMessage } from "./format";
import { applyTheme, loadTheme, loadView, saveTheme, saveView } from "./theme";
import type { ThemeChoice, ViewMode } from "./theme";

export type Route =
	| { page: "projects" }
	| { page: "packages" }
	| { page: "presets" }
	| { page: "backups" }
	| { page: "new" }
	| { page: "manage"; path: string }
	| { page: "unity" }
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

export interface ToastAction {
	label: string;
	run: () => void;
}

export interface NotifyOptions {
	action?: ToastAction;
	sticky?: boolean;
}

export interface Toast {
	id: number;
	kind: NoticeKind;
	text: string;
	action?: ToastAction;
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
	rescanProjects: () => Promise<void>;
	editors: UnityEditorInfo[];
	editorsLoading: boolean;
	editorsError: string | null;
	refreshEditors: () => Promise<void>;
	settings: AppSettings;
	updateSettings: (patch: SettingsPatch) => Promise<boolean>;
	unityInstall: UnityInstall | null;
	update: UpdateStatus | null;
	checkUpdates: (manual: boolean) => Promise<void>;
	packageOperation: boolean;
	setPackageOperation: (active: boolean) => void;
	packageImport: ImportProgress | null;
	setPackageImport: (progress: ImportProgress | null) => void;
	repositoriesVersion: number;
	bumpRepositories: () => void;
	incomingRepository: IncomingRepository | null;
	clearIncomingRepository: () => void;
	notify: (kind: NoticeKind, text: string, options?: NotifyOptions) => void;
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
		unityDownloadLimit: 0,
		updateCheckDisabled: false,
	});
	const [unityInstall, setUnityInstall] = useState<UnityInstall | null>(null);
	const [update, setUpdate] = useState<UpdateStatus | null>(null);
	const [packageOperation, setPackageOperation] = useState(false);
	const [packageImport, setPackageImport] = useState<ImportProgress | null>(null);
	const [repositoriesVersion, setRepositoriesVersion] = useState(0);
	const [incomingRepository, setIncomingRepository] = useState<IncomingRepository | null>(null);
	const nextId = useRef(1);

	const linkToast = useRef<number | null>(null);
	const packageOperationRef = useRef(false);
	const incomingRef = useRef<IncomingRepository | null>(null);
	const installNotice = useRef("");
	const announcedUpdate = useRef("");

	const dismissToast = useCallback((id: number) => {
		if (linkToast.current === id) {
			linkToast.current = null;
		}
		setToasts((current) => current.filter((toast) => toast.id !== id));
	}, []);

	const notify = useCallback(
		(kind: NoticeKind, text: string, options?: NotifyOptions) => {
			const id = nextId.current++;
			setToasts((current) => [...current, { id, kind, text, action: options?.action }]);
			if (kind === "error" || kind === "warning") {
				setLog((current) => [{ id, time: Date.now(), kind, text }, ...current].slice(0, 200));
			}
			if (!options?.sticky) {
				window.setTimeout(() => dismissToast(id), kind === "error" ? 7000 : 4000);
			}
			return id;
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

	const rescanProjects = useCallback(async () => {
		try {
			await refreshProjectInfo();
		} catch (error) {
			notify("error", errorMessage(error));
		}
		await refreshProjects();
	}, [notify, refreshProjects]);

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
		void rescanProjects();
		void refreshEditors();
		getSettings()
			.then(setSettings)
			.catch((error) => notify("error", errorMessage(error)));
	}, [refreshProjects, rescanProjects, refreshEditors, notify]);

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
		listen("close-blocked", () => notify("info", "Something is still in progress. Close the app again once it finishes."))
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

	const checkUpdates = useCallback(
		async (manual: boolean) => {
			try {
				const status = await checkForUpdate(manual);
				setUpdate(status);
				if (status.available && status.latest && status.url) {
					if (manual || announcedUpdate.current !== status.latest) {
						announcedUpdate.current = status.latest;
						const url = status.url;
						notify("info", `Version ${status.latest} is available.`, {
							sticky: true,
							action: {
								label: "View",
								run: () => void openExternal(url).catch((error) => notify("error", errorMessage(error))),
							},
						});
					}
				} else if (manual) {
					notify("success", "You have the latest version.");
				}
			} catch (error) {
				if (manual) {
					notify("error", errorMessage(error));
				}
			}
		},
		[notify],
	);

	useEffect(() => {
		void checkUpdates(false);
	}, [checkUpdates]);

	useEffect(() => {
		let unlisten: (() => void) | null = null;
		let cancelled = false;
		listen<UnityInstall>("unity-install", (event) => {
			const install = event.payload;
			if (install.notice && install.notice !== installNotice.current) {
				installNotice.current = install.notice;
				notify("warning", install.notice);
			}
			if (install.phase === "finished" || install.phase === "failed" || install.phase === "cancelled") {
				installNotice.current = "";
				setUnityInstall(null);
				notify(install.phase === "finished" ? "success" : install.phase === "failed" ? "error" : "info", install.message);
				void refreshEditors();
			} else {
				setUnityInstall(install);
			}
		})
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
	}, [notify, refreshEditors]);

	useEffect(() => {
		packageOperationRef.current = packageOperation;
	}, [packageOperation]);

	useEffect(() => {
		incomingRef.current = incomingRepository;
	}, [incomingRepository]);

	useEffect(() => {
		let unlisten: (() => void) | null = null;
		let cancelled = false;
		const open = () => {
			if (packageOperationRef.current) {
				notify("info", "Finish the current package change first, then open the link again.");
				announce();
				return;
			}
			deepLinkTakeAddRepository()
				.then((link) => {
					if (link && !cancelled) {
						setIncomingRepository(link);
						setRoute((current) => (current.page === "packages" ? current : { page: "packages" }));
					}
				})
				.catch(() => undefined);
		};
		const announce = () => {
			if (cancelled || linkToast.current !== null) {
				return;
			}
			linkToast.current = notify("info", "A repository link was opened. Nothing has been requested yet.", {
				sticky: true,
				action: { label: "Open", run: open },
			});
		};
		const arrived = () => {
			if (!incomingRef.current) {
				announce();
			}
		};
		listen("deep-link-add-repository", arrived)
			.then((stop) => {
				if (cancelled) {
					stop();
				} else {
					unlisten = stop;
					deepLinkPendingCount()
						.then((count) => {
							if (count > 0) {
								announce();
							}
						})
						.catch(() => undefined);
				}
			})
			.catch(() => undefined);
		return () => {
			cancelled = true;
			unlisten?.();
		};
	}, [notify]);

	const clearIncomingRepository = useCallback(() => {
		setIncomingRepository(null);
		deepLinkTakeAddRepository()
			.then((link) => {
				if (link) {
					setIncomingRepository(link);
				}
			})
			.catch(() => undefined);
	}, []);

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
			rescanProjects,
			editors,
			editorsLoading,
			editorsError,
			refreshEditors,
			settings,
			updateSettings,
			unityInstall,
			update,
			checkUpdates,
			packageOperation,
			setPackageOperation,
			packageImport,
			setPackageImport,
			repositoriesVersion,
			bumpRepositories: () => setRepositoriesVersion((value) => value + 1),
			incomingRepository,
			clearIncomingRepository,
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
			rescanProjects,
			editors,
			editorsLoading,
			editorsError,
			refreshEditors,
			settings,
			updateSettings,
			unityInstall,
			update,
			checkUpdates,
			packageOperation,
			packageImport,
			repositoriesVersion,
			incomingRepository,
			clearIncomingRepository,
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
