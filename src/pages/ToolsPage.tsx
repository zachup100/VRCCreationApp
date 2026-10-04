import { useApp } from "../AppContext";
import { useActions } from "../actions";
import { Icon } from "../components/Icons";
import type { IconName } from "../components/Icons";

export function ToolsPage() {
	const { refreshEditors, notify, navigate } = useApp();
	const actions = useActions();
	const tools: { title: string; blurb: string; icon: IconName; run: () => void }[] = [
		{ title: "Add Existing Project", blurb: "Pick a Unity project folder to add to the list.", icon: "folderOpen", run: () => void actions.addExisting() },
		{ title: "Import Projects From VCC", blurb: "Copy your project list from the VRChat Creator Companion.", icon: "download", run: () => void actions.importVcc() },
		{
			title: "Import Packages From VCC",
			blurb: "Copy repositories and package folders from the VRChat Creator Companion.",
			icon: "package",
			run: () => void actions.importPackagesVcc(),
		},
		{
			title: "Rescan Unity Editors",
			blurb: "Look for newly installed Unity editors.",
			icon: "refresh",
			run: () => void refreshEditors().then(() => notify("info", "Unity editors rescanned")),
		},
		{ title: "Download Unity", blurb: "Open the official Unity download page.", icon: "unity", run: () => void actions.openLink("https://unity.com/download") },
		{ title: "Settings", blurb: "Themes, repositories and Unity editors.", icon: "settings", run: () => navigate({ page: "settings" }) },
	];
	return (
		<div className="page">
			<header className="page-header">
				<h1>Tools</h1>
			</header>
			<div className="grid">
				{tools.map((tool) => (
					<button key={tool.title} type="button" className="card link-card" onClick={tool.run}>
						<Icon name={tool.icon} size={28} />
						<strong>{tool.title}</strong>
						<span>{tool.blurb}</span>
					</button>
				))}
			</div>
		</div>
	);
}
