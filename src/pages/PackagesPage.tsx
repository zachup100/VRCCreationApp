import { useCallback, useEffect, useState } from "react";
import { useApp } from "../AppContext";
import { repositoriesInfo, repositoryRemove, repositorySetHidden } from "../api";
import type { CommunityRepository, RepositoriesInfo } from "../api";
import { AddRepositoryModal } from "../components/AddRepositoryModal";
import { Button, IconButton } from "../components/Button";
import { Checkbox, ConfirmModal, Switch } from "../components/Controls";
import { HiddenPackages } from "../components/HiddenPackages";
import { ImportPackagesButton } from "../components/ImportPackagesButton";
import { errorMessage } from "../format";

function PrereleaseSetting() {
	const { settings, updateSettings } = useApp();
	return (
		<div className="settings-section">
			<h2>Pre-Release Packages</h2>
			<div className="card setting-list">
				<div className="setting">
					<div>
						<strong>Show pre-release packages</strong>
						<p className="muted">
							Pre-release versions are for testing and can be unstable. When on, they appear in package lists and count as the latest version.
						</p>
					</div>
					<Switch
						label="Show pre-release packages"
						checked={settings.showPrerelease}
						onChange={(checked) => void updateSettings({ showPrerelease: checked })}
					/>
				</div>
			</div>
		</div>
	);
}

function PackagesContent({ onOpenHidden }: { onOpenHidden: () => void }) {
	const { notify, repositoriesVersion } = useApp();
	const [info, setInfo] = useState<RepositoriesInfo | null>(null);
	const [loadError, setLoadError] = useState<string | null>(null);
	const [adding, setAdding] = useState(false);
	const [pendingRemove, setPendingRemove] = useState<CommunityRepository | null>(null);

	const load = useCallback(async () => {
		try {
			setInfo(await repositoriesInfo());
			setLoadError(null);
		} catch (error) {
			setLoadError(errorMessage(error));
		}
	}, []);

	useEffect(() => {
		void load();
	}, [load, repositoriesVersion]);

	const guard = async (task: () => Promise<void>) => {
		try {
			await task();
		} catch (error) {
			notify("error", errorMessage(error));
		}
		await load();
	};

	const toggle = (id: string, visible: boolean) => guard(() => repositorySetHidden(id, !visible));

	return (
		<section className="settings-section">
			<div className="section-head">
				<div>
					<h2>Installed Repositories</h2>
					<p className="muted">
						Official and curated repositories are included by default. You can add community repositories with Add Repository.
					</p>
				</div>
				<div className="button-stack">
					<ImportPackagesButton />
					<Button icon="plusCircle" variant="ghost" onClick={() => setAdding(true)}>
						Add Repository
					</Button>
				</div>
			</div>
			{loadError && <p className="field-error">{loadError}</p>}
			<div className="card table">
				<div className="table-head repos">
					<span className="th">Name</span>
					<span className="th">Author</span>
					<span className="th">URL</span>
					<span className="th" />
				</div>
				<div className="group-label">Built-In Repositories</div>
				{(info?.builtin ?? []).map((repo) => (
					<div key={repo.id} className="row repos">
						<span className="repo-name">
							<Checkbox checked={!repo.hidden} label={`Show ${repo.name} packages`} onChange={(value) => void toggle(repo.id, value)} />
							{repo.name}
						</span>
						<span />
						<span />
						<span />
					</div>
				))}
				<div className="group-label">Community Repositories</div>
				{info && info.community.length === 0 && <div className="repo-empty muted">No community repositories added.</div>}
				{(info?.community ?? []).map((repo) => (
					<div key={repo.id} className="row repos">
						<span className="repo-name">
							<Checkbox checked={!repo.hidden} label={`Show ${repo.name} packages`} onChange={(value) => void toggle(repo.id, value)} />
							<span className="ellipsis" title={repo.name}>
								{repo.name}
							</span>
						</span>
						<span className="ellipsis" title={repo.author ?? ""}>
							{repo.author ?? ""}
						</span>
						<span className="ellipsis muted" title={repo.url ?? ""}>
							{repo.url ?? ""}
						</span>
						<IconButton icon="minusCircle" label={`Remove ${repo.name}`} className="remove" onClick={() => setPendingRemove(repo)} />
					</div>
				))}
			</div>
			<div className="section-foot">
				<Button icon="package" onClick={onOpenHidden}>
					Hidden Packages
				</Button>
			</div>
			{adding && (
				<AddRepositoryModal
					onClose={() => setAdding(false)}
					onAdded={(name) => {
						notify("success", `Added repository ${name}`);
						void load();
					}}
				/>
			)}
			<PrereleaseSetting />
			{pendingRemove && (
				<ConfirmModal
					title="Remove Repository"
					confirmLabel="Remove Repository"
					danger
					onClose={() => setPendingRemove(null)}
					onConfirm={() =>
						void guard(async () => {
							await repositoryRemove(pendingRemove.index, pendingRemove.id);
							notify("info", `Removed repository ${pendingRemove.name}`);
						})
					}
				>
					<p>
						Remove <strong>{pendingRemove.name}</strong>? Packages already installed in your projects stay installed.
					</p>
				</ConfirmModal>
			)}
		</section>
	);
}

export function PackagesPage() {
	const [view, setView] = useState<"main" | "hidden">("main");
	if (view === "hidden") {
		return <HiddenPackages onClose={() => setView("main")} />;
	}
	return (
		<div className="page">
			<header className="page-header">
				<h1>Packages</h1>
			</header>
			<PackagesContent onOpenHidden={() => setView("hidden")} />
		</div>
	);
}
