import { useCallback, useEffect, useMemo, useState } from "react";
import { useApp } from "../AppContext";
import { availablePackages, savePreset } from "../api";
import type { AvailablePackage, Preset, PresetKind } from "../api";
import { errorMessage } from "../format";
import { filterOptions, sourceLabels } from "../packageUi";
import { loadOnlySupported, saveOnlySupported } from "../theme";
import type { RepoFilter } from "../packageUi";
import { Button, IconButton } from "./Button";
import { Checkbox, EmptyState, SearchInput, Segmented, Select, TextField } from "./Controls";
import { Icon } from "./Icons";

const LATEST = "latest";

interface Row extends Omit<AvailablePackage, "source"> {
	source: AvailablePackage["source"] | "unavailable";
}

type LoadState = { status: "loading" } | { status: "error"; message: string } | { status: "ready"; rows: Row[] };

function sortRows(rows: Row[], selected: Set<string>): Row[] {
	return [...rows].sort((a, b) => {
		const byStatus = Number(selected.has(b.name)) - Number(selected.has(a.name));
		return byStatus !== 0 ? byStatus : a.displayName.toLowerCase().localeCompare(b.displayName.toLowerCase());
	});
}

export function PresetEditor({ preset, onClose }: { preset: Preset | null; onClose: () => void }) {
	const { notify } = useApp();
	const [name, setName] = useState(preset?.name ?? "");
	const [kind, setKind] = useState<PresetKind>(preset?.kind ?? "avatar");
	const [selected, setSelected] = useState<Set<string>>(() => new Set(preset?.packages.map((item) => item.name) ?? []));
	const [choices, setChoices] = useState<Record<string, string>>(() =>
		Object.fromEntries((preset?.packages ?? []).map((item) => [item.name, item.version ?? LATEST])),
	);
	const [load, setLoad] = useState<LoadState>({ status: "loading" });
	const [refreshing, setRefreshing] = useState(false);
	const [query, setQuery] = useState("");
	const [filter, setFilter] = useState<RepoFilter>("all");
	const [saving, setSaving] = useState(false);
	const [onlySupported, setOnlySupported] = useState(loadOnlySupported);
	const [error, setError] = useState<string | null>(null);

	const fetchRows = useCallback(
		async (refresh: boolean) => {
			setRefreshing(true);
			try {
				const available = await availablePackages(refresh);
				const known = new Set(available.map((item) => item.name));
				const missing: Row[] = (preset?.packages ?? [])
					.filter((item) => !known.has(item.name))
					.map((item) => ({
						name: item.name,
						displayName: item.name,
						versions: [],
						latestVersion: null,
						source: "unavailable",
						repository: null,
						requiresSdk: null,
						hidden: false,
					}));
				const initial = new Set(preset?.packages.map((item) => item.name) ?? []);
				setLoad({ status: "ready", rows: sortRows([...available, ...missing], initial) });
				if (refresh) {
					notify("success", "Package listings refreshed");
				}
			} catch (failure) {
				setLoad({ status: "error", message: errorMessage(failure) });
			} finally {
				setRefreshing(false);
			}
		},
		[preset, notify],
	);

	useEffect(() => {
		void fetchRows(false);
	}, [fetchRows]);

	const visible = useMemo(() => {
		if (load.status !== "ready") {
			return [];
		}
		const needle = query.trim().toLowerCase();
		return load.rows.filter(
			(row) =>
				(filter === "all" || row.source === filter || (row.source === "unavailable" && selected.has(row.name))) &&
				(!row.hidden || selected.has(row.name)) &&
				(!onlySupported || row.requiresSdk === null || row.requiresSdk === kind || selected.has(row.name)) &&
				(!needle || `${row.displayName} ${row.name}`.toLowerCase().includes(needle)),
		);
	}, [load, query, filter, selected, onlySupported, kind]);

	const toggle = (rowName: string) => {
		setSelected((current) => {
			const next = new Set(current);
			if (next.has(rowName)) {
				next.delete(rowName);
			} else {
				next.add(rowName);
			}
			return next;
		});
	};

	const skipped = load.status === "ready" ? load.rows.filter((row) => selected.has(row.name) && row.requiresSdk !== null && row.requiresSdk !== kind) : [];
	const trimmed = name.trim();
	const nameProblem = !trimmed ? "Enter a preset name" : trimmed.length > 60 ? "The name is too long" : null;
	const canSave = !nameProblem && selected.size > 0 && !saving;

	const save = async () => {
		setSaving(true);
		setError(null);
		try {
			const rows = load.status === "ready" ? load.rows : [];
			const packages = [...selected].map((packageName) => {
				const choice = choices[packageName] ?? LATEST;
				const known = preset?.packages.find((item) => item.name === packageName);
				const row = rows.find((item) => item.name === packageName);
				const displayName = row && row.source !== "unavailable" ? row.displayName : (known?.displayName ?? packageName);
				return { name: packageName, version: choice === LATEST ? null : choice, displayName };
			});
			await savePreset({ id: preset?.id ?? null, name: trimmed, kind, packages });
			notify("success", `Saved preset ${trimmed}`);
			onClose();
		} catch (failure) {
			setError(errorMessage(failure));
			setSaving(false);
		}
	};

	return (
		<div className="page">
			<header className="page-header">
				<IconButton icon="arrowLeft" label="Back to presets" disabled={saving} onClick={onClose} />
				<h1>{preset ? "Edit Preset" : "New Preset"}</h1>
			</header>

			<div className="form preset-form">
				<TextField label="Preset Name" value={name} placeholder="My avatar setup" valid={!nameProblem} onChange={(event) => setName(event.target.value)} />
				<div className="field">
					<span className="field-label">Template Type</span>
					<Segmented<PresetKind>
						label="Template type"
						value={kind}
						onChange={setKind}
						options={[
							{ value: "world", label: "World" },
							{ value: "avatar", label: "Avatar" },
						]}
					/>
				</div>
				{error && <p className="field-error">{error}</p>}
			</div>

			<header className="page-header sub">
				<h2>Packages</h2>
				<IconButton icon="refresh" label="Refresh packages" spinning={refreshing} disabled={refreshing} onClick={() => void fetchRows(true)} />
				<span className="spacer" />
				<label className="check-inline">
					<Checkbox
						checked={onlySupported}
						label={`Only show packages supported by ${kind} projects`}
						onChange={(value) => {
							setOnlySupported(value);
							saveOnlySupported(value);
						}}
					/>
					<span>Only show supported packages ({kind === "world" ? "World" : "Avatar"})</span>
				</label>
				<SearchInput value={query} onChange={setQuery} placeholder="Search Packages..." />
				<span className="label-inline">Selected Repos</span>
				<Select label="Selected repositories" value={filter} onChange={setFilter} options={filterOptions} />
			</header>
			<p className="muted preset-hint">
				Choose the packages to install in new projects. Pick Latest to always use the newest version at the time the project is created, or a specific version.
			</p>
			{skipped.length > 0 && (
				<div className="notice warn">
					<Icon name="alert" size={18} />
					<span>
						{skipped.length} selected {skipped.length === 1 ? "package needs" : "packages need"} the {kind === "world" ? "Avatars" : "Worlds"} SDK and will be skipped in {kind} projects:{" "}
						{skipped.map((row) => row.displayName).join(", ")}.
					</span>
				</div>
			)}

			<div className="card table">
				<div className="table-head packages">
					<span className="th">Name</span>
					<span className="th">Version</span>
					<span className="th">Latest Version</span>
					<span className="th">Source</span>
					<span className="th" />
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
						{visible.map((row) => {
							const isSelected = selected.has(row.name);
							const options = [{ value: LATEST, label: "Latest" }, ...row.versions.map((version) => ({ value: version, label: version }))];
							const current = choices[row.name] ?? LATEST;
							const hasChoice = options.some((option) => option.value === current);
							const otherSdk = row.requiresSdk !== null && row.requiresSdk !== kind;
							return (
								<div key={row.name} className={`row packages${isSelected ? " staged-install" : ""}${otherSdk ? " other-sdk" : ""}`}>
									<div className="pkg-name">
										<Icon name="package" size={22} />
										<div>
											<strong title={row.name}>
												{row.displayName}
												{otherSdk && <span className="tag warn-tag">{row.requiresSdk === "avatar" ? "Avatar SDK only" : "World SDK only"}</span>}
											</strong>
											{row.displayName !== row.name && <span className="path">{row.name}</span>}
										</div>
									</div>
									<div>
										<Select
											label={`Version of ${row.displayName}`}
											value={hasChoice ? current : LATEST}
											options={options}
											disabled={row.source === "unavailable"}
											onChange={(version) => setChoices((existing) => ({ ...existing, [row.name]: version }))}
										/>
									</div>
									<div className="latest">{row.latestVersion ?? <span className="muted">None</span>}</div>
									<div className="muted" title={row.repository ?? undefined}>
										{row.source === "unavailable" ? "Unavailable" : sourceLabels[row.source]}
									</div>
									<div className="pkg-action">
										<IconButton
											icon={isSelected ? "check" : "plusCircle"}
											label={isSelected ? `Remove ${row.displayName} from the preset` : `Add ${row.displayName} to the preset`}
											className={isSelected ? "add selected" : "add"}
											aria-pressed={isSelected}
											disabled={otherSdk && !isSelected}
											onClick={() => toggle(row.name)}
										/>
									</div>
								</div>
							);
						})}
					</div>
				)}
			</div>

			<div className="selection-bar" role="status">
				<span>
					{selected.size} {selected.size === 1 ? "package" : "packages"}
				</span>
				<Button variant="ghost" disabled={saving} onClick={onClose}>
					Cancel
				</Button>
				<Button variant="primary" disabled={!canSave} onClick={() => void save()}>
					{saving ? "Saving..." : "Save Preset"}
				</Button>
			</div>
		</div>
	);
}
