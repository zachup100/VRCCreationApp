import { useCallback, useEffect, useMemo, useState } from "react";
import { useApp } from "../AppContext";
import { availablePackages, setPackagesHidden, showAllPackages } from "../api";
import type { AvailablePackage } from "../api";
import { errorMessage } from "../format";
import { filterOptions, sourceLabels } from "../packageUi";
import type { RepoFilter } from "../packageUi";
import { Button, IconButton } from "./Button";
import { Checkbox, EmptyState, SearchInput, Select } from "./Controls";
import { Icon } from "./Icons";

type LoadState = { status: "loading" } | { status: "error"; message: string } | { status: "ready"; rows: AvailablePackage[] };

export function HiddenPackages({ onClose }: { onClose: () => void }) {
	const { notify } = useApp();
	const [load, setLoad] = useState<LoadState>({ status: "loading" });
	const [refreshing, setRefreshing] = useState(false);
	const [query, setQuery] = useState("");
	const [filter, setFilter] = useState<RepoFilter>("all");

	const fetchRows = useCallback(
		async (refresh: boolean) => {
			setRefreshing(true);
			try {
				setLoad({ status: "ready", rows: await availablePackages(refresh) });
			} catch (error) {
				setLoad({ status: "error", message: errorMessage(error) });
			} finally {
				setRefreshing(false);
			}
		},
		[],
	);

	useEffect(() => {
		void fetchRows(false);
	}, [fetchRows]);

	const rows = load.status === "ready" ? load.rows : [];
	const hiddenCount = rows.filter((row) => row.hidden).length;

	const visible = useMemo(() => {
		const needle = query.trim().toLowerCase();
		return rows.filter((row) => (filter === "all" || row.source === filter) && (!needle || `${row.displayName} ${row.name}`.toLowerCase().includes(needle)));
	}, [rows, query, filter]);

	const toggle = async (name: string, shown: boolean) => {
		setLoad((current) =>
			current.status === "ready" ? { status: "ready", rows: current.rows.map((row) => (row.name === name ? { ...row, hidden: !shown } : row)) } : current,
		);
		try {
			await setPackagesHidden([name], !shown);
		} catch (error) {
			notify("error", errorMessage(error));
			await fetchRows(false);
		}
	};

	const showAll = async () => {
		try {
			await showAllPackages();
			setLoad((current) => (current.status === "ready" ? { status: "ready", rows: current.rows.map((row) => ({ ...row, hidden: false })) } : current));
		} catch (error) {
			notify("error", errorMessage(error));
		}
	};

	return (
		<div className="page">
			<header className="page-header">
				<IconButton icon="arrowLeft" label="Back to packages" onClick={onClose} />
				<h1>Hidden Packages</h1>
				<IconButton icon="refresh" label="Refresh packages" spinning={refreshing} disabled={refreshing} onClick={() => void fetchRows(true)} />
				<span className="spacer" />
				<SearchInput value={query} onChange={setQuery} placeholder="Search Packages..." />
				<span className="label-inline">Selected Repos</span>
				<Select label="Selected repositories" value={filter} onChange={setFilter} options={filterOptions} />
				<Button variant="ghost" icon="check" disabled={hiddenCount === 0} onClick={() => void showAll()}>
					Show All
				</Button>
			</header>
			<p className="muted preset-hint">
				Uncheck a package to hide it from the package lists on the Manage Project page and in presets. A package that is already installed in a project, or selected in a preset, stays visible there until you remove it.
				{load.status === "ready" && ` ${hiddenCount} of ${rows.length} packages are hidden.`}
			</p>

			<div className="card table">
				<div className="table-head hidden-pkgs">
					<span className="th" />
					<span className="th">Name</span>
					<span className="th">Latest Version</span>
					<span className="th">Source</span>
				</div>
				{load.status === "loading" && (
					<EmptyState icon="package" title="Loading packages">
						The first load downloads the Official and Curated listings, so it can take a few seconds.
					</EmptyState>
				)}
				{load.status === "error" && (
					<EmptyState
						icon="package"
						title="Could not load packages"
						actions={
							<Button icon="refresh" onClick={() => void fetchRows(true)}>
								Try again
							</Button>
						}
					>
						{load.message}
					</EmptyState>
				)}
				{load.status === "ready" && visible.length === 0 && (
					<EmptyState icon="package" title="No matching packages">
						Try a different search or repository filter.
					</EmptyState>
				)}
				{load.status === "ready" && visible.length > 0 && (
					<div className="table-body">
						{visible.map((row) => (
							<div key={row.name} className={`row hidden-pkgs${row.hidden ? " is-hidden" : ""}`}>
								<div>
									<Checkbox checked={!row.hidden} label={`Show ${row.displayName}`} onChange={(shown) => void toggle(row.name, shown)} />
								</div>
								<div className="pkg-name">
									<Icon name="package" size={22} />
									<div>
										<strong title={row.name}>{row.displayName}</strong>
										{row.displayName !== row.name && <span className="path">{row.name}</span>}
									</div>
								</div>
								<div className="latest">{row.latestVersion ?? <span className="muted">None</span>}</div>
								<div className="muted" title={row.repository ?? undefined}>
									{sourceLabels[row.source]}
								</div>
							</div>
						))}
					</div>
				)}
			</div>
		</div>
	);
}
