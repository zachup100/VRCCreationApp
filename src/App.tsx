import { useEffect } from "react";
import { AppProvider, useApp } from "./AppContext";
import { Sidebar } from "./components/Sidebar";
import { Toasts } from "./components/Toasts";
import { BackupsPage } from "./pages/BackupsPage";
import { LearnPage } from "./pages/LearnPage";
import { LogsPage } from "./pages/LogsPage";
import { ManageProjectPage } from "./pages/ManageProjectPage";
import { NewProjectPage } from "./pages/NewProjectPage";
import { PackagesPage } from "./pages/PackagesPage";
import { PresetsPage } from "./pages/PresetsPage";
import { ProjectsPage } from "./pages/ProjectsPage";
import { SettingsPage } from "./pages/SettingsPage";
import { ToolsPage } from "./pages/ToolsPage";
import { UnityPage } from "./pages/UnityPage";

const titles: Record<string, string> = {
	projects: "Projects",
	packages: "Packages",
	presets: "Presets",
	backups: "Backups",
	new: "New Project",
	manage: "Manage Project",
	unity: "Unity",
	learn: "Learn",
	tools: "Tools",
	logs: "Logs",
	settings: "Settings",
};

function Page() {
	const { route } = useApp();
	useEffect(() => {
		document.title = `${titles[route.page] ?? "VRCCreationApp"} - VRCCreationApp`;
	}, [route.page]);
	switch (route.page) {
		case "projects":
			return <ProjectsPage />;
		case "packages":
			return <PackagesPage />;
		case "presets":
			return <PresetsPage />;
		case "backups":
			return <BackupsPage />;
		case "new":
			return <NewProjectPage />;
		case "manage":
			return <ManageProjectPage path={route.path} />;
		case "unity":
			return <UnityPage />;
		case "learn":
			return <LearnPage />;
		case "tools":
			return <ToolsPage />;
		case "logs":
			return <LogsPage />;
		case "settings":
			return <SettingsPage />;
	}
}

export function App() {
	return (
		<AppProvider>
			<div className="app">
				<a className="skip-link" href="#main-content">
					Skip to content
				</a>
				<Sidebar />
				<main className="content" id="main-content" tabIndex={-1}>
					<Page />
				</main>
				<Toasts />
			</div>
		</AppProvider>
	);
}
