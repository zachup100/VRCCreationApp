import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { useApp } from "../AppContext";
import {
	addCustomEditor,
	installUnityEditor,
	installUnityHub,
	recommendedEditors,
	removeCustomEditor,
	unityHubStatus,
} from "../api";
import type { HubStatus, RecommendedEditor } from "../api";
import { useActions } from "../actions";
import { Button, IconButton } from "../components/Button";
import { Checkbox, ConfirmModal, EmptyState, Modal } from "../components/Controls";
import { Icon } from "../components/Icons";
import { UnityInstallStatus } from "../components/UnityInstallStatus";
import { errorMessage } from "../format";

export function UnityPage() {
	const { editors, editorsLoading, editorsError, refreshEditors, projects, notify, unityInstall } = useApp();
	const actions = useActions();
	const [hub, setHub] = useState<HubStatus | null>(null);
	const [recommended, setRecommended] = useState<RecommendedEditor[]>([]);
	const [addingEditor, setAddingEditor] = useState(false);
	const [pendingEditor, setPendingEditor] = useState<string | null>(null);
	const [installTarget, setInstallTarget] = useState<RecommendedEditor | null>(null);
	const [withAndroid, setWithAndroid] = useState(false);
	const [withIos, setWithIos] = useState(false);
	const [hubPrompt, setHubPrompt] = useState(false);
	const idle = unityInstall === null;

	useEffect(() => {
		recommendedEditors()
			.then(setRecommended)
			.catch((error) => notify("error", errorMessage(error)));
	}, [notify]);

	useEffect(() => {
		if (!idle) {
			return;
		}
		unityHubStatus()
			.then(setHub)
			.catch((error) => notify("error", errorMessage(error)));
	}, [idle, notify]);

	const addEditor = async () => {
		const isWindows = navigator.userAgent.includes("Windows");
		let selected: string | string[] | null;
		try {
			selected = await open({
				directory: false,
				multiple: false,
				title: "Select the Unity editor program",
				filters: isWindows ? [{ name: "Unity", extensions: ["exe"] }] : undefined,
			});
		} catch (error) {
			notify("error", errorMessage(error));
			return;
		}
		if (typeof selected !== "string") {
			return;
		}
		setPendingEditor(selected);
	};

	const confirmAddEditor = async (selected: string) => {
		setAddingEditor(true);
		try {
			const added = await addCustomEditor(selected);
			await refreshEditors();
			notify("success", `Added Unity ${added.version}`);
		} catch (error) {
			notify("error", errorMessage(error));
		} finally {
			setAddingEditor(false);
		}
	};

	const removeEditor = async (path: string, version: string) => {
		try {
			await removeCustomEditor(path);
			await refreshEditors();
			notify("info", `Removed Unity ${version} from the list`);
		} catch (error) {
			notify("error", errorMessage(error));
		}
	};

	const startEditorInstall = async (target: RecommendedEditor, android: boolean, ios: boolean) => {
		try {
			await installUnityEditor(target.version, android, ios);
		} catch (error) {
			notify("error", errorMessage(error));
		}
	};

	const startHubInstall = async () => {
		try {
			await installUnityHub();
		} catch (error) {
			notify("error", errorMessage(error));
		}
	};

	const installed = (version: string) => editors.some((editor) => editor.version === version);
	const hubMissing = hub !== null && !hub.found;

	return (
		<div className="page">
			<header className="page-header">
				<h1>Unity</h1>
				<span className="spacer" />
				<Button icon="plusCircle" variant="ghost" disabled={addingEditor} onClick={() => void addEditor()}>
					{addingEditor ? "Adding..." : "Add Editor"}
				</Button>
				<Button icon="refresh" variant="ghost" disabled={editorsLoading} onClick={() => void refreshEditors()}>
					Rescan
				</Button>
			</header>

			{unityInstall && <UnityInstallStatus install={unityInstall} />}

			{hubMissing && (
				<div className="notice warn">
					<Icon name="unity" size={20} />
					<span>Unity Hub was not found. It is needed to install editors from this page.</span>
					<span className="spacer" />
					{hub.canInstall && (
						<Button variant="primary" icon="download" disabled={!idle} onClick={() => setHubPrompt(true)}>
							Install Unity Hub
						</Button>
					)}
					<Button icon="external" onClick={() => void actions.openLink("https://unity.com/download")}>
						Open download page
					</Button>
				</div>
			)}

			<section className="settings-section">
				<div>
					<h2>Recommended Editors</h2>
					<p className="muted">Editor versions VRChat supports, installed through Unity Hub.</p>
				</div>
				<div className="card">
					<ul className="editor-list">
						{recommended.map((entry) => {
							const present = installed(entry.version);
							return (
								<li key={entry.version} className="editor-row">
									<span className="editor-icon">
										<Icon name="unity" size={22} />
									</span>
									<div className="editor-info">
										<div className="editor-title">
											<strong>{entry.version}</strong>
											<span className="chip">{entry.label}</span>
										</div>
										<span className="muted">{entry.note}</span>
									</div>
									{present ? (
										<span className="editor-usage">Installed</span>
									) : (
										<Button
											variant="primary"
											icon="download"
											disabled={!idle || hub === null || !hub.found}
											title={hubMissing ? "Install Unity Hub first" : undefined}
											onClick={() => {
												setWithAndroid(false);
												setWithIos(false);
												setInstallTarget(entry);
											}}
										>
											Install
										</Button>
									)}
								</li>
							);
						})}
					</ul>
				</div>
			</section>

			<section className="settings-section">
				<div>
					<h2>Installed Editors</h2>
					<p className="muted">
						Editors found through Unity Hub and its install folders, plus any you add yourself. Adding an editor may run it once to read its version.
					</p>
				</div>
				<div className="card">
					{editorsError ? (
						<EmptyState icon="unity" title="Could not read Unity installs">
							{editorsError}
						</EmptyState>
					) : editors.length === 0 && !editorsLoading ? (
						<EmptyState icon="unity" title="No Unity editors found">
							Install a recommended editor above, or add one you already have.
						</EmptyState>
					) : (
						<ul className="editor-list">
							{editors.map((editor) => {
								const used = projects.filter((project) => project.unityVersion === editor.version).length;
								return (
									<li key={`${editor.version}-${editor.path}`} className={`editor-row${editor.custom ? " custom" : ""}`}>
										<span className="editor-icon">
											<Icon name="unity" size={22} />
										</span>
										<div className="editor-info">
											<div className="editor-title">
												<strong>{editor.version}</strong>
												{editor.architecture && <span className="chip">{editor.architecture}</span>}
												{editor.custom && <span className="chip">custom</span>}
											</div>
											<span className="path-chip" title={editor.path}>
												{editor.path}
											</span>
										</div>
										<span className={`editor-usage${used === 0 ? " none" : ""}`}>
											{used === 0 ? "No projects" : `${used} ${used === 1 ? "project" : "projects"}`}
										</span>
										{editor.custom && (
											<IconButton
												icon="minusCircle"
												label={`Remove Unity ${editor.version} from the list`}
												className="remove"
												onClick={() => void removeEditor(editor.path, editor.version)}
											/>
										)}
									</li>
								);
							})}
						</ul>
					)}
				</div>
			</section>

			{pendingEditor && (
				<ConfirmModal
					title="Add Unity Editor"
					confirmLabel="Add Editor"
					onClose={() => setPendingEditor(null)}
					onConfirm={() => void confirmAddEditor(pendingEditor)}
				>
					<div className="kv-value">{pendingEditor}</div>
					<p>To read its version, this app may run this program once. Only add Unity editors that you trust.</p>
				</ConfirmModal>
			)}

			{installTarget && (
				<Modal
					title={`Install Unity ${installTarget.version}`}
					onClose={() => setInstallTarget(null)}
					actions={
						<>
							<Button onClick={() => setInstallTarget(null)}>Cancel</Button>
							<Button
								variant="primary"
								icon="download"
								onClick={() => {
									const target = installTarget;
									setInstallTarget(null);
									void startEditorInstall(target, withAndroid, withIos);
								}}
							>
								Install
							</Button>
						</>
					}
				>
					<p>Unity Hub will download and install Unity {installTarget.version}. This is several gigabytes and can take a long time.</p>
					<label className="check-row top">
						<Checkbox checked={withAndroid} onChange={setWithAndroid} label="Install Android Support" />
						<span className="option-text">
							<strong>Install Android Support</strong>
							<span className="muted">For Quest and other Android builds. Includes OpenJDK and the SDK and NDK tools.</span>
						</span>
					</label>
					<label className="check-row top">
						<Checkbox checked={withIos} onChange={setWithIos} label="Install iOS Support" />
						<span className="option-text">
							<strong>Install iOS Support</strong>
							<span className="muted">For iPhone and iPad builds.</span>
						</span>
					</label>
					<p className="modal-note muted">While the install runs, projects cannot be opened or changed. You can cancel it from this page, but a cancelled install may leave a partial editor behind.</p>
				</Modal>
			)}

			{hubPrompt && (
				<Modal
					title="Install Unity Hub"
					onClose={() => setHubPrompt(false)}
					actions={
						<>
							<Button onClick={() => setHubPrompt(false)}>Cancel</Button>
							<Button
								variant="primary"
								icon="download"
								onClick={() => {
									setHubPrompt(false);
									void startHubInstall();
								}}
							>
								Install Unity Hub
							</Button>
						</>
					}
				>
					<p>This will do the following:</p>
					<ol className="plain-steps">
						<li>Download the Unity Hub installer from Unity (about 190 MB).</li>
						<li>Check that its checksum matches Unity's download list and that it is signed by Unity Technologies. If either check fails, it is deleted and not run.</li>
						<li>Run the installer silently. Windows will ask for permission to make changes, and Unity Hub goes into its default location.</li>
						<li>Delete the installer, then look for Unity Hub again.</li>
					</ol>
					<p className="modal-note muted">While this runs, projects cannot be opened or changed. You can cancel during the download, but not once the installer has started.</p>
				</Modal>
			)}
		</div>
	);
}
