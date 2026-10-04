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

export function openProject(path: string): Promise<void> {
	return invoke<void>("open_project", { path });
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
