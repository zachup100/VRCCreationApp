import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import { useApp } from "../AppContext";
import { useActions } from "../actions";
import { backupProject, migrateProject, openBackupFolder } from "../api";
import type { BackupResult, MigrationKind, ProjectInfo, TaskProgress } from "../api";
import { errorMessage } from "../format";
import { Button } from "./Button";
import { Checkbox, Modal } from "./Controls";
import { Icon } from "./Icons";

export type TaskMode = "backup" | MigrationKind;

const UNITY_2022 = "2022.3.22f1";

const titles: Record<TaskMode, string> = {
	backup: "Back Up Project",
	vpm: "Migrate Project",
	unity2022: "Migrate to Unity 2022",
};

type Phase = "backup" | "packages" | "unity";

type State =
	| { step: "confirm" }
	| { step: "running" }
	| { step: "done"; backup: BackupResult | null }
	| { step: "failed"; message: string; backupRequested: boolean };

function formatBytes(bytes: number): string {
	if (bytes < 1024 * 1024) {
		return `${Math.max(1, Math.round(bytes / 1024))} KB`;
	}
	if (bytes < 1024 * 1024 * 1024) {
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}
	return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function ProjectTaskModal({ project, mode, onClose }: { project: ProjectInfo; mode: TaskMode; onClose: () => void }) {
	const { settings, editors, editorsLoading, refreshProjects, setPackageOperation, notify } = useApp();
	const actions = useActions();
	const [state, setState] = useState<State>({ step: "confirm" });
	const [backup, setBackup] = useState(true);
	const [phase, setPhase] = useState<Phase | null>(null);
	const [backupProgress, setBackupProgress] = useState<{ done: number; total: number } | null>(null);
	const [logLines, setLogLines] = useState<string[]>([]);
	const logRef = useRef<HTMLPreElement>(null);

	const running = state.step === "running";
	const needsUnity = mode === "unity2022";
	const unityMissing = needsUnity && !editorsLoading && !editors.some((editor) => editor.version === UNITY_2022);

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
			const progress = event.payload;
			if (progress.kind === "phase") {
				setPhase(progress.name);
			} else if (progress.kind === "backup") {
				setBackupProgress({ done: progress.done, total: progress.total });
			} else if (progress.kind === "unityLog") {
				setLogLines((lines) => [...lines.slice(-80), progress.line]);
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

	useEffect(() => {
		if (logRef.current) {
			logRef.current.scrollTop = logRef.current.scrollHeight;
		}
	}, [logLines]);

	const start = async () => {
		setState({ step: "running" });
		setPhase(null);
		setBackupProgress(null);
		setLogLines([]);
		try {
			const result =
				mode === "backup" ? await backupProject(project.path) : await migrateProject(project.path, mode, backup);
			await refreshProjects();
			notify("success", mode === "backup" ? "Backup created" : "Project migrated");
			setState({ step: "done", backup: result });
		} catch (error) {
			await refreshProjects();
			setState({ step: "failed", message: errorMessage(error), backupRequested: mode === "backup" ? false : backup });
		}
	};

	const backupWhere = settings.backupLocation ?? "the app's Backups folder";
	const phaseOrder: Phase[] = mode === "backup" ? ["backup"] : [...(backup ? (["backup"] as Phase[]) : []), "packages", ...(needsUnity ? (["unity"] as Phase[]) : [])];
	const phaseLabels: Record<Phase, string> = {
		backup: "Backing up the project",
		packages: mode === "unity2022" ? "Updating VRChat packages" : "Migrating to the VPM package system",
		unity: "Upgrading the project with Unity 2022",
	};

	const closeable = !running;

	return (
		<Modal
			title={titles[mode]}
			onClose={closeable ? onClose : () => undefined}
			actions={
				state.step === "confirm" ? (
					<>
						<Button onClick={onClose}>Cancel</Button>
						<Button variant="primary" disabled={unityMissing} onClick={() => void start()}>
							{mode === "backup" ? "Create Backup" : "Start Migration"}
						</Button>
					</>
				) : state.step === "running" ? (
					<Button disabled>Working...</Button>
				) : (
					<>
						{(state.step === "done" ? state.backup !== null : state.backupRequested) && (
							<Button icon="folderOpen" onClick={() => void openBackupFolder().catch((error) => notify("error", errorMessage(error)))}>
								Open Backup Folder
							</Button>
						)}
						<Button variant="primary" onClick={onClose}>
							Close
						</Button>
					</>
				)
			}
		>
			{state.step === "confirm" && (
				<>
					<p>
						{mode === "backup" && (
							<>
								Create a zip backup of <strong>{project.name}</strong>. Library, Temp, Logs and build cache folders are left out.
							</>
						)}
						{mode === "vpm" && (
							<>
								Migrate <strong>{project.name}</strong> from the legacy SDK to the VPM package system. The VRChat SDK is replaced with its VPM packages, and the old SDK files are removed from the project.
							</>
						)}
						{mode === "unity2022" && (
							<>
								Upgrade <strong>{project.name}</strong> from Unity 2019 to Unity {UNITY_2022}. VRChat packages are updated and old XR packages removed, then Unity {UNITY_2022} is run in the background to upgrade the project. This can take several minutes.
							</>
						)}
					</p>
					{mode !== "backup" && (
						<>
							<p className="warn-text">
								<Icon name="alert" size={16} /> Close the project in Unity first. This changes your project files and cannot be undone without a backup.
							</p>
							<label className="check-row">
								<Checkbox checked={backup} onChange={setBackup} label="Create a backup first" />
								<span>Create a backup first (recommended)</span>
							</label>
						</>
					)}
					<p className="muted modal-note">Backups are saved to {backupWhere}.</p>
					{unityMissing && (
						<p className="warn-text">
							<Icon name="alert" size={16} /> Unity {UNITY_2022} is not installed.{" "}
							<button type="button" className="link-btn" onClick={() => void actions.openLink("https://unity.com/download")}>
								Open the Unity download page
							</button>
						</p>
					)}
				</>
			)}

			{state.step === "running" && (
				<div className="task-steps">
					{phaseOrder.map((item) => {
						const index = phaseOrder.indexOf(item);
						const current = phase === null ? index === 0 : phaseOrder.indexOf(phase) === index;
						const done = phase !== null && phaseOrder.indexOf(phase) > index;
						return (
							<div key={item} className={`task-step${current ? " current" : ""}${done ? " done" : ""}`}>
								<Icon name={done ? "check" : current ? "refresh" : "info"} size={18} className={current ? "spin" : undefined} />
								<span>{phaseLabels[item]}</span>
								{item === "backup" && current && backupProgress && (
									<span className="muted">
										{backupProgress.done} / {backupProgress.total} files
									</span>
								)}
							</div>
						);
					})}
					{phase === "backup" && backupProgress && (
						<div className="progress" role="progressbar" aria-valuemin={0} aria-valuemax={backupProgress.total} aria-valuenow={backupProgress.done}>
							<div className="progress-fill" style={{ width: `${backupProgress.total ? (backupProgress.done / backupProgress.total) * 100 : 0}%` }} />
						</div>
					)}
					{phase === "unity" && <pre className="task-log" ref={logRef}>{logLines.join("\n") || "Starting Unity..."}</pre>}
				</div>
			)}

			{state.step === "done" && (
				<p>
					<Icon name="check" size={18} className="ok" />{" "}
					{state.backup ? (
						<>
							Backup saved ({state.backup.files} files, {formatBytes(state.backup.bytes)}):
							<br />
							<code>{state.backup.path}</code>
						</>
					) : (
						"The project was migrated."
					)}
				</p>
			)}

			{state.step === "failed" && (
				<>
					<p className="field-error">{state.message}</p>
					{state.backupRequested && <p className="muted">A backup was requested first, so a copy of the project may be in the backup folder.</p>}
				</>
			)}
		</Modal>
	);
}
