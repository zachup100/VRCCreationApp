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
