import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useApp } from "../AppContext";
import { useActions } from "../actions";
import { Button, IconButton } from "../components/Button";
import { EmptyState, Select, Tabs } from "../components/Controls";
import { ImportPackagesButton } from "../components/ImportPackagesButton";
import { LicensesModal } from "../components/LicensesModal";
import type { Tab } from "../components/Controls";
import { Icon } from "../components/Icons";
import type { IconName } from "../components/Icons";
import { errorMessage } from "../format";
import type { ThemeChoice, ViewMode } from "../theme";

type TabId = "general" | "appearance";

const tabs: Tab<TabId>[] = [
	{ id: "general", label: "General" },
	{ id: "appearance", label: "Appearance" },
];

function SettingRow({
	icon,
	title,
	description,
	extra,
	children,
}: {
	icon: IconName;
	title: string;
	description: string;
	extra?: ReactNode;
	children: ReactNode;
}) {
	return (
		<div className="setting-row">
			<span className="icon-tile">
				<Icon name={icon} size={22} />
			</span>
			<div className="setting-text">
				<strong>{title}</strong>
				<p className="muted">{description}</p>
				{extra}
			</div>
			<div className="setting-control">{children}</div>
		</div>
	);
}

function FolderSetting({
	icon,
	title,
	description,
	value,
	onChange,
}: {
	icon: IconName;
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
		<SettingRow
			icon={icon}
			title={title}
			description={description}
			extra={
				<span className={`path-chip${value ? "" : " empty"}`} title={value ?? ""}>
					{value ?? "Not set"}
				</span>
			}
		>
			<Button icon="folderOpen" onClick={() => void pick()}>
				Pick Folder
			</Button>
			{value && <IconButton icon="x" label={`Clear ${title}`} onClick={() => onChange("")} />}
		</SettingRow>
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
					icon="folder"
					title="Default Project Location"
					description="New projects are created here unless you pick another folder."
					value={settings.defaultProjectLocation}
					onChange={(folder) => void updateSettings({ defaultProjectLocation: folder })}
				/>
				<FolderSetting
					icon="backups"
					title="Backup Folder"
					description="Where project backups and the backups made before a migration are saved. When empty, the app's own Backups folder is used."
					value={settings.backupLocation}
					onChange={(folder) => void updateSettings({ backupLocation: folder })}
				/>
				<SettingRow
					icon="unity"
					title="Default Unity Launch Arguments"
					description="Added when opening any project. A project's own launch options take priority."
				>
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
				</SettingRow>
			</div>
		</section>
	);
}

function AboutSection() {
	const [version, setVersion] = useState("");
	const [showLicenses, setShowLicenses] = useState(false);
	useEffect(() => {
		getVersion()
			.then(setVersion)
			.catch(() => setVersion(""));
	}, []);
	return (
		<section className="settings-section">
			<h2>About</h2>
			<div className="card setting-list">
				<SettingRow
					icon="info"
					title={version ? `VRCreationApp ${version}` : "VRCreationApp"}
					description="An open-source manager for VRChat projects and VPM packages, built on vrc-get. It is not made or endorsed by VRChat."
				>
					<Button icon="book" onClick={() => setShowLicenses(true)}>
						Open Source Licenses
					</Button>
				</SettingRow>
			</div>
			{showLicenses && <LicensesModal onClose={() => setShowLicenses(false)} />}
		</section>
	);
}

function GeneralTab() {
	const { editors, editorsLoading, editorsError, refreshEditors, projects } = useApp();
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
						<ul className="editor-list">
							{editors.map((editor) => {
								const used = projects.filter((project) => project.unityVersion === editor.version).length;
								return (
									<li key={`${editor.version}-${editor.path}`} className="editor-row">
										<span className="editor-icon">
											<Icon name="unity" size={22} />
										</span>
										<div className="editor-info">
											<div className="editor-title">
												<strong>{editor.version}</strong>
												{editor.architecture && <span className="chip">{editor.architecture}</span>}
											</div>
											<span className="editor-path" title={editor.path}>
												{editor.path}
											</span>
										</div>
										<span className={`editor-usage${used === 0 ? " none" : ""}`}>
											{used === 0 ? "No projects" : `${used} ${used === 1 ? "project" : "projects"}`}
										</span>
									</li>
								);
							})}
						</ul>
					)}
				</div>
			</section>
			<ProjectSettings />
			<section className="settings-section">
				<div>
					<h2>VRChat Creator Companion</h2>
					<p className="muted">Bring your setup over from an existing VCC install. Your VCC data is only read, never changed.</p>
				</div>
				<div className="card setting-list">
					<SettingRow icon="download" title="Projects" description="Copy your project list, favorites and last modified dates.">
						<Button icon="download" onClick={() => void actions.importVcc()}>
							Import Projects From VCC
						</Button>
					</SettingRow>
					<SettingRow icon="package" title="Packages" description="Copy community repositories, including saved authentication headers, and local package folders.">
						<ImportPackagesButton />
					</SettingRow>
				</div>
			</section>
			<AboutSection />
		</>
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
						<p className="muted">Choose a dark, OLED black or light look, or follow your system.</p>
					</div>
					<Select<ThemeChoice>
						label="Theme"
						value={theme}
						onChange={setTheme}
						options={[
							{ value: "dark", label: "Dark" },
							{ value: "oled", label: "OLED" },
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
			{tab === "appearance" && <AppearanceTab />}
		</div>
	);
}
