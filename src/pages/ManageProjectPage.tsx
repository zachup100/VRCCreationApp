import { useCallback, useEffect, useMemo, useState } from "react";
import { useApp } from "../AppContext";
import { useActions } from "../actions";
import { projectPackages } from "../api";
import type { PackageRow, PackageSource } from "../api";
import { Button, IconButton } from "../components/Button";
import { EmptyState, Select, SearchInput } from "../components/Controls";
import { Icon } from "../components/Icons";
import { SplitButton } from "../components/Menu";
import { TypeBadge, VersionChip, hasEditor } from "../components/ProjectParts";
import { errorMessage, timeAgo } from "../format";

type RepoFilter = "all" | PackageSource;

const filterOptions: { value: RepoFilter; label: string }[] = [
	{ value: "all", label: "Multiple Repositories" },
	{ value: "official", label: "Official" },
	{ value: "curated", label: "Curated" },
	{ value: "community", label: "Community" },
	{ value: "local", label: "Local User Packages" },
];

const sourceLabels: Record<PackageSource, string> = {
	official: "Official",
	curated: "Curated",
	community: "Community",
	local: "Local",
};

type PackagesState =
	| { status: "loading" }
	| { status: "error"; message: string }
	| { status: "ready"; rows: PackageRow[] };

function PackageTable({ rows, query, filter }: { rows: PackageRow[]; query: string; filter: RepoFilter }) {
	const visible = useMemo(() => {
		const needle = query.trim().toLowerCase();
		return rows.filter(
			(row) =>
				(filter === "all" || row.source === filter) &&
				(!needle || `${row.displayName} ${row.name}`.toLowerCase().includes(needle)),
		);
	}, [rows, query, filter]);

	if (visible.length === 0) {
		return (
			<EmptyState icon="package" title={rows.length === 0 ? "No packages available" : "No matching packages"}>
				{rows.length === 0
					? "Turn on a repository in Settings, Packages, or check your internet connection and refresh."
					: "Try a different search or repository filter."}
			</EmptyState>
		);
	}

	return (
		<div className="table-body">
			{visible.map((row) => (
				<div key={row.name} className={`row packages${row.installedVersion ? "" : " not-installed"}`}>
					<div className="pkg-name">
						<Icon name="package" size={22} />
						<div>
							<strong title={row.name}>{row.displayName}</strong>
							{row.displayName !== row.name && <span className="path">{row.name}</span>}
						</div>
					</div>
					<div className={row.installedVersion ? "" : "muted"}>{row.installedVersion ?? "Not Installed"}</div>
					<div className="latest">
						{row.latestVersion ?? <span className="muted">None</span>}
						{row.hasUpdate && <span className="tag update">Update</span>}
					</div>
					<div className="muted" title={row.repository ?? undefined}>
						{sourceLabels[row.source]}
					</div>
				</div>
			))}
		</div>
	);
}

export function ManageProjectPage({ path }: { path: string }) {
	const { projects, editors, editorsLoading, navigate, notify } = useApp();
	const actions = useActions();
	const [query, setQuery] = useState("");
	const [filter, setFilter] = useState<RepoFilter>("all");
	const [packages, setPackages] = useState<PackagesState>({ status: "loading" });
	const [refreshing, setRefreshing] = useState(false);
	const project = projects.find((item) => item.path === path);
	const projectExists = project?.exists ?? false;

	const load = useCallback(
		async (refresh: boolean) => {
			setRefreshing(true);
			try {
				const result = await projectPackages(path, refresh);
				setPackages({ status: "ready", rows: result.rows });
				if (refresh) {
					notify("success", "Package listings refreshed");
				}
			} catch (error) {
				setPackages({ status: "error", message: errorMessage(error) });
			} finally {
				setRefreshing(false);
			}
		},
		[path, notify],
	);

	useEffect(() => {
		if (projectExists) {
			void load(false);
		}
	}, [projectExists, load]);

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
				<IconButton
					icon="refresh"
					label="Refresh packages"
					spinning={refreshing}
					disabled={refreshing || !project.exists}
					onClick={() => void load(true)}
				/>
				<span className="spacer" />
				<SearchInput value={query} onChange={setQuery} placeholder="Search Packages..." />
				<span className="label-inline">Selected Repos</span>
				<Select
					label="Selected repositories"
					value={filter}
					onChange={setFilter}
					options={filterOptions}
				/>
			</header>
			<div className="card table">
				<div className="table-head packages">
					<span className="th">Name</span>
					<span className="th">Installed Version</span>
					<span className="th">Latest Version</span>
					<span className="th">Source</span>
				</div>
				{packages.status === "loading" && (
					<EmptyState icon="package" title="Loading packages">
						The first load downloads the Official and Curated listings, so it can take a few seconds.
					</EmptyState>
				)}
				{packages.status === "error" && (
					<EmptyState
						icon="package"
						title="Could not load packages"
						actions={
							<Button icon="refresh" onClick={() => void load(true)}>
								Try again
							</Button>
						}
					>
						{packages.message}
					</EmptyState>
				)}
				{packages.status === "ready" && <PackageTable rows={packages.rows} query={query} filter={filter} />}
			</div>
			<div className="page-foot">
				<Button variant="ghost" onClick={() => navigate({ page: "projects" })}>
					Back to projects
				</Button>
			</div>
		</div>
	);
}
