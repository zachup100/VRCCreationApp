import { invoke } from "@tauri-apps/api/core";

export interface UnityEditorInfo {
	version: string;
	path: string;
	architecture: string | null;
	custom: boolean;
}

export interface HubStatus {
	found: boolean;
	path: string | null;
	canInstall: boolean;
}

export interface RecommendedEditor {
	version: string;
	label: string;
	note: string;
}

export type InstallPhase = "starting" | "downloading" | "verifying" | "installing" | "finished" | "failed" | "cancelled";

export interface UnityInstall {
	kind: "hub" | "editor";
	phase: InstallPhase;
	bytes: number;
	total: number;
	line: string;
	version: string;
	notice: string;
	message: string;
}

export type ProjectCategory = "avatar" | "world" | "legacy-avatar" | "legacy-world" | "other";

export interface ProjectInfo {
	path: string;
	name: string;
	unityVersion: string | null;
	category: ProjectCategory;
	favorite: boolean;
	lastModified: number;
	launchArgs: string | null;
	exists: boolean;
}

export interface ImportResult {
	found: number;
	imported: number;
	skipped: number;
}

export function listUnityEditors(): Promise<UnityEditorInfo[]> {
	return invoke<UnityEditorInfo[]>("list_unity_editors");
}

export function addCustomEditor(path: string): Promise<UnityEditorInfo> {
	return invoke<UnityEditorInfo>("add_custom_editor", { path });
}

export function unityHubStatus(): Promise<HubStatus> {
	return invoke<HubStatus>("unity_hub_status");
}

export function recommendedEditors(): Promise<RecommendedEditor[]> {
	return invoke<RecommendedEditor[]>("recommended_editors");
}

export function installUnityHub(): Promise<void> {
	return invoke<void>("install_unity_hub");
}

export function installUnityEditor(version: string, android: boolean, ios: boolean): Promise<void> {
	return invoke<void>("install_unity_editor", { version, android, ios });
}

export function cancelUnityInstall(): Promise<boolean> {
	return invoke<boolean>("cancel_unity_install");
}

export function removeCustomEditor(path: string): Promise<void> {
	return invoke<void>("remove_custom_editor", { path });
}

export function openExternal(url: string): Promise<void> {
	return invoke<void>("open_external", { url });
}

export function openProject(path: string, editorVersion?: string): Promise<void> {
	return invoke<void>("open_project", { path, editorVersion: editorVersion ?? null });
}

export function setProjectLaunchArgs(path: string, args: string): Promise<void> {
	return invoke<void>("set_project_launch_args", { path, args });
}

export function openProjectFolder(path: string): Promise<void> {
	return invoke<void>("open_project_folder", { path });
}

export function listProjects(): Promise<ProjectInfo[]> {
	return invoke<ProjectInfo[]>("list_projects");
}

export function addProject(path: string): Promise<ProjectInfo> {
	return invoke<ProjectInfo>("add_project", { path });
}

export function removeProject(path: string): Promise<void> {
	return invoke<void>("remove_project", { path });
}

export function deleteProject(path: string): Promise<void> {
	return invoke<void>("delete_project", { path });
}

export function setProjectFavorite(path: string, favorite: boolean): Promise<void> {
	return invoke<void>("set_project_favorite", { path, favorite });
}

export function importFromVcc(): Promise<ImportResult> {
	return invoke<ImportResult>("import_from_vcc");
}

export interface BuiltinRepository {
	id: string;
	name: string;
	hidden: boolean;
}

export interface CommunityRepository {
	index: number;
	id: string;
	url: string | null;
	name: string;
	author: string | null;
	hidden: boolean;
}

export interface RepositoriesInfo {
	builtin: BuiltinRepository[];
	community: CommunityRepository[];
}

export type RepositoryPreview =
	| { status: "badUrl" }
	| { status: "duplicated"; byUrl: boolean; name: string }
	| { status: "downloadError"; message: string }
	| { status: "success"; id: string; url: string; name: string; author: string | null; packages: string[] };

export function repositoriesInfo(): Promise<RepositoriesInfo> {
	return invoke<RepositoriesInfo>("repositories_info");
}

export interface HeaderInput {
	name: string;
	value: string;
}

export function repositoryPreview(url: string, headers: HeaderInput[], ignoreIndex?: number): Promise<RepositoryPreview> {
	return invoke<RepositoryPreview>("repository_preview", { url, headers, ignoreIndex: ignoreIndex ?? null });
}

export function repositoryHeaders(index: number, expectedId: string): Promise<HeaderInput[]> {
	return invoke<HeaderInput[]>("repository_headers", { index, expectedId });
}

export function repositoryUpdate(index: number, expectedId: string, url: string, headers: HeaderInput[]): Promise<void> {
	return invoke<void>("repository_update", { index, expectedId, url, headers });
}

export function repositoryAdd(url: string, headers: HeaderInput[]): Promise<void> {
	return invoke<void>("repository_add", { url, headers });
}

export function repositoryRemove(index: number, expectedId: string): Promise<void> {
	return invoke<void>("repository_remove", { index, expectedId });
}

export interface DeepLinkStatus {
	supported: boolean;
	registered: boolean;
	otherHandler: boolean;
}

export interface IncomingRepository {
	url: string;
	headers: HeaderInput[];
}

export function deepLinkStatus(): Promise<DeepLinkStatus> {
	return invoke<DeepLinkStatus>("deep_link_status");
}

export function deepLinkRegister(): Promise<DeepLinkStatus> {
	return invoke<DeepLinkStatus>("deep_link_register");
}

export function deepLinkUnregister(): Promise<DeepLinkStatus> {
	return invoke<DeepLinkStatus>("deep_link_unregister");
}

export function deepLinkPendingCount(): Promise<number> {
	return invoke<number>("deep_link_pending_count");
}

export function deepLinkTakeAddRepository(): Promise<IncomingRepository | null> {
	return invoke<IncomingRepository | null>("deep_link_take_add_repository");
}

export function getPrivateHostBlocking(): Promise<boolean> {
	return invoke<boolean>("get_private_host_blocking");
}

export function setPrivateHostBlocking(enabled: boolean): Promise<boolean> {
	return invoke<boolean>("set_private_host_blocking", { enabled });
}

export function repositorySetHidden(id: string, hidden: boolean): Promise<void> {
	return invoke<void>("repository_set_hidden", { id, hidden });
}

export type PackageSource = "official" | "curated" | "community" | "local";

export interface PackageRow {
	name: string;
	displayName: string;
	installedVersion: string | null;
	latestVersion: string | null;
	versions: string[];
	source: PackageSource;
	repository: string | null;
	hasUpdate: boolean;
	requiresSdk: PresetKind | null;
}

export interface ProjectPackages {
	unityVersion: string;
	rows: PackageRow[];
	missing: string[];
	redundant: string[];
}

export function projectPackages(path: string, refresh: boolean): Promise<ProjectPackages> {
	return invoke<ProjectPackages>("project_packages", { path, refresh });
}

export interface ChangeRequest {
	installs: { name: string; version: string }[];
	removes: string[];
	resolve?: boolean;
	reinstall?: string[];
	acknowledgeDependents?: boolean;
}

export interface ChangeSummary {
	installs: {
		name: string;
		displayName: string;
		version: string;
		previousVersion: string | null;
		kind: "new" | "upgrade" | "downgrade" | "reinstall";
	}[];
	removes: { name: string; displayName: string; reason: "requested" | "dependent" | "legacy" | "unused"; dependsOn: string[] }[];
	conflicts: {
		package: string;
		conflictingPackages: string[];
		conflictsWithUnity: boolean;
		unlockedNames: string[];
	}[];
	legacyFiles: string[];
	legacyFolders: string[];
}

export function projectPlanChanges(path: string, request: ChangeRequest): Promise<ChangeSummary> {
	return invoke<ChangeSummary>("project_plan_changes", { path, request });
}

export function projectApplyChanges(path: string, request: ChangeRequest): Promise<void> {
	return invoke<void>("project_apply_changes", { path, request });
}

export interface PackageImportResult {
	repositoriesFound: number;
	repositoriesAdded: number;
	repositoriesSkipped: number;
	foldersFound: number;
	foldersAdded: number;
	failures: string[];
}

export function importPackagesFromVcc(): Promise<PackageImportResult> {
	return invoke<PackageImportResult>("import_packages_from_vcc");
}

export interface TemplateInfo {
	id: string;
	title: string;
	blurb: string;
	kind: "world" | "avatar";
	year: string;
	unityVersion: string;
}

export function listTemplates(): Promise<TemplateInfo[]> {
	return invoke<TemplateInfo[]>("list_templates");
}

export interface CreatedProject {
	project: ProjectInfo;
	warnings: string[];
	notes: string[];
}

export function createProject(template: string, name: string, location: string, presetId?: string): Promise<CreatedProject> {
	return invoke<CreatedProject>("create_project", { template, name, location, presetId: presetId ?? null });
}

export interface AppSettings {
	defaultProjectLocation: string | null;
	backupLocation: string | null;
	showPrerelease: boolean;
	defaultUnityArgs: string;
	unityDownloadLimit: number;
	updateCheckDisabled: boolean;
}

export interface SettingsPatch {
	defaultProjectLocation?: string;
	backupLocation?: string;
	showPrerelease?: boolean;
	defaultUnityArgs?: string;
	unityDownloadLimit?: number;
	updateCheckDisabled?: boolean;
}

export interface UpdateStatus {
	current: string;
	latest: string | null;
	available: boolean;
	url: string | null;
	checkedAt: number;
}

export function checkForUpdate(manual: boolean): Promise<UpdateStatus> {
	return invoke<UpdateStatus>("check_for_update", { manual });
}

export function getSettings(): Promise<AppSettings> {
	return invoke<AppSettings>("get_settings");
}

export function updateSettings(patch: SettingsPatch): Promise<AppSettings> {
	return invoke<AppSettings>("update_settings", { patch });
}

export interface BackupResult {
	path: string;
	files: number;
	bytes: number;
}

export type MigrationKind = "vpm" | "unity2022";

export type TaskProgress =
	| { kind: "phase"; name: "backup" | "packages" | "unity" }
	| { kind: "backup"; done: number; total: number }
	| { kind: "restore"; done: number; total: number }
	| { kind: "duplicate"; done: number; total: number }
	| { kind: "unityLog"; line: string };

export function backupProject(path: string): Promise<BackupResult> {
	return invoke<BackupResult>("backup_project", { path });
}

export function migrateProject(path: string, kind: MigrationKind, backup: boolean): Promise<BackupResult | null> {
	return invoke<BackupResult | null>("migrate_project", { path, kind, backup });
}

export function openBackupFolder(): Promise<void> {
	return invoke<void>("open_backup_folder");
}

export interface BackupInfo {
	fileName: string;
	projectName: string;
	category: ProjectCategory;
	unityVersion: string | null;
	createdAt: number;
	sizeBytes: number;
}

export type RestoreOutcome = { status: "restored"; project: ProjectInfo } | { status: "conflict"; name: string };

export function listBackups(): Promise<BackupInfo[]> {
	return invoke<BackupInfo[]>("list_backups");
}

export function deleteBackup(fileName: string): Promise<void> {
	return invoke<void>("delete_backup", { fileName });
}

export function restoreBackup(fileName: string, location: string, name?: string): Promise<RestoreOutcome> {
	return invoke<RestoreOutcome>("restore_backup", { fileName, location, name: name ?? null });
}

export type PresetKind = "world" | "avatar";

export interface PresetPackage {
	name: string;
	version: string | null;
	displayName?: string | null;
}

export interface Preset {
	id: string;
	name: string;
	kind: PresetKind;
	packages: PresetPackage[];
	createdAt: number;
	updatedAt: number;
}

export interface PresetInput {
	id: string | null;
	name: string;
	kind: PresetKind;
	packages: PresetPackage[];
}

export interface AvailablePackage {
	name: string;
	displayName: string;
	versions: string[];
	latestVersion: string | null;
	source: PackageSource;
	repository: string | null;
	requiresSdk: PresetKind | null;
	hidden: boolean;
}

export function listPresets(): Promise<Preset[]> {
	return invoke<Preset[]>("list_presets");
}

export function savePreset(input: PresetInput): Promise<Preset> {
	return invoke<Preset>("save_preset", { input });
}

export function deletePreset(id: string): Promise<void> {
	return invoke<void>("delete_preset", { id });
}

export function availablePackages(refresh: boolean): Promise<AvailablePackage[]> {
	return invoke<AvailablePackage[]>("available_packages", { refresh });
}

export interface BackendLogEntry {
	id: number;
	time: number;
	level: "info" | "success" | "warning" | "error";
	message: string;
}

export function getLogs(): Promise<BackendLogEntry[]> {
	return invoke<BackendLogEntry[]>("get_logs");
}

export function clearBackendLogs(): Promise<void> {
	return invoke<void>("clear_logs");
}

export function projectCleanupManifest(path: string): Promise<string[]> {
	return invoke<string[]>("project_cleanup_manifest", { path });
}

export function duplicateProject(path: string, name: string, includeLibrary: boolean): Promise<ProjectInfo> {
	return invoke<ProjectInfo>("duplicate_project", { path, name, includeLibrary });
}

export function refreshProjectInfo(): Promise<number> {
	return invoke<number>("refresh_project_info");
}

export function setPackagesHidden(names: string[], hidden: boolean): Promise<void> {
	return invoke<void>("set_packages_hidden", { names, hidden });
}

export function showAllPackages(): Promise<void> {
	return invoke<void>("show_all_packages");
}

export function getLicenses(): Promise<string> {
	return invoke<string>("get_licenses");
}
