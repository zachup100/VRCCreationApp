import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { useApp } from "../AppContext";
import { duplicateProject } from "../api";
import type { ProjectInfo, TaskProgress } from "../api";
import { projectNameProblem } from "../names";
import { errorMessage } from "../format";
import { Button } from "./Button";
import { Checkbox, Modal, TextField } from "./Controls";
import { Icon } from "./Icons";

type State =
	| { step: "form"; error: string | null }
	| { step: "running" }
	| { step: "done"; project: ProjectInfo }
	| { step: "failed"; message: string };

function parentOf(path: string): string {
	const trimmed = path.replace(/[\\/]+$/, "");
	const cut = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
	return cut > 0 ? trimmed.slice(0, cut) : trimmed;
}

export function DuplicateProjectModal({ project, onClose }: { project: ProjectInfo; onClose: () => void }) {
	const { refreshProjects, setPackageOperation, notify } = useApp();
	const [name, setName] = useState(`${project.name} Copy`);
	const [includeLibrary, setIncludeLibrary] = useState(true);
	const [state, setState] = useState<State>({ step: "form", error: null });
	const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
	const running = state.step === "running";
	const problem = projectNameProblem(name);

	useEffect(() => {
		setPackageOperation(running);
		return () => setPackageOperation(false);
	}, [running, setPackageOperation]);

	useEffect(() => {
		if (!running) {
			return;
		}
		let unlisten: (() => void) | null = null;
		let cancelled = false;
		listen<TaskProgress>("project-task", (event) => {
			if (event.payload.kind === "duplicate") {
				setProgress({ done: event.payload.done, total: event.payload.total });
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
	}, [running]);

	const start = async () => {
		if (problem) {
			return;
		}
		setProgress(null);
		setState({ step: "running" });
		try {
			const created = await duplicateProject(project.path, name.trim(), includeLibrary);
			await refreshProjects();
			notify("success", `Duplicated ${project.name} as ${created.name}`);
			setState({ step: "done", project: created });
		} catch (error) {
			const message = errorMessage(error);
			await refreshProjects();
			setState(message.includes("already exists") ? { step: "form", error: message } : { step: "failed", message });
		}
	};

	return (
		<Modal
			title="Duplicate Project"
			onClose={running ? () => undefined : onClose}
			actions={
				state.step === "form" ? (
					<>
						<Button onClick={onClose}>Cancel</Button>
						<Button variant="primary" icon="copy" disabled={problem !== null} onClick={() => void start()}>
							Duplicate
						</Button>
					</>
				) : running ? (
					<Button disabled>Copying...</Button>
				) : (
					<Button variant="primary" onClick={onClose}>
						Close
					</Button>
				)
			}
		>
			{state.step === "form" && (
				<form
					className="modal-form"
					onSubmit={(event) => {
						event.preventDefault();
						void start();
					}}
				>
					<p>
						Create a copy of <strong>{project.name}</strong> and add it to your projects. The copy is made next to the original, in <code>{parentOf(project.path)}</code>.
					</p>
					<TextField
						label="Name of the new project"
						value={name}
						autoFocus
						valid={problem === null}
						onChange={(event) => setName(event.target.value)}
					/>
					{(problem || state.error) && <p className="field-error">{problem ?? state.error}</p>}
					<label className="check-row">
						<Checkbox checked={includeLibrary} onChange={setIncludeLibrary} label="Include the Library folder" />
						<span>Include the Library folder (the copy opens faster, but is larger and takes longer to make)</span>
					</label>
					<p className="warn-text">
						<Icon name="alert" size={16} /> Close the project in Unity first, so the copy is taken from a clean state.
					</p>
				</form>
			)}
			{state.step === "running" && (
				<>
					<p>
						Copying <strong>{project.name}</strong> to <strong>{name.trim()}</strong>...
					</p>
					<div className="progress" role="progressbar" aria-valuemin={0} aria-valuemax={progress?.total ?? 0} aria-valuenow={progress?.done ?? 0}>
						<div className="progress-fill" style={{ width: `${progress && progress.total ? (progress.done / progress.total) * 100 : 0}%` }} />
					</div>
					{progress && (
						<p className="muted modal-note">
							{progress.done} / {progress.total} items
						</p>
					)}
				</>
			)}
			{state.step === "done" && (
				<p>
					<Icon name="check" size={18} className="ok" /> Created <strong>{state.project.name}</strong>:
					<br />
					<code>{state.project.path}</code>
				</p>
			)}
			{state.step === "failed" && <p className="field-error">{state.message}</p>}
		</Modal>
	);
}
