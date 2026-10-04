import { useApp } from "../AppContext";
import type { Route } from "../AppContext";
import { Icon } from "./Icons";
import type { IconName } from "./Icons";

type Page = Route["page"];

const top: { page: Page; label: string; icon: IconName }[] = [
	{ page: "projects", label: "Projects", icon: "projects" },
	{ page: "presets", label: "Presets", icon: "presets" },
	{ page: "backups", label: "Backups", icon: "backups" },
	{ page: "learn", label: "Learn", icon: "learn" },
	{ page: "tools", label: "Tools", icon: "tools" },
];

const bottom: { page: Page; label: string; icon: IconName }[] = [
	{ page: "logs", label: "Logs", icon: "logs" },
	{ page: "settings", label: "Settings", icon: "settings" },
];

function activeGroup(page: Page): Page {
	return page === "new" || page === "manage" ? "projects" : page;
}

function NavItem({ page, label, icon }: { page: Page; label: string; icon: IconName }) {
	const { route, navigate, packageOperation } = useApp();
	const active = activeGroup(route.page) === page;
	return (
		<button
			type="button"
			className={`nav-item${active ? " active" : ""}`}
			aria-current={active ? "page" : undefined}
			disabled={packageOperation && !active}
			title={packageOperation && !active ? "Wait for the package changes to finish" : undefined}
			onClick={() => navigate({ page } as Route)}
		>
			<Icon name={icon} size={24} />
			<span>{label}</span>
		</button>
	);
}

export function Sidebar() {
	return (
		<nav className="sidebar" aria-label="Main">
			<div className="sidebar-top">
				{top.map((item) => (
					<NavItem key={item.page} {...item} />
				))}
			</div>
			<div className="sidebar-bottom">
				{bottom.map((item) => (
					<NavItem key={item.page} {...item} />
				))}
			</div>
		</nav>
	);
}
