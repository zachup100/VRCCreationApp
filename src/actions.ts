import { open } from "@tauri-apps/plugin-dialog";
import { useApp } from "./AppContext";
import {
	addProject,
	importFromVcc,
	openExternal,
	openProject,
	openProjectFolder,
	removeProject,
	setProjectFavorite,
} from "./api";
import type { ProjectInfo } from "./api";
import { errorMessage } from "./format";

export function useActions() {
	const { notify, refreshProjects } = useApp();

	const guard = async (task: () => Promise<void>) => {
		try {
			await task();
		} catch (error) {
			notify("error", errorMessage(error));
		}
	};

	return {
		addExisting: () =>
			guard(async () => {
				const selected = await open({ directory: true, multiple: false, title: "Select a Unity project folder" });
				if (typeof selected !== "string") {
					return;
				}
				const project = await addProject(selected);
				notify("success", `Added ${project.name}`);
				await refreshProjects();
			}),
		importVcc: () =>
			guard(async () => {
				const result = await importFromVcc();
				notify(
					result.imported > 0 ? "success" : "info",
					`VCC import finished: ${result.imported} imported, ${result.skipped} already in the list`,
				);
				await refreshProjects();
			}),
		open: (project: ProjectInfo) =>
			guard(async () => {
				await openProject(project.path);
				notify("info", `Opening ${project.name} in Unity`);
				await refreshProjects();
			}),
		openFolder: (project: ProjectInfo) => guard(() => openProjectFolder(project.path)),
		remove: (project: ProjectInfo) =>
			guard(async () => {
				await removeProject(project.path);
				notify("info", `Removed ${project.name} from the list`);
				await refreshProjects();
			}),
		toggleFavorite: (project: ProjectInfo) =>
			guard(async () => {
				await setProjectFavorite(project.path, !project.favorite);
				await refreshProjects();
			}),
		copyPath: (project: ProjectInfo) =>
			guard(async () => {
				await navigator.clipboard.writeText(project.path);
				notify("info", "Path copied to the clipboard");
			}),
		openLink: (url: string) => guard(() => openExternal(url)),
	};
}
