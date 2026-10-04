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

function Page() {
	const { route } = useApp();
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
				<Sidebar />
				<main className="content">
					<Page />
				</main>
				<Toasts />
			</div>
		</AppProvider>
	);
}
