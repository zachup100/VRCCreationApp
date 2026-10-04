import { useState } from "react";
import type { ChangeSummary } from "../api";
import { Button } from "./Button";
import { Icon } from "./Icons";
import { Modal } from "./Controls";
import { DownloadStatus } from "./DownloadStatus";

const installLabels: Record<ChangeSummary["installs"][number]["kind"], string> = {
	new: "Install",
	upgrade: "Upgrade",
	downgrade: "Downgrade",
	reinstall: "Reinstall",
};

const removeLabels: Record<ChangeSummary["removes"][number]["reason"], string> = {
	requested: "Remove",
	legacy: "Replaced legacy package",
	unused: "No longer needed",
};

export function ReviewChangesModal({
	summary,
	busy,
	onApply,
	onClose,
}: {
	summary: ChangeSummary;
	busy: boolean;
	onApply: () => void;
	onClose: () => void;
}) {
	const [showLegacy, setShowLegacy] = useState(false);
	const blocked = summary.conflicts.length > 0;
	const legacyCount = summary.legacyFiles.length + summary.legacyFolders.length;
	const empty = summary.installs.length === 0 && summary.removes.length === 0;

	return (
		<Modal
			title="Review Changes"
			onClose={busy ? () => undefined : onClose}
			actions={
				<>
					<Button onClick={onClose} disabled={busy}>
						Cancel
					</Button>
					<Button variant="primary" disabled={busy || blocked || empty} onClick={onApply}>
						{busy ? "Applying..." : "Apply Changes"}
					</Button>
				</>
			}
		>
			<DownloadStatus active={busy} />
			{empty && !blocked && <p>Nothing needs to change.</p>}

			{blocked && (
				<div className="notice warn change-block">
					<Icon name="alert" size={18} />
					<div>
						<strong>These changes conflict with the project and cannot be applied.</strong>
						<ul className="change-list">
							{summary.conflicts.map((conflict) => (
								<li key={conflict.package}>
									<strong>{conflict.package}</strong>
									{conflict.conflictsWithUnity && " does not support this Unity version."}
									{conflict.conflictingPackages.length > 0 && ` conflicts with ${conflict.conflictingPackages.join(", ")}.`}
									{conflict.unlockedNames.length > 0 && ` is already in the project as ${conflict.unlockedNames.join(", ")}.`}
								</li>
							))}
						</ul>
					</div>
				</div>
			)}

			{summary.installs.length > 0 && (
				<>
					<h3 className="modal-subtitle">Packages to install</h3>
					<ul className="change-list">
						{summary.installs.map((item) => (
							<li key={item.name} className="change-row">
								<span className={`tag kind-${item.kind}`}>{installLabels[item.kind]}</span>
								<strong>{item.displayName}</strong>
								<span className="muted">
									{item.previousVersion && item.previousVersion !== item.version
										? `${item.previousVersion} to ${item.version}`
										: item.version}
								</span>
							</li>
						))}
					</ul>
				</>
			)}

			{summary.removes.length > 0 && (
				<>
					<h3 className="modal-subtitle">Packages to remove</h3>
					<ul className="change-list">
						{summary.removes.map((item) => (
							<li key={item.name} className="change-row">
								<span className="tag kind-remove">{removeLabels[item.reason]}</span>
								<strong>{item.name}</strong>
							</li>
						))}
					</ul>
				</>
			)}

			{legacyCount > 0 && (
				<>
					<div className="notice warn change-block">
						<Icon name="alert" size={18} />
						<div>
							<strong>
								{legacyCount} legacy file{legacyCount === 1 ? "" : "s"} or folder{legacyCount === 1 ? "" : "s"} will be deleted from the project.
							</strong>{" "}
							<button type="button" className="link-btn" onClick={() => setShowLegacy((value) => !value)}>
								{showLegacy ? "Hide list" : "Show list"}
							</button>
						</div>
					</div>
					{showLegacy && (
						<ul className="change-list legacy-list">
							{[...summary.legacyFolders, ...summary.legacyFiles].map((path) => (
								<li key={path}>
									<code>{path}</code>
								</li>
							))}
						</ul>
					)}
				</>
			)}
		</Modal>
	);
}
