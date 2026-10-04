import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";
import {
	addProject,
	importFromVcc,
	listProjects,
	removeProject,
	setProjectFavorite,
} from "./api";
import type { ProjectInfo } from "./api";

export function Projects() {
	const [projects, setProjects] = useState<ProjectInfo[]>([]);
	const [message, setMessage] = useState<{ kind: "info" | "error"; text: string } | null>(null);

	const refresh = useCallback(async () => {
		try {
			setProjects(await listProjects());
		} catch (error) {
			setMessage({ kind: "error", text: String(error) });
		}
	}, []);

	useEffect(() => {
		void refresh();
	}, [refresh]);

	const run = async (action: () => Promise<string | null>) => {
		try {
			const text = await action();
			setMessage(text ? { kind: "info", text } : null);
		} catch (error) {
			setMessage({ kind: "error", text: String(error) });
		}
		await refresh();
	};

	const onAdd = () =>
		run(async () => {
			const selected = await open({ directory: true, multiple: false });
			if (typeof selected !== "string") {
				return null;
			}
			const project = await addProject(selected);
			return `Added ${project.name}`;
		});

	const onImport = () =>
		run(async () => {
			const result = await importFromVcc();
			return `VCC import: ${result.imported} imported, ${result.skipped} skipped, ${result.found} found`;
		});

	return (
		<section>
			<h2>Projects ({projects.length})</h2>
			<div className="toolbar">
				<button type="button" onClick={() => void onAdd()}>
					Add project
				</button>
				<button type="button" onClick={() => void onImport()}>
					Import from VCC
				</button>
			</div>
			{message && <p className={message.kind === "error" ? "error" : "info"}>{message.text}</p>}
			{projects.length === 0 && <p>No projects yet. Add a Unity project folder or import from VCC.</p>}
			<ul className="list">
				{projects.map((project) => (
					<li key={project.path} className={project.exists ? "" : "missing"}>
						<div className="row">
							<button
								type="button"
								className="icon"
								title={project.favorite ? "Remove favorite" : "Favorite"}
								onClick={() =>
									void run(async () => {
										await setProjectFavorite(project.path, !project.favorite);
										return null;
									})
								}
							>
								{project.favorite ? "★" : "☆"}
							</button>
							<strong>{project.name}</strong>
							<span className="tag">{project.category}</span>
							{project.unityVersion && <span className="tag">{project.unityVersion}</span>}
							{!project.exists && <span className="tag warn">missing</span>}
							<button
								type="button"
								className="push"
								onClick={() =>
									void run(async () => {
										await removeProject(project.path);
										return null;
									})
								}
							>
								Remove
							</button>
						</div>
						<div className="path">{project.path}</div>
					</li>
				))}
			</ul>
		</section>
	);
}
