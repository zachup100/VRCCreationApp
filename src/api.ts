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
