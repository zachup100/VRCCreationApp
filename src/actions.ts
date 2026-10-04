import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useApp } from "./AppContext";
import {
	addProject,
	importFromVcc,
	importPackagesFromVcc,
	openExternal,
	openProject,
	openProjectFolder,
	removeProject,
	setProjectFavorite,
} from "./api";
import type { ProjectInfo } from "./api";
import { errorMessage } from "./format";

export function useActions() {
	const { notify, refreshProjects, packageImport, setPackageImport, bumpRepositories } = useApp();

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
					`VCC project import finished: ${result.imported} imported, ${result.skipped} already in the list`,
				);
				await refreshProjects();
			}),
		packageImportRunning: packageImport,
		importPackagesVcc: async () => {
			if (packageImport) {
				return;
			}
			setPackageImport({ done: 0, total: 0 });
			let unlisten: (() => void) | null = null;
			try {
				unlisten = await listen<{ done: number; total: number }>("vcc-import-progress", (event) => setPackageImport(event.payload));
				const result = await importPackagesFromVcc();
				const added = result.repositoriesAdded + result.foldersAdded;
				const skipped = result.repositoriesSkipped;
				notify(
					added > 0 ? "success" : "info",
					`VCC package import finished: ${result.repositoriesAdded} repositories and ${result.foldersAdded} package folders added, ${skipped} repositories already installed`,
				);
				if (result.failures.length > 0) {
					const shown = result.failures.slice(0, 3).join("; ");
					const more = result.failures.length > 3 ? ` and ${result.failures.length - 3} more` : "";
					notify("error", `${result.failures.length} could not be imported: ${shown}${more}`);
				}
				bumpRepositories();
			} catch (error) {
				notify("error", errorMessage(error));
			} finally {
				unlisten?.();
				setPackageImport(null);
			}
		},
		open: (project: ProjectInfo, editorVersion?: string) =>
			guard(async () => {
				await openProject(project.path, editorVersion);
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
