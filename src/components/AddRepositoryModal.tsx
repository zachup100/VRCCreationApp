import { useState } from "react";
import { repositoryAdd, repositoryPreview } from "../api";
import type { RepositoryPreview } from "../api";
import { errorMessage } from "../format";
import { Button } from "./Button";
import { Modal, TextField } from "./Controls";

type CheckState =
	| { phase: "idle" }
	| { phase: "checking" }
	| { phase: "result"; preview: RepositoryPreview }
	| { phase: "failed"; message: string };

function ReadOnlyField({ label, value }: { label: string; value: string }) {
	return (
		<div className="kv">
			<span>{label}</span>
			<div className="kv-value">{value}</div>
		</div>
	);
}

export function AddRepositoryModal({ onClose, onAdded }: { onClose: () => void; onAdded: (name: string) => void }) {
	const [url, setUrl] = useState("");
	const [state, setState] = useState<CheckState>({ phase: "idle" });
	const [adding, setAdding] = useState(false);
	const [addError, setAddError] = useState<string | null>(null);

	const check = async () => {
		if (!url.trim() || state.phase === "checking") {
			return;
		}
		setState({ phase: "checking" });
		setAddError(null);
		try {
			setState({ phase: "result", preview: await repositoryPreview(url) });
		} catch (error) {
			setState({ phase: "failed", message: errorMessage(error) });
		}
	};

	const preview = state.phase === "result" ? state.preview : null;
	const success = preview?.status === "success" ? preview : null;

	const add = async () => {
		if (!success) {
			return;
		}
		setAdding(true);
		setAddError(null);
		try {
			await repositoryAdd(success.url);
			onAdded(success.name);
			onClose();
		} catch (error) {
			setAddError(errorMessage(error));
			setAdding(false);
		}
	};

	const problem =
		state.phase === "failed"
			? state.message
			: preview?.status === "badUrl"
				? "Enter a valid http or https address."
				: preview?.status === "downloadError"
					? preview.message
					: preview?.status === "duplicated"
						? preview.byUrl
							? `This address is already installed as "${preview.name}".`
							: `A repository with the same ID is already installed as "${preview.name}".`
						: addError;

	return (
		<Modal
			title="Add Repository"
			onClose={onClose}
			actions={
				<>
					<Button onClick={onClose}>Cancel</Button>
					{success ? (
						<Button variant="primary" disabled={adding} onClick={() => void add()}>
							I Understand, Add Repository
						</Button>
					) : (
						<Button variant="primary" disabled={!url.trim() || state.phase === "checking"} onClick={() => void check()}>
							{state.phase === "checking" ? "Checking..." : "Check Repository"}
						</Button>
					)}
				</>
			}
		>
			<form
				className="modal-form"
				onSubmit={(event) => {
					event.preventDefault();
					if (!success) {
						void check();
					}
				}}
			>
				<TextField
					label="Repository URL"
					value={url}
					placeholder="https://example.github.io/vpm/index.json"
					autoFocus
					onChange={(event) => {
						setUrl(event.target.value);
						setState({ phase: "idle" });
						setAddError(null);
					}}
				/>
				{problem && <p className="field-error">{problem}</p>}
			</form>

			{success && (
				<>
					<div className="kv-list">
						<ReadOnlyField label="Name" value={success.name} />
						<ReadOnlyField label="Author" value={success.author ?? "Unknown"} />
						<ReadOnlyField label="Id" value={success.id} />
						<ReadOnlyField label="Url" value={success.url} />
					</div>
					<h3 className="modal-subtitle">Included Packages</h3>
					{success.packages.length === 0 ? (
						<p>This repository does not list any packages.</p>
					) : (
						<ul className="package-chips">
							{success.packages.slice(0, 30).map((name) => (
								<li key={name}>{name}</li>
							))}
							{success.packages.length > 30 && <li className="muted">and {success.packages.length - 30} more</li>}
						</ul>
					)}
					<h3 className="modal-subtitle">Community Repositories Disclaimer</h3>
					<p className="disclaimer">
						Warning: only add repositories from people you trust. Packages from community repositories are made by third parties and can contain
						harmful code. Anything you install is at your own risk.
					</p>
				</>
			)}
		</Modal>
	);
}
