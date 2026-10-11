import { UnityInstallNotice } from "../components/UnityInstallStatus";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";
import { useApp } from "../AppContext";
import { deleteBackup, listBackups, openBackupFolder } from "../api";
import type { BackupInfo } from "../api";
import { Button, IconButton } from "../components/Button";
import { ConfirmModal, EmptyState } from "../components/Controls";
import { TypeBadge } from "../components/ProjectParts";
import { RestoreBackupModal } from "../components/RestoreBackupModal";
import { errorMessage, timeAgo } from "../format";

export function BackupsPage() {
	const { settings, notify } = useApp();
	const [backups, setBackups] = useState<BackupInfo[]>([]);
	const [loading, setLoading] = useState(true);
	const [pendingDelete, setPendingDelete] = useState<BackupInfo | null>(null);
	const [restoring, setRestoring] = useState<{ backup: BackupInfo; location: string } | null>(null);

	const load = useCallback(async () => {
		setLoading(true);
		try {
			setBackups(await listBackups());
		} catch (error) {
			notify("error", errorMessage(error));
		} finally {
			setLoading(false);
		}
	}, [notify]);

	useEffect(() => {
		void load();
	}, [load]);

	const restore = async (backup: BackupInfo) => {
		let location = settings.defaultProjectLocation;
		if (!location) {
			try {
				const picked = await open({ directory: true, multiple: false, title: "Choose where to restore the project" });
				if (typeof picked !== "string") {
					return;
				}
				location = picked;
			} catch (error) {
				notify("error", errorMessage(error));
				return;
			}
		}
		setRestoring({ backup, location });
	};

	const remove = async (backup: BackupInfo) => {
		try {
			await deleteBackup(backup.fileName);
			notify("info", `Deleted the backup of ${backup.projectName}`);
		} catch (error) {
			notify("error", errorMessage(error));
		}
		await load();
	};

	return (
		<div className="page">
			<header className="page-header">
				<h1>Backups</h1>
				<IconButton icon="refresh" label="Refresh backups" spinning={loading} onClick={() => void load()} />
				<span className="spacer" />
				<Button variant="ghost" icon="folderOpen" onClick={() => void openBackupFolder().catch((error) => notify("error", errorMessage(error)))}>
					Open Backup Folder
				</Button>
			</header>
			<UnityInstallNotice />

			{backups.length === 0 && !loading ? (
				<div className="card">
					<EmptyState icon="backups" title="No backups yet">
						Backups made from the project menu, or automatically before a migration, show up here. Only backups made by this app are listed.
					</EmptyState>
				</div>
			) : (
				<div className="card table">
					<div className="table-head backups">
						<span className="th">Project Name</span>
						<span className="th">Project Type</span>
						<span className="th chip-offset">Unity</span>
						<span className="th">Created</span>
						<span className="th" />
					</div>
					<div className="table-body">
						{backups.map((backup) => (
							<div key={backup.fileName} className="row backups">
								<div className="col-name">
									<strong title={backup.projectName}>{backup.projectName}</strong>
									<span className="path-chip compact" title={backup.fileName}>
										{backup.fileName}
									</span>
								</div>
								<div>
									<TypeBadge category={backup.category} />
								</div>
								<div>
									<span className={`version-chip${backup.unityVersion ? "" : " muted"}`}>{backup.unityVersion ?? "Unknown"}</span>
								</div>
								<div className="created">
									<span>{new Date(backup.createdAt).toLocaleString()}</span>
									<span className="muted">{timeAgo(backup.createdAt)}</span>
								</div>
								<div className="col-actions">
									<Button variant="primary" onClick={() => void restore(backup)}>
										Restore
									</Button>
									<Button variant="danger" onClick={() => setPendingDelete(backup)}>
										Delete
									</Button>
								</div>
							</div>
						))}
					</div>
				</div>
			)}

			{restoring && (
				<RestoreBackupModal
					key={restoring.backup.fileName}
					backup={restoring.backup}
					location={restoring.location}
					onClose={() => setRestoring(null)}
				/>
			)}
			{pendingDelete && (
				<ConfirmModal
					title="Delete Backup"
					confirmLabel="Delete Backup"
					danger
					onClose={() => setPendingDelete(null)}
					onConfirm={() => void remove(pendingDelete)}
				>
					<p>
						Permanently delete the backup of <strong>{pendingDelete.projectName}</strong> from {new Date(pendingDelete.createdAt).toLocaleString()}? This cannot be undone.
					</p>
				</ConfirmModal>
			)}
		</div>
	);
}
