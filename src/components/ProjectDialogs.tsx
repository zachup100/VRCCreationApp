import { useState } from "react";
import type { ReactNode } from "react";
import { useApp } from "../AppContext";
import { useActions } from "../actions";
import { setProjectLaunchArgs } from "../api";
import type { ProjectInfo } from "../api";
import { errorMessage } from "../format";
import { Button } from "./Button";
import { Modal, TextField } from "./Controls";
import { Icon } from "./Icons";

type Dialog = { kind: "openWith"; project: ProjectInfo } | { kind: "launch"; project: ProjectInfo };

function OpenWithModal({ project, onClose }: { project: ProjectInfo; onClose: () => void }) {
	const { editors } = useApp();
	const actions = useActions();
	const [version, setVersion] = useState(project.unityVersion ?? editors[0]?.version ?? "");
	const different = project.unityVersion !== null && version !== project.unityVersion;

	return (
		<Modal
			title="Open With"
			onClose={onClose}
			actions={
				<>
					<Button onClick={onClose}>Cancel</Button>
					<Button
						variant="primary"
						disabled={!version}
						onClick={() => {
							onClose();
							void actions.open(project, version);
						}}
					>
						Open Project
					</Button>
				</>
			}
		>
			{editors.length === 0 ? (
				<p>No Unity editors were found. Install one and rescan in Settings.</p>
			) : (
				<>
					<p>
						Choose the Unity editor to open <strong>{project.name}</strong> with.
					</p>
					<ul className="radio-list" role="radiogroup" aria-label="Unity editor">
						{editors.map((editor) => (
							<li key={`${editor.version}-${editor.path}`}>
								<button
									type="button"
									role="radio"
									aria-checked={version === editor.version}
									className={`radio-option${version === editor.version ? " selected" : ""}`}
									onClick={() => setVersion(editor.version)}
								>
									<span className="radio-dot" />
									<strong>{editor.version}</strong>
									{editor.architecture && <span className="tag">{editor.architecture}</span>}
									{editor.version === project.unityVersion && <span className="tag update">Project version</span>}
								</button>
							</li>
						))}
					</ul>
					{different && (
						<p className="warn-text">
							<Icon name="alert" size={16} /> Opening with a different Unity version can upgrade the project and may not be reversible. Back up the project first.
						</p>
					)}
				</>
			)}
		</Modal>
	);
}

function LaunchOptionsModal({ project, onClose }: { project: ProjectInfo; onClose: () => void }) {
	const { notify, refreshProjects, settings } = useApp();
	const [args, setArgs] = useState(project.launchArgs ?? "");
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<string | null>(null);

	const save = async () => {
		setSaving(true);
		try {
			await setProjectLaunchArgs(project.path, args);
			notify("success", "Launch options saved");
			await refreshProjects();
			onClose();
		} catch (failure) {
			setError(errorMessage(failure));
			setSaving(false);
		}
	};

	return (
		<Modal
			title="Launch Options"
			onClose={onClose}
			actions={
				<>
					<Button onClick={onClose}>Cancel</Button>
					<Button variant="primary" disabled={saving} onClick={() => void save()}>
						Save
					</Button>
				</>
			}
		>
			<p>
				Extra command line arguments for Unity when opening <strong>{project.name}</strong>. Leave empty to use the default
				{settings.defaultUnityArgs ? ` (${settings.defaultUnityArgs})` : " from Settings"}.
			</p>
			<form
				className="modal-form"
				onSubmit={(event) => {
					event.preventDefault();
					void save();
				}}
			>
				<TextField
					label="Launch arguments"
					value={args}
					placeholder="-vrcSdkLogging -disable-assembly-updater"
					autoFocus
					onChange={(event) => {
						setArgs(event.target.value);
						setError(null);
					}}
				/>
				{error && <p className="field-error">{error}</p>}
			</form>
		</Modal>
	);
}

export function useProjectDialogs(): { openWith: (project: ProjectInfo) => void; launchOptions: (project: ProjectInfo) => void; element: ReactNode } {
	const [dialog, setDialog] = useState<Dialog | null>(null);
	const close = () => setDialog(null);
	return {
		openWith: (project) => setDialog({ kind: "openWith", project }),
		launchOptions: (project) => setDialog({ kind: "launch", project }),
		element: dialog ? (
			dialog.kind === "openWith" ? (
				<OpenWithModal key={dialog.project.path} project={dialog.project} onClose={close} />
			) : (
				<LaunchOptionsModal key={dialog.project.path} project={dialog.project} onClose={close} />
			)
		) : null,
	};
}
