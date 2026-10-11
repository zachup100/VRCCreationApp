import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useApp } from "../AppContext";
import {
	deepLinkRegister,
	deepLinkStatus,
	deepLinkUnregister,
	getPrivateHostBlocking,
	setPrivateHostBlocking,
} from "../api";
import type { DeepLinkStatus } from "../api";
import { useActions } from "../actions";
import { Button, IconButton } from "../components/Button";
import { ConfirmModal, Select, Switch, Tabs } from "../components/Controls";
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
					title={version ? `VRCCreationApp ${version}` : "VRCCreationApp"}
					description="An open-source manager for VRChat projects and VPM packages, published by zachLabs and built on vrc-get. It is not made or endorsed by VRChat."
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

function DeepLinkRow() {
	const { notify } = useApp();
	const [status, setStatus] = useState<DeepLinkStatus | null>(null);
	const [working, setWorking] = useState(false);
	const [confirmTakeover, setConfirmTakeover] = useState(false);

	useEffect(() => {
		deepLinkStatus()
			.then(setStatus)
			.catch((error) => notify("error", errorMessage(error)));
	}, [notify]);

	const run = async (task: () => Promise<DeepLinkStatus>, message: string) => {
		setWorking(true);
		try {
			setStatus(await task());
			notify("success", message);
		} catch (error) {
			notify("error", errorMessage(error));
		} finally {
			setWorking(false);
		}
	};

	const registered = status?.registered ?? false;
	const description =
		status && !status.supported
			? "Opening vcc:// links is not available on this system."
			: "Let the Add to VCC buttons on websites open this app. It is off until you register it, and only one app on your computer can handle vcc:// links at a time. Links only open the Add Repository window, so nothing is added until you confirm.";

	return (
		<SettingRow
			icon="external"
			title="Add Repository Links"
			description={description}
			extra={
				status?.supported ? (
					<span className={`chip link-state${registered ? " on" : ""}`}>
						{registered ? "Registered to this app" : status.otherHandler ? "Handled by another app" : "Not registered"}
					</span>
				) : undefined
			}
		>
			<Button
				icon="external"
				disabled={!status?.supported || working}
				onClick={() => {
					if (registered) {
						void run(deepLinkUnregister, "Stopped handling vcc:// links");
					} else if (status?.otherHandler) {
						setConfirmTakeover(true);
					} else {
						void run(deepLinkRegister, "This app now handles vcc:// links");
					}
				}}
			>
				{registered ? "Unregister Deep Link" : "Register Deep Link"}
			</Button>
			{confirmTakeover && (
				<ConfirmModal
					title="Register Deep Link"
					confirmLabel="Register Deep Link"
					onClose={() => setConfirmTakeover(false)}
					onConfirm={() => void run(deepLinkRegister, "This app now handles vcc:// links")}
				>
					<p>
						Another app, probably the VRChat Creator Companion, currently handles vcc:// links. Registering makes this app handle them instead.
						Unregistering later removes the link handler completely, so the other app has to register itself again.
					</p>
				</ConfirmModal>
			)}
		</SettingRow>
	);
}

function PrivateHostsRow() {
	const { notify } = useApp();
	const [enabled, setEnabled] = useState<boolean | null>(null);

	useEffect(() => {
		getPrivateHostBlocking()
			.then(setEnabled)
			.catch((error) => notify("error", errorMessage(error)));
	}, [notify]);

	const change = async (value: boolean) => {
		try {
			setEnabled(await setPrivateHostBlocking(value));
		} catch (error) {
			notify("error", errorMessage(error));
		}
	};

	return (
		<SettingRow
			icon="world"
			title="Block Private Addresses"
			description="Refuse repository addresses that point to this computer or a local network, such as localhost, 192.168.x.x or 169.254.x.x. Turning this off only lasts until you close the app, and it is on again the next time it starts."
		>
			<Switch label="Block private addresses" checked={enabled ?? true} disabled={enabled === null} onChange={(value) => void change(value)} />
		</SettingRow>
	);
}

function GeneralTab() {
	const actions = useActions();
	return (
		<>
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
					<DeepLinkRow />
					<PrivateHostsRow />
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
