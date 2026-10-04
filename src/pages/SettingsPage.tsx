import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { useApp } from "../AppContext";
import { useActions } from "../actions";
import { Button } from "../components/Button";
import { Checkbox, EmptyState, Select, Tabs } from "../components/Controls";
import type { Tab } from "../components/Controls";
import { Icon } from "../components/Icons";
import type { ThemeChoice, ViewMode } from "../theme";

type TabId = "general" | "packages" | "appearance";

const tabs: Tab<TabId>[] = [
	{ id: "general", label: "General" },
	{ id: "packages", label: "Packages" },
	{ id: "appearance", label: "Appearance" },
];

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
			<section className="settings-section">
				<div className="section-head">
					<div>
						<h2>VRChat Creator Companion</h2>
						<p className="muted">Copy the project list from an existing VCC install. Your VCC data is only read, never changed.</p>
					</div>
					<Button icon="download" onClick={() => void actions.importVcc()}>
						Import From VCC
					</Button>
				</div>
			</section>
		</>
	);
}

function PackagesTab() {
	return (
		<section className="settings-section">
			<div className="section-head">
				<div>
					<h2>Installed Repositories</h2>
					<p className="muted">
						Official and curated repositories are included by default. Adding community repositories is coming soon.
					</p>
				</div>
				<Button icon="plusCircle" variant="ghost" disabled title="Not available yet">
					Add Repository
				</Button>
			</div>
			<div className="card table">
				<div className="table-head repos">
					<span className="th">Name</span>
					<span className="th">Author</span>
					<span className="th">URL</span>
				</div>
				<div className="group-label">Built-In Repositories</div>
				{["Official", "Curated", "Local User Packages"].map((name) => (
					<div key={name} className="row repos">
						<span className="repo-name">
							<Checkbox checked disabled label={name} />
							{name}
						</span>
						<span />
						<span />
					</div>
				))}
				<div className="group-label">Community Repositories</div>
				<div className="repo-empty muted">No community repositories added.</div>
			</div>
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
