import { useCallback, useEffect, useMemo, useState } from "react";
import { useApp } from "../AppContext";
import { useActions } from "../actions";
import { projectApplyChanges, projectCleanupManifest, projectPackages, projectPlanChanges } from "../api";
import type { ChangeRequest, ChangeSummary, PackageRow } from "../api";
import { Button, IconButton } from "../components/Button";
import { Checkbox, ConfirmModal, EmptyState, Select, SearchInput } from "../components/Controls";
import { ReviewChangesModal } from "../components/ReviewChangesModal";
import { Icon } from "../components/Icons";
import { MoreMenu, SplitButton } from "../components/Menu";
import { useProjectDialogs } from "../components/ProjectDialogs";
import { TypeBadge, VersionChip, canMigrateTo2022, hasEditor, isLegacy } from "../components/ProjectParts";
import { errorMessage, timeAgo } from "../format";
import { filterOptions, sourceLabels } from "../packageUi";
import { loadOnlySupported, saveOnlySupported } from "../theme";
import type { RepoFilter } from "../packageUi";

type PackagesState =
	| { status: "loading" }
	| { status: "error"; message: string }
	| { status: "ready"; rows: PackageRow[]; missing: string[]; redundant: string[] };

type StagedChange = { type: "install"; version: string } | { type: "remove" };
type Staged = Record<string, StagedChange>;

function PackageTable({
	rows,
	query,
	filter,
	busy,
	hasSelection,
	staged,
	onStage,
	onReinstall,
	projectKind,
	onlySupported,
}: {
	rows: PackageRow[];
	query: string;
	filter: RepoFilter;
	busy: boolean;
	hasSelection: boolean;
	staged: Staged;
	onStage: (name: string, change: StagedChange | null) => void;
	onReinstall: (name: string) => void;
	projectKind: "world" | "avatar" | null;
	onlySupported: boolean;
}) {
	const [targets, setTargets] = useState<Record<string, string>>({});
	const visible = useMemo(() => {
		const needle = query.trim().toLowerCase();
		return rows.filter(
			(row) =>
				(filter === "all" || row.source === filter) &&
				(!onlySupported || projectKind === null || row.requiresSdk === null || row.requiresSdk === projectKind || row.installedVersion !== null) &&
				(!needle || `${row.displayName} ${row.name}`.toLowerCase().includes(needle)),
		);
	}, [rows, query, filter, onlySupported, projectKind]);

	if (visible.length === 0) {
		return (
			<EmptyState icon="package" title={rows.length === 0 ? "No packages available" : "No matching packages"}>
				{rows.length === 0
					? "Turn on a repository on the Packages page, or check your internet connection and refresh."
					: "Try a different search or repository filter."}
			</EmptyState>
		);
	}

	return (
		<div className="table-body">
			{visible.map((row) => {
				const installed = row.installedVersion !== null;
				const change = staged[row.name];
				const stagedVersion = change?.type === "install" ? change.version : null;
				const target = stagedVersion ?? targets[row.name] ?? row.installedVersion ?? row.latestVersion ?? "";
				const options = row.versions.map((version) => ({ value: version, label: version }));
				const rowClass = change ? (change.type === "remove" ? " staged-remove" : " staged-install") : "";
				return (
					<div key={row.name} className={`row packages with-menu${installed ? "" : " not-installed"}${rowClass}`}>
						<div className="pkg-name">
							<Icon name="package" size={22} />
							<div>
								<strong title={row.name}>
									{row.displayName}
									{projectKind !== null && row.requiresSdk !== null && row.requiresSdk !== projectKind && (
										<span className="tag warn-tag">{row.requiresSdk === "avatar" ? "Avatar SDK only" : "World SDK only"}</span>
									)}
								</strong>
								{row.displayName !== row.name && <span className="path">{row.name}</span>}
							</div>
						</div>
						<div>
							{options.length > 0 ? (
								<Select
									label={`Version of ${row.displayName}`}
									value={target}
									disabled={busy || change?.type === "remove"}
									options={options}
									placeholder={installed ? undefined : "Not Installed"}
									muted={!installed && !stagedVersion && !(row.name in targets)}
									onChange={(version) => {
										if (installed) {
											onStage(row.name, version === row.installedVersion ? null : { type: "install", version });
										} else {
											setTargets((current) => ({ ...current, [row.name]: version }));
											if (change) {
												onStage(row.name, { type: "install", version });
											}
										}
									}}
								/>
							) : (
								<span className="muted">{row.installedVersion ?? "Not Installed"}</span>
							)}
						</div>
						<div className="latest">
							{row.latestVersion ?? <span className="muted">None</span>}
							{row.hasUpdate && <span className="tag update">Update</span>}
						</div>
						<div className="muted" title={row.repository ?? undefined}>
							{sourceLabels[row.source]}
						</div>
						<div className="pkg-action">
							{installed ? (
								<IconButton
									icon={change?.type === "remove" ? "check" : "minusCircle"}
									label={change?.type === "remove" ? `Keep ${row.displayName}` : `Select ${row.displayName} for removal`}
									className={`remove${change?.type === "remove" ? " selected" : ""}`}
									aria-pressed={change?.type === "remove"}
									disabled={busy}
									onClick={() => onStage(row.name, change?.type === "remove" ? null : { type: "remove" })}
								/>
							) : (
								<IconButton
									icon={change?.type === "install" ? "check" : "plusCircle"}
									label={change?.type === "install" ? `Unselect ${row.displayName}` : `Select ${row.displayName} for install`}
									className={change?.type === "install" ? "add selected" : "add"}
									aria-pressed={change?.type === "install"}
									disabled={busy || !target}
									onClick={() => onStage(row.name, change?.type === "install" ? null : { type: "install", version: target })}
								/>
							)}
						</div>
						<div className="pkg-menu">
							{installed && (
								<MoreMenu
									label={`More actions for ${row.displayName}`}
									items={[{ label: "Reinstall", icon: "refresh", disabled: busy || hasSelection, onSelect: () => onReinstall(row.name) }]}
								/>
							)}
						</div>
					</div>
				);
			})}
		</div>
	);
}

export function ManageProjectPage({ path }: { path: string }) {
	const { projects, editors, editorsLoading, navigate, notify, setPackageOperation } = useApp();
	const actions = useActions();
	const dialogs = useProjectDialogs();
	const [query, setQuery] = useState("");
	const [filter, setFilter] = useState<RepoFilter>("all");
	const [packages, setPackages] = useState<PackagesState>({ status: "loading" });
	const [refreshing, setRefreshing] = useState(false);
	const [review, setReview] = useState<{ request: ChangeRequest; summary: ChangeSummary } | null>(null);
	const [working, setWorking] = useState(false);
	const [confirmCleanup, setConfirmCleanup] = useState(false);
	const [onlySupported, setOnlySupported] = useState(loadOnlySupported);
	const [staged, setStaged] = useState<Staged>({});
	const project = projects.find((item) => item.path === path);
	const projectExists = project?.exists ?? false;
	const projectKind = project?.category.endsWith("world") ? "world" : project?.category.endsWith("avatar") ? "avatar" : null;

	const load = useCallback(
		async (refresh: boolean) => {
			setRefreshing(true);
			try {
				const result = await projectPackages(path, refresh);
				setPackages({ status: "ready", rows: result.rows, missing: result.missing, redundant: result.redundant });
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
		setPackageOperation(working);
		return () => setPackageOperation(false);
	}, [working, setPackageOperation]);

	const stageChange = (name: string, change: StagedChange | null) => {
		setStaged((current) => {
			const next = { ...current };
			if (change) {
				next[name] = change;
			} else {
				delete next[name];
			}
			return next;
		});
	};

	const stagedCount = Object.keys(staged).length;

	const stagedRequest = (): ChangeRequest => ({
		installs: Object.entries(staged).flatMap(([name, change]) => (change.type === "install" ? [{ name, version: change.version }] : [])),
		removes: Object.entries(staged).flatMap(([name, change]) => (change.type === "remove" ? [name] : [])),
	});

	const plan = async (request: ChangeRequest) => {
		setWorking(true);
		try {
			setReview({ request, summary: await projectPlanChanges(path, request) });
		} catch (error) {
			notify("error", errorMessage(error));
		} finally {
			setWorking(false);
		}
	};

	const apply = async (acknowledged: boolean) => {
		if (!review) {
			return;
		}
		setWorking(true);
		try {
			await projectApplyChanges(path, { ...review.request, acknowledgeDependents: acknowledged });
			notify("success", "Package changes applied");
			setReview(null);
			setStaged({});
			await load(false);
		} catch (error) {
			notify("error", errorMessage(error));
		} finally {
			setWorking(false);
		}
	};

	const missing = packages.status === "ready" ? packages.missing : [];
	const redundant = packages.status === "ready" ? packages.redundant : [];

	const cleanup = async () => {
		setWorking(true);
		try {
			const removed = await projectCleanupManifest(path);
			notify("success", `Removed ${removed.length} redundant ${removed.length === 1 ? "entry" : "entries"}`);
			await load(false);
		} catch (error) {
			notify("error", errorMessage(error));
		} finally {
			setWorking(false);
		}
	};
	const updatable = packages.status === "ready" ? packages.rows.filter((row) => row.hasUpdate && row.latestVersion) : [];

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
				<IconButton icon="arrowLeft" label="Back to projects" disabled={working} onClick={() => navigate({ page: "projects" })} />
				<h1>{project.name}</h1>
				<span className="spacer" />
				<SplitButton
					disabled={working}
					onClick={() => void actions.open(project)}
					items={[
						{ label: "Open With...", icon: "unity", disabled: !project.exists, onSelect: () => dialogs.openWith(project) },
						{ label: "Launch Options...", icon: "settings", onSelect: () => dialogs.launchOptions(project) },
						...(isLegacy(project)
							? [{ label: "Migrate Project...", icon: "upload" as const, disabled: !project.exists, onSelect: () => dialogs.task(project, "vpm") }]
							: []),
						...(canMigrateTo2022(project)
							? [{ label: "Migrate to Unity 2022...", icon: "upload" as const, disabled: !project.exists, onSelect: () => dialogs.task(project, "unity2022") }]
							: []),
						{ label: "Back Up Project...", icon: "download", disabled: !project.exists, onSelect: () => dialogs.task(project, "backup") },
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
			{isLegacy(project) && project.exists && (
				<div className="notice warn">
					<span>This is a legacy project that uses the old VRChat SDK. Migrate it to the VPM package system to manage packages here.</span>
					<Button variant="success" icon="upload" disabled={working} onClick={() => dialogs.task(project, "vpm")}>
						Migrate Project
					</Button>
				</div>
			)}
			{missing.length > 0 && project.exists && (
				<div className="notice warn">
					<span>
						{missing.length} {missing.length === 1 ? "package is" : "packages are"} missing from this project's Packages folder ({missing.slice(0, 4).join(", ")}
						{missing.length > 4 ? ", and more" : ""}). Resolve to download and reinstall them.
					</span>
					<Button variant="success" icon="refresh" disabled={working || stagedCount > 0} onClick={() => void plan({ installs: [], removes: [], resolve: true })}>
						Resolve Packages
					</Button>
				</div>
			)}
			{redundant.length > 0 && project.exists && (
				<div className="notice warn">
					<span>
						{redundant.length} {redundant.length === 1 ? "entry" : "entries"} in this project's package list ({redundant.join(", ")}) {redundant.length === 1 ? "is" : "are"} already included in an installed package and
						{redundant.length === 1 ? " was" : " were"} never installed. They can be cleaned up safely.
					</span>
					<Button variant="success" icon="trash" disabled={working || stagedCount > 0} onClick={() => setConfirmCleanup(true)}>
						Clean Up
					</Button>
				</div>
			)}
			{confirmCleanup && (
				<ConfirmModal
					title="Clean Up Package List"
					confirmLabel="Clean Up"
					onClose={() => setConfirmCleanup(false)}
					onConfirm={() => void cleanup()}
				>
					<p>
						Remove {redundant.join(", ")} from this project's vpm-manifest.json? No package files are deleted, because these entries were never installed.
					</p>
				</ConfirmModal>
			)}
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
					disabled={refreshing || working || !project.exists}
					onClick={() => void load(true)}
				/>
				<span className="spacer" />
				{updatable.length > 0 && (
					<Button
						variant="success"
						icon="upload"
						disabled={working || stagedCount > 0}
						title={stagedCount > 0 ? "Submit or cancel your selection first" : undefined}
						onClick={() =>
							void plan({
								installs: updatable.map((row) => ({ name: row.name, version: row.latestVersion ?? "" })),
								removes: [],
							})
						}
					>
						Upgrade All ({updatable.length})
					</Button>
				)}
				{projectKind !== null && (
					<label className="check-inline">
						<Checkbox
							checked={onlySupported}
							label={`Only show packages supported by ${projectKind} projects`}
							onChange={(value) => {
								setOnlySupported(value);
								saveOnlySupported(value);
							}}
						/>
						<span>Only show supported packages ({projectKind === "world" ? "World" : "Avatar"})</span>
					</label>
				)}
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
				<div className="table-head packages with-menu">
					<span className="th">Name</span>
					<span className="th">Installed Version</span>
					<span className="th">Latest Version</span>
					<span className="th">Source</span>
					<span className="th" />
					<span className="th" />
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
				{packages.status === "ready" && (
					<PackageTable
						rows={packages.rows}
						query={query}
						filter={filter}
						busy={working}
						hasSelection={stagedCount > 0}
						staged={staged}
						onStage={stageChange}
						onReinstall={(name) => void plan({ installs: [], removes: [], reinstall: [name] })}
						projectKind={projectKind}
						onlySupported={onlySupported}
					/>
				)}
			</div>
			{stagedCount > 0 && !review && (
				<div className="selection-bar" role="status">
					<span>
						{stagedCount} package {stagedCount === 1 ? "change" : "changes"}
					</span>
					<Button variant="ghost" disabled={working} onClick={() => setStaged({})}>
						Cancel
					</Button>
					<Button variant="primary" disabled={working} onClick={() => void plan(stagedRequest())}>
						Submit
					</Button>
				</div>
			)}
			{dialogs.element}
			{review && (
				<ReviewChangesModal summary={review.summary} busy={working} onApply={(acknowledged) => void apply(acknowledged)}
					onClose={() => {
						setReview(null);
						setStaged({});
					}}
				/>
			)}
			<div className="page-foot">
				<Button variant="ghost" disabled={working} onClick={() => navigate({ page: "projects" })}>
					Back to projects
				</Button>
			</div>
		</div>
	);
}
