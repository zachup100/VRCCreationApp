import { useState } from "react";
import { useApp } from "../AppContext";
import { useActions } from "../actions";
import { Button, IconButton } from "../components/Button";
import { EmptyState, Select, SearchInput } from "../components/Controls";
import { SplitButton } from "../components/Menu";
import { TypeBadge, VersionChip, hasEditor } from "../components/ProjectParts";
import { timeAgo } from "../format";

export function ManageProjectPage({ path }: { path: string }) {
	const { projects, editors, editorsLoading, navigate } = useApp();
	const actions = useActions();
	const [query, setQuery] = useState("");
	const [repos, setRepos] = useState<"all">("all");
	const project = projects.find((item) => item.path === path);

	if (!project) {
		return (
			<div className="page">
				<header className="page-header">
					<IconButton icon="arrowLeft" label="Back to projects" onClick={() => navigate({ page: "projects" })} />
					<h1>Project not found</h1>
				</header>
				<div className="card">
					<EmptyState icon="projects" title="This project is no longer in the list" />
				</div>
			</div>
		);
	}

	const installed = hasEditor(project, editors);
	const canOpen = project.exists && (editorsLoading || installed);

	return (
		<div className="page">
			<header className="page-header">
				<IconButton icon="arrowLeft" label="Back to projects" onClick={() => navigate({ page: "projects" })} />
				<h1>{project.name}</h1>
				<span className="spacer" />
				<SplitButton
					onClick={() => void actions.open(project)}
					items={[
						{ label: "Open Folder", icon: "folderOpen", disabled: !project.exists, onSelect: () => void actions.openFolder(project) },
						{ label: "Copy Path", icon: "copy", onSelect: () => void actions.copyPath(project) },
					]}
				>
					Open Project
				</SplitButton>
			</header>
			<p className="located">
				<strong>Located At:</strong> <span className="path-inline">{project.path}</span>
				{!project.exists && <span className="tag danger">missing</span>}
			</p>
			{!canOpen && (
				<div className="notice warn">
					<span>
						{project.exists ? `Unity ${project.unityVersion ?? ""} is not installed, so this project cannot be opened yet.` : "The project folder could not be found."}
					</span>
				</div>
			)}

			<div className="stat-row">
				<div className="card stat">
					<span className="stat-label">Type</span>
					<TypeBadge category={project.category} />
				</div>
				<div className="card stat">
					<span className="stat-label">Unity</span>
					<VersionChip project={project} editors={editors} loaded={!editorsLoading} />
				</div>
				<div className="card stat">
					<span className="stat-label">Last Modified</span>
					<span>{timeAgo(project.lastModified)}</span>
				</div>
			</div>

			<header className="page-header sub">
				<h2>Manage Packages</h2>
				<IconButton icon="refresh" label="Refresh packages" disabled />
				<span className="spacer" />
				<SearchInput value={query} onChange={setQuery} placeholder="Search Packages..." />
				<span className="label-inline">Selected Repos</span>
				<Select
					label="Selected repositories"
					value={repos}
					onChange={setRepos}
					options={[{ value: "all", label: "Multiple Repositories" }]}
					disabled
				/>
			</header>
			<div className="card table">
				<div className="table-head packages">
					<span className="th">Name</span>
					<span className="th">Installed Version</span>
					<span className="th">Latest Version</span>
					<span className="th" />
				</div>
				<EmptyState icon="package" title="Package management is coming soon">
					Installing, upgrading and removing VPM packages for this project will appear here.
				</EmptyState>
			</div>
			<div className="page-foot">
				<Button variant="ghost" onClick={() => navigate({ page: "projects" })}>
					Back to projects
				</Button>
			</div>
		</div>
	);
}
