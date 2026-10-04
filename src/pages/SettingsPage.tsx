import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";
import { useApp } from "../AppContext";
import { useActions } from "../actions";
import { repositoriesInfo, repositoryRemove, repositorySetHidden } from "../api";
import type { CommunityRepository, RepositoriesInfo } from "../api";
import { AddRepositoryModal } from "../components/AddRepositoryModal";
import { Button, IconButton } from "../components/Button";
import { Checkbox, ConfirmModal, EmptyState, Select, Switch, Tabs } from "../components/Controls";
import type { Tab } from "../components/Controls";
import { Icon } from "../components/Icons";
import { errorMessage } from "../format";
import type { ThemeChoice, ViewMode } from "../theme";

type TabId = "general" | "packages" | "appearance";

const tabs: Tab<TabId>[] = [
	{ id: "general", label: "General" },
	{ id: "packages", label: "Packages" },
	{ id: "appearance", label: "Appearance" },
];

function ImportPackagesButton() {
	const actions = useActions();
	const progress = actions.packageImportRunning;
	return (
		<Button icon="package" disabled={progress !== null} onClick={() => void actions.importPackagesVcc()}>
			{progress ? (progress.total > 0 ? `Importing ${progress.done}/${progress.total}...` : "Importing...") : "Import Packages From VCC"}
		</Button>
	);
}

function FolderSetting({
	title,
	description,
	value,
	onChange,
}: {
	title: string;
	description: string;
	value: string | null;
	onChange: (folder: string) => void;
}) {
	const { notify } = useApp();
	const pick = async () => {
		try {
			const folder = await open({ directory: true, multiple: false, title });
			if (typeof folder === "string") {
				onChange(folder);
			}
		} catch (error) {
			notify("error", errorMessage(error));
		}
	};
	return (
		<div className="setting">
			<div>
				<strong>{title}</strong>
				<p className="muted">{description}</p>
			</div>
			<div className="setting-path">
				<span className="path" title={value ?? ""}>
					{value ?? "Not set"}
				</span>
				<Button icon="folderOpen" onClick={() => void pick()}>
					Pick Folder
				</Button>
				{value && (
					<Button variant="ghost" onClick={() => onChange("")}>
						Clear
					</Button>
				)}
			</div>
		</div>
	);
}

function ProjectSettings() {
	const { settings, updateSettings } = useApp();
	const [args, setArgs] = useState(settings.defaultUnityArgs);
	useEffect(() => setArgs(settings.defaultUnityArgs), [settings.defaultUnityArgs]);
	return (
		<section className="settings-section">
			<h2>Projects</h2>
			<div className="card setting-list">
				<FolderSetting
					title="Default Project Location"
					description="New projects are created here unless you pick another folder."
					value={settings.defaultProjectLocation}
					onChange={(folder) => void updateSettings({ defaultProjectLocation: folder })}
				/>
				<FolderSetting
					title="Backup Folder"
					description="Where project backups will be stored. Used by project backups and migration once they are added."
					value={settings.backupLocation}
					onChange={(folder) => void updateSettings({ backupLocation: folder })}
				/>
				<div className="setting">
					<div>
						<strong>Default Unity Launch Arguments</strong>
						<p className="muted">Added when opening any project. A project&apos;s own launch options take priority.</p>
					</div>
					<form
						className="setting-args"
						onSubmit={(event) => {
							event.preventDefault();
							void updateSettings({ defaultUnityArgs: args });
						}}
					>
						<input
							type="text"
							value={args}
							placeholder="-vrcSdkLogging"
							aria-label="Default Unity launch arguments"
							onChange={(event) => setArgs(event.target.value)}
						/>
						<Button type="submit" disabled={args.trim() === settings.defaultUnityArgs}>
							Save
						</Button>
					</form>
				</div>
			</div>
		</section>
	);
}

function GeneralTab() {
	const { editors, editorsLoading, editorsError, refreshEditors } = useApp();
	const actions = useActions();
	return (
		<>
			<section className="settings-section">
				<div className="section-head">
					<div>
						<h2>Unity Editors</h2>
						<p className="muted">Editors found through Unity Hub and its install folders.</p>
					</div>
					<Button icon="refresh" variant="ghost" disabled={editorsLoading} onClick={() => void refreshEditors()}>
						Rescan
					</Button>
				</div>
				<div className="card">
					{editorsError ? (
						<EmptyState icon="unity" title="Could not read Unity installs">
							{editorsError}
						</EmptyState>
					) : editors.length === 0 && !editorsLoading ? (
						<EmptyState
							icon="unity"
							title="No Unity editors found"
							actions={
								<Button variant="primary" icon="external" onClick={() => void actions.openLink("https://unity.com/download")}>
									Open Unity download page
								</Button>
							}
						>
							Install Unity Hub and a Unity editor, then rescan.
						</EmptyState>
					) : (
						<ul className="plain-list">
							{editors.map((editor) => (
								<li key={`${editor.version}-${editor.path}`}>
									<Icon name="unity" size={22} />
									<div>
										<strong>{editor.version}</strong>
										{editor.architecture && <span className="tag">{editor.architecture}</span>}
										<span className="path">{editor.path}</span>
									</div>
								</li>
							))}
						</ul>
					)}
				</div>
			</section>
			<ProjectSettings />
			<section className="settings-section">
				<div className="section-head">
					<div>
						<h2>VRChat Creator Companion</h2>
						<p className="muted">Copy your projects, repositories and package folders from an existing VCC install. Your VCC data is only read, never changed.</p>
					</div>
					<div className="button-stack">
						<Button icon="download" onClick={() => void actions.importVcc()}>
							Import Projects From VCC
						</Button>
						<ImportPackagesButton />
					</div>
				</div>
			</section>
		</>
	);
}

function PrereleaseSetting() {
	const { settings, updateSettings } = useApp();
	return (
		<div className="settings-section">
			<h2>Pre-Release Packages</h2>
			<div className="card setting-list">
				<div className="setting">
					<div>
						<strong>Show pre-release packages</strong>
						<p className="muted">
							Pre-release versions are for testing and can be unstable. When on, they appear in package lists and count as the latest version.
						</p>
					</div>
					<Switch
						label="Show pre-release packages"
						checked={settings.showPrerelease}
						onChange={(checked) => void updateSettings({ showPrerelease: checked })}
					/>
				</div>
			</div>
		</div>
	);
}

function PackagesTab() {
	const { notify, repositoriesVersion } = useApp();
	const [info, setInfo] = useState<RepositoriesInfo | null>(null);
	const [loadError, setLoadError] = useState<string | null>(null);
	const [adding, setAdding] = useState(false);
	const [pendingRemove, setPendingRemove] = useState<CommunityRepository | null>(null);

	const load = useCallback(async () => {
		try {
			setInfo(await repositoriesInfo());
			setLoadError(null);
		} catch (error) {
			setLoadError(errorMessage(error));
		}
	}, []);

	useEffect(() => {
		void load();
	}, [load, repositoriesVersion]);

	const guard = async (task: () => Promise<void>) => {
		try {
			await task();
		} catch (error) {
			notify("error", errorMessage(error));
		}
		await load();
	};

	const toggle = (id: string, visible: boolean) => guard(() => repositorySetHidden(id, !visible));

	return (
		<section className="settings-section">
			<div className="section-head">
				<div>
					<h2>Installed Repositories</h2>
					<p className="muted">
						Official and curated repositories are included by default. You can add community repositories with Add Repository.
					</p>
				</div>
				<div className="button-stack">
					<ImportPackagesButton />
					<Button icon="plusCircle" variant="ghost" onClick={() => setAdding(true)}>
						Add Repository
					</Button>
				</div>
			</div>
			{loadError && <p className="field-error">{loadError}</p>}
			<div className="card table">
				<div className="table-head repos">
					<span className="th">Name</span>
					<span className="th">Author</span>
					<span className="th">URL</span>
					<span className="th" />
				</div>
				<div className="group-label">Built-In Repositories</div>
				{(info?.builtin ?? []).map((repo) => (
					<div key={repo.id} className="row repos">
						<span className="repo-name">
							<Checkbox checked={!repo.hidden} label={`Show ${repo.name} packages`} onChange={(value) => void toggle(repo.id, value)} />
							{repo.name}
						</span>
						<span />
						<span />
						<span />
					</div>
				))}
				<div className="group-label">Community Repositories</div>
				{info && info.community.length === 0 && <div className="repo-empty muted">No community repositories added.</div>}
				{(info?.community ?? []).map((repo) => (
					<div key={repo.id} className="row repos">
						<span className="repo-name">
							<Checkbox checked={!repo.hidden} label={`Show ${repo.name} packages`} onChange={(value) => void toggle(repo.id, value)} />
							<span className="ellipsis" title={repo.name}>
								{repo.name}
							</span>
						</span>
						<span className="ellipsis" title={repo.author ?? ""}>
							{repo.author ?? ""}
						</span>
						<span className="ellipsis muted" title={repo.url ?? ""}>
							{repo.url ?? ""}
						</span>
						<IconButton icon="minusCircle" label={`Remove ${repo.name}`} className="remove" onClick={() => setPendingRemove(repo)} />
					</div>
				))}
			</div>
			{adding && (
				<AddRepositoryModal
					onClose={() => setAdding(false)}
					onAdded={(name) => {
						notify("success", `Added repository ${name}`);
						void load();
					}}
				/>
			)}
			<PrereleaseSetting />
			{pendingRemove && (
				<ConfirmModal
					title="Remove Repository"
					confirmLabel="Remove Repository"
					danger
					onClose={() => setPendingRemove(null)}
					onConfirm={() =>
						void guard(async () => {
							await repositoryRemove(pendingRemove.index, pendingRemove.id);
							notify("info", `Removed repository ${pendingRemove.name}`);
						})
					}
				>
					<p>
						Remove <strong>{pendingRemove.name}</strong>? Packages already installed in your projects stay installed.
					</p>
				</ConfirmModal>
			)}
		</section>
	);
}

function AppearanceTab() {
	const { theme, setTheme, view, setView } = useApp();
	return (
		<section className="settings-section">
			<h2>Appearance</h2>
			<div className="card setting-list">
				<div className="setting">
					<div>
						<strong>Theme</strong>
						<p className="muted">Choose a dark or light look, or follow your system.</p>
					</div>
					<Select<ThemeChoice>
						label="Theme"
						value={theme}
						onChange={setTheme}
						options={[
							{ value: "dark", label: "Dark" },
							{ value: "light", label: "Light" },
							{ value: "system", label: "Follow system" },
						]}
					/>
				</div>
				<div className="setting">
					<div>
						<strong>Project view</strong>
						<p className="muted">How projects are shown on the Projects page.</p>
					</div>
					<Select<ViewMode>
						label="Project view"
						value={view}
						onChange={setView}
						options={[
							{ value: "list", label: "List View" },
							{ value: "grid", label: "Grid View" },
						]}
					/>
				</div>
			</div>
		</section>
	);
}

export function SettingsPage() {
	const [tab, setTab] = useState<TabId>("general");
	const [version, setVersion] = useState("");
	useEffect(() => {
		getVersion()
			.then((value) => setVersion(`v${value}`))
			.catch(() => setVersion(""));
	}, []);
	return (
		<div className="page">
			<header className="page-header">
				<h1>Settings</h1>
				<span className="spacer" />
				<span className="muted">{version}</span>
			</header>
			<Tabs tabs={tabs} active={tab} onChange={setTab} />
			{tab === "general" && <GeneralTab />}
			{tab === "packages" && <PackagesTab />}
			{tab === "appearance" && <AppearanceTab />}
		</div>
	);
}
