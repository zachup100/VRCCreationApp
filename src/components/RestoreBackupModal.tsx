import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { useApp } from "../AppContext";
import { restoreBackup } from "../api";
import type { BackupInfo, TaskProgress } from "../api";
import { errorMessage } from "../format";
import { Button } from "./Button";
import { Modal, TextField } from "./Controls";
import { Icon } from "./Icons";

const invalidName = /[\\/:*?"<>|\u0000-\u001f]/;

type State =
	| { step: "running" }
	| { step: "naming"; conflict: string; error: string | null }
	| { step: "done"; name: string; path: string }
	| { step: "failed"; message: string };

function nameProblem(value: string): string | null {
	const trimmed = value.trim();
	if (!trimmed) {
		return "Enter a project name";
	}
	if (invalidName.test(trimmed)) {
		return "The name contains characters that are not allowed";
	}
	if (/[. ]$/.test(trimmed)) {
		return "The name cannot end with a dot or a space";
	}
	return null;
}

export function RestoreBackupModal({ backup, location, onClose }: { backup: BackupInfo; location: string; onClose: () => void }) {
	const { refreshProjects, setPackageOperation, navigate } = useApp();
	const [state, setState] = useState<State>({ step: "running" });
	const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
	const [name, setName] = useState("");
	const running = state.step === "running";
	const started = useRef(false);

	useEffect(() => {
		setPackageOperation(running);
		return () => setPackageOperation(false);
	}, [running, setPackageOperation]);

	useEffect(() => {
		let unlisten: (() => void) | null = null;
		let cancelled = false;
		listen<TaskProgress>("project-task", (event) => {
			if (event.payload.kind === "restore") {
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
	}, []);

	const run = async (chosen?: string) => {
		setState({ step: "running" });
		setProgress(null);
		try {
			const outcome = await restoreBackup(backup.fileName, location, chosen);
			if (outcome.status === "conflict") {
				setName(chosen ? chosen : `${outcome.name} (Restored)`);
				setState({
					step: "naming",
					conflict: outcome.name,
					error: chosen ? `A folder named "${outcome.name}" already exists there too. Choose another name.` : null,
				});
				return;
			}
			await refreshProjects();
			setState({ step: "done", name: outcome.project.name, path: outcome.project.path });
		} catch (error) {
			setState({ step: "failed", message: errorMessage(error) });
		}
	};

	useEffect(() => {
		if (started.current) {
			return;
		}
		started.current = true;
		void run();
	});

	const problem = nameProblem(name);

	return (
		<Modal
			title={state.step === "naming" ? "Name Already Used" : "Restore Backup"}
			onClose={running ? () => undefined : onClose}
			actions={
				state.step === "naming" ? (
					<>
						<Button onClick={onClose}>Cancel</Button>
						<Button variant="primary" disabled={problem !== null} onClick={() => void run(name.trim())}>
							Restore
						</Button>
					</>
				) : state.step === "running" ? (
					<Button disabled>Restoring...</Button>
				) : state.step === "done" ? (
					<>
						<Button
							onClick={() => {
								onClose();
								navigate({ page: "projects" });
							}}
						>
							Go to Projects
						</Button>
						<Button variant="primary" onClick={onClose}>
							Close
						</Button>
					</>
				) : (
					<Button variant="primary" onClick={onClose}>
						Close
					</Button>
				)
			}
		>
			{state.step === "running" && (
				<>
					<p>
						Restoring <strong>{backup.projectName}</strong> to <code>{location}</code>
					</p>
					<div className="progress" role="progressbar" aria-valuemin={0} aria-valuemax={progress?.total ?? 0} aria-valuenow={progress?.done ?? 0}>
						<div className="progress-fill" style={{ width: `${progress && progress.total ? (progress.done / progress.total) * 100 : 0}%` }} />
					</div>
					{progress && (
						<p className="muted modal-note">
							{progress.done} / {progress.total} files
						</p>
					)}
				</>
			)}
			{state.step === "naming" && (
				<form
					className="modal-form"
					onSubmit={(event) => {
						event.preventDefault();
						if (!problem) {
							void run(name.trim());
						}
					}}
				>
					<p>
						A folder named <strong>{state.conflict}</strong> already exists in <code>{location}</code>. What should the restored project be called?
					</p>
					<TextField label="Project Name" value={name} autoFocus valid={problem === null} onChange={(event) => setName(event.target.value)} />
					{(problem || state.error) && <p className="field-error">{problem ?? state.error}</p>}
				</form>
			)}
			{state.step === "done" && (
				<p>
					<Icon name="check" size={18} className="ok" /> Restored <strong>{state.name}</strong> and added it to your projects:
					<br />
					<code>{state.path}</code>
				</p>
			)}
			{state.step === "failed" && <p className="field-error">{state.message}</p>}
		</Modal>
	);
}
