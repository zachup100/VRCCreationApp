import { useMemo, useState } from "react";
import { useApp } from "../AppContext";
import { useActions } from "../actions";
import type { ProjectInfo } from "../api";
import { Button, IconButton } from "../components/Button";
import { ConfirmModal, EmptyState, SearchInput } from "../components/Controls";
import { FolderDrop } from "../components/FolderDrop";
import { Icon } from "../components/Icons";
import { MoreMenu, SplitButton } from "../components/Menu";
import { useProjectDialogs } from "../components/ProjectDialogs";
import type { MenuItem } from "../components/Menu";
import { TypeBadge, VersionChip, canMigrateTo2022, hasEditor, isLegacy } from "../components/ProjectParts";
import { timeAgo } from "../format";

type SortKey = "favorite" | "name" | "type" | "unity" | "modified";

const columns: { key: SortKey; label: string; className: string }[] = [
	{ key: "favorite", label: "", className: "col-star" },
	{ key: "name", label: "Project Name", className: "col-name" },
	{ key: "type", label: "Project Type", className: "col-type" },
	{ key: "unity", label: "Unity", className: "col-unity chip-offset" },
	{ key: "modified", label: "Last Modified", className: "col-modified" },
];

function compare(a: ProjectInfo, b: ProjectInfo, key: SortKey): number {
	switch (key) {
		case "favorite":
			return Number(a.favorite) - Number(b.favorite);
		case "name":
			return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
		case "type":
			return a.category.localeCompare(b.category);
		case "unity":
			return (a.unityVersion ?? "").localeCompare(b.unityVersion ?? "", undefined, { numeric: true });
		case "modified":
			return a.lastModified - b.lastModified;
	}
}

function StarButton({ project, onToggle }: { project: ProjectInfo; onToggle: () => void }) {
	return (
		<IconButton
			icon="star"
			filled={project.favorite}
			label={project.favorite ? "Remove from favorites" : "Add to favorites"}
			className={project.favorite ? "star on" : "star"}
			onClick={onToggle}
			size={18}
		/>
	);
}

export function ProjectsPage() {
	const { packageOperation, projects, projectsLoading, rescanProjects, refreshEditors, editors, editorsLoading, navigate, view, setView } = useApp();
	const actions = useActions();
	const dialogs = useProjectDialogs();
	const [query, setQuery] = useState("");
	const [sort, setSort] = useState<{ key: SortKey; desc: boolean }>({ key: "modified", desc: true });
	const [pendingRemove, setPendingRemove] = useState<ProjectInfo | null>(null);

	const visible = useMemo(() => {
		const needle = query.trim().toLowerCase();
		const filtered = needle
			? projects.filter((project) => `${project.name} ${project.path}`.toLowerCase().includes(needle))
			: projects;
		const direction = sort.desc ? -1 : 1;
		return [...filtered].sort((a, b) => {
			if (sort.key !== "favorite" && a.favorite !== b.favorite) {
				return a.favorite ? -1 : 1;
			}
			return compare(a, b, sort.key) * direction;
		});
	}, [projects, query, sort]);

	const toggleSort = (key: SortKey) => {
		setSort((current) => (current.key === key ? { key, desc: !current.desc } : { key, desc: key === "modified" || key === "favorite" }));
	};

	const menuFor = (project: ProjectInfo): MenuItem[] => [
		...(isLegacy(project) ? [{ label: "Manage Project", icon: "package" as const, disabled: !project.exists, onSelect: () => navigate({ page: "manage", path: project.path }) }] : []),
		{ label: "Open With...", icon: "unity", disabled: !project.exists, onSelect: () => dialogs.openWith(project) },
		{ label: "Launch Options...", icon: "settings", onSelect: () => dialogs.launchOptions(project) },
		...(canMigrateTo2022(project)
			? [{ label: "Migrate to Unity 2022...", icon: "upload" as const, disabled: !project.exists, onSelect: () => dialogs.task(project, "unity2022") }]
			: []),
		{ label: "Duplicate...", icon: "copy", disabled: !project.exists, onSelect: () => dialogs.duplicate(project) },
		{ label: "Back Up Project...", icon: "download", disabled: !project.exists, onSelect: () => dialogs.task(project, "backup") },
		{ label: "Open Folder", icon: "folderOpen", disabled: !project.exists, onSelect: () => void actions.openFolder(project) },
		{ label: "Copy Path", icon: "copy", onSelect: () => void actions.copyPath(project) },
		{
			label: project.favorite ? "Remove Favorite" : "Add Favorite",
			icon: "star",
			onSelect: () => void actions.toggleFavorite(project),
		},
		{ label: "Remove From List", icon: "trash", danger: true, onSelect: () => setPendingRemove(project) },
	];

	const rowActions = (project: ProjectInfo) => {
		const installed = hasEditor(project, editors);
		const canOpen = project.exists && (editorsLoading || installed);
		return (
			<>
				<Button
					variant="primary"
					disabled={!canOpen}
					title={!project.exists ? "The project folder is missing" : !installed && !editorsLoading ? "Install this Unity version first" : undefined}
					onClick={() => void actions.open(project)}
				>
					Open Project
				</Button>
				{isLegacy(project) ? (
					<Button variant="success" icon="upload" disabled={!project.exists} onClick={() => dialogs.task(project, "vpm")}>
						Migrate Project
					</Button>
				) : (
					<Button disabled={!project.exists} onClick={() => navigate({ page: "manage", path: project.path })}>
						Manage Project
					</Button>
				)}
				<MoreMenu items={menuFor(project)} />
			</>
		);
	};

	return (
		<div className="page">
			<header className="page-header">
				<h1>Projects</h1>
				<IconButton
					icon="refresh"
					label="Refresh projects"
					spinning={projectsLoading}
					onClick={() => {
						void rescanProjects();
						void refreshEditors();
					}}
				/>
				<SearchInput value={query} onChange={setQuery} placeholder="Search Projects..." />
				<Button variant="ghost" icon={view === "list" ? "list" : "grid"} onClick={() => setView(view === "list" ? "grid" : "list")}>
					{view === "list" ? "List View" : "Grid View"}
				</Button>
				<SplitButton
					onClick={() => navigate({ page: "new" })}
					items={[
						{ label: "Add Existing Project", icon: "folderOpen", onSelect: () => void actions.addExisting() },
						{ label: "Import Projects From VCC", icon: "download", onSelect: () => void actions.importVcc() },
					]}
				>
					Create New Project
				</SplitButton>
			</header>

			{projects.length === 0 && !projectsLoading ? (
				<div className="card">
					<EmptyState
						icon="projects"
						title="No projects yet"
						actions={
							<>
								<Button variant="primary" icon="plus" onClick={() => navigate({ page: "new" })}>
									Create New Project
								</Button>
								<Button icon="folderOpen" onClick={() => void actions.addExisting()}>
									Add Existing Project
								</Button>
								<Button icon="download" onClick={() => void actions.importVcc()}>
									Import Projects From VCC
								</Button>
							</>
						}
					>
						Create a project, add an existing Unity project folder, or bring over your list from the VRChat Creator Companion.
					</EmptyState>
				</div>
			) : visible.length === 0 ? (
				<div className="card">
					<EmptyState icon="projects" title="No matching projects">
						Nothing matches &ldquo;{query}&rdquo;.
					</EmptyState>
				</div>
			) : view === "list" ? (
				<div className="card table">
					<div className="table-head projects">
						{columns.map((column) => (
							<button
								key={column.key}
								type="button"
								className={`th ${column.className}${sort.key === column.key ? " sorted" : ""}`}
								onClick={() => toggleSort(column.key)}
								aria-label={column.label || "Sort by favorite"}
							>
								{column.key === "favorite" ? <Icon name="star" size={16} filled /> : column.label}
								{sort.key === column.key && <Icon name={sort.desc ? "arrowDown" : "arrowUp"} size={14} />}
							</button>
						))}
						<span className="th col-actions" />
					</div>
					<div className="table-body">
						{visible.map((project) => (
							<div key={project.path} className={`row project-row${project.exists ? "" : " is-missing"}`}>
								<div className="col-star">
									<StarButton project={project} onToggle={() => void actions.toggleFavorite(project)} />
								</div>
								<div className="col-name">
									<strong title={project.name}>{project.name}</strong>
									<span className="path-chip compact" title={project.path}>
										{project.exists ? project.path : `${project.path} (missing)`}
									</span>
								</div>
								<div className="col-type">
									<TypeBadge category={project.category} />
								</div>
								<div className="col-unity">
									<VersionChip project={project} editors={editors} loaded={!editorsLoading} />
								</div>
								<div className="col-modified muted" title={new Date(project.lastModified).toLocaleString()}>
									{timeAgo(project.lastModified)}
								</div>
								<div className="col-actions">{rowActions(project)}</div>
							</div>
						))}
					</div>
				</div>
			) : (
				<div className="grid">
					{visible.map((project) => (
						<div key={project.path} className={`card project-card${project.exists ? "" : " is-missing"}`}>
							<div className="project-card-top">
								<TypeBadge category={project.category} />
								<StarButton project={project} onToggle={() => void actions.toggleFavorite(project)} />
							</div>
							<strong title={project.name}>{project.name}</strong>
							<span className="path-chip compact" title={project.path}>
								{project.exists ? project.path : `${project.path} (missing)`}
							</span>
							<div className="project-card-meta">
								<VersionChip project={project} editors={editors} loaded={!editorsLoading} />
								<span className="muted">{timeAgo(project.lastModified)}</span>
							</div>
							<div className="project-card-actions">{rowActions(project)}</div>
						</div>
					))}
				</div>
			)}

			<FolderDrop enabled={!packageOperation} onDrop={(paths) => void actions.addPaths(paths)} />
			{dialogs.element}
			{pendingRemove && (
				<ConfirmModal
					title="Remove Project"
					confirmLabel="Remove From List"
					danger
					onClose={() => setPendingRemove(null)}
					onConfirm={() => void actions.remove(pendingRemove)}
				>
					<p>
						Remove <strong>{pendingRemove.name}</strong> from the project list? The project files stay on disk and you can add it again later.
					</p>
				</ConfirmModal>
			)}
		</div>
	);
}
