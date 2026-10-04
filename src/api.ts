import { invoke } from "@tauri-apps/api/core";

export interface UnityEditorInfo {
	version: string;
	path: string;
	architecture: string | null;
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

export function repositoryPreview(url: string): Promise<RepositoryPreview> {
	return invoke<RepositoryPreview>("repository_preview", { url });
}

export function repositoryAdd(url: string): Promise<void> {
	return invoke<void>("repository_add", { url });
}

export function repositoryRemove(index: number, expectedId: string): Promise<void> {
	return invoke<void>("repository_remove", { index, expectedId });
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
}

export interface ProjectPackages {
	unityVersion: string;
	rows: PackageRow[];
}

export function projectPackages(path: string, refresh: boolean): Promise<ProjectPackages> {
	return invoke<ProjectPackages>("project_packages", { path, refresh });
}

export interface ChangeRequest {
	installs: { name: string; version: string }[];
	removes: string[];
}

export interface ChangeSummary {
	installs: {
		name: string;
		displayName: string;
		version: string;
		previousVersion: string | null;
		kind: "new" | "upgrade" | "downgrade" | "reinstall";
	}[];
	removes: { name: string; reason: "requested" | "legacy" | "unused" }[];
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

export function createProject(template: string, name: string, location: string): Promise<ProjectInfo> {
	return invoke<ProjectInfo>("create_project", { template, name, location });
}

export interface AppSettings {
	defaultProjectLocation: string | null;
	backupLocation: string | null;
	showPrerelease: boolean;
	defaultUnityArgs: string;
}

export interface SettingsPatch {
	defaultProjectLocation?: string;
	backupLocation?: string;
	showPrerelease?: boolean;
	defaultUnityArgs?: string;
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
