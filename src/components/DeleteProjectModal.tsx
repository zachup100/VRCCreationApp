import { useState } from "react";
import type { ProjectInfo } from "../api";
import { Button } from "./Button";
import { Checkbox, Modal } from "./Controls";

export function DeleteProjectModal({
	project,
	onDelete,
	onClose,
}: {
	project: ProjectInfo;
	onDelete: () => Promise<boolean>;
	onClose: () => void;
}) {
	const [understood, setUnderstood] = useState(false);
	const [deleting, setDeleting] = useState(false);

	const run = async () => {
		setDeleting(true);
		const done = await onDelete();
		if (done) {
			onClose();
		} else {
			setDeleting(false);
		}
	};

	return (
		<Modal
			title="Delete Project"
			onClose={() => {
				if (!deleting) {
					onClose();
				}
			}}
			actions={
				<>
					<Button disabled={deleting} onClick={onClose}>
						Cancel
					</Button>
					<Button variant="danger" disabled={!understood || deleting} onClick={() => void run()}>
						{deleting ? "Deleting..." : "Delete Project"}
					</Button>
				</>
			}
		>
			<p>
				Permanently delete <strong>{project.name}</strong> and every file inside its folder?
			</p>
			<span className="path-chip">{project.path}</span>
			<div className="notice warn change-block">
				<div>
					<strong>This cannot be undone.</strong>
					<p>The folder is deleted from your disk, not moved to the trash. Make a backup first if you might need it again.</p>
				</div>
			</div>
			<label className="check-row">
				<Checkbox checked={understood} onChange={setUnderstood} disabled={deleting} label="I understand this project will be permanently deleted" />
				<span>I understand that this project and all of its files will be permanently deleted.</span>
			</label>
		</Modal>
	);
}
