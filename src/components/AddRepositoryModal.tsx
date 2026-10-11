import { useEffect, useState } from "react";
import { repositoryAdd, repositoryHeaders, repositoryPreview, repositoryUpdate } from "../api";
import type { CommunityRepository, HeaderInput, RepositoryPreview } from "../api";
import { errorMessage, isSensitiveHeader } from "../format";
import { Button } from "./Button";
import { Modal, Switch, TextField } from "./Controls";
import { Icon } from "./Icons";

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

export function AddRepositoryModal({
	onClose,
	onAdded,
	editing,
	initial,
}: {
	onClose: () => void;
	onAdded: (name: string) => void;
	editing?: CommunityRepository;
	initial?: { url: string; headers: HeaderInput[] };
}) {
	const [url, setUrl] = useState(editing?.url ?? initial?.url ?? "");
	const [state, setState] = useState<CheckState>({ phase: "idle" });
	const [useHeaders, setUseHeaders] = useState((initial?.headers.length ?? 0) > 0);
	const [headers, setHeaders] = useState<HeaderInput[]>(
		initial && initial.headers.length > 0 ? initial.headers : [{ name: "", value: "" }],
	);
	const [adding, setAdding] = useState(false);

	useEffect(() => {
		if (!editing) {
			return;
		}
		let cancelled = false;
		repositoryHeaders(editing.index, editing.id)
			.then((saved) => {
				if (!cancelled && saved.length > 0) {
					setHeaders(saved);
					setUseHeaders(true);
				}
			})
			.catch((error) => {
				if (!cancelled) {
					setAddError(errorMessage(error));
				}
			});
		return () => {
			cancelled = true;
		};
	}, [editing]);
	const [addError, setAddError] = useState<string | null>(null);

	const activeHeaders = useHeaders ? headers.filter((header) => header.name.trim() !== "") : [];

	const changeHeaders = (next: HeaderInput[]) => {
		setHeaders(next);
		setState({ phase: "idle" });
		setAddError(null);
	};

	const check = async () => {
		if (!url.trim() || state.phase === "checking") {
			return;
		}
		setState({ phase: "checking" });
		setAddError(null);
		try {
			setState({ phase: "result", preview: await repositoryPreview(url, activeHeaders, editing?.index) });
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
			if (editing) {
				await repositoryUpdate(editing.index, editing.id, success.url, activeHeaders);
			} else {
				await repositoryAdd(success.url, activeHeaders);
			}
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
			title={editing ? "Edit Repository" : "Add Repository"}
			onClose={onClose}
			actions={
				<>
					<Button onClick={onClose}>Cancel</Button>
					{success ? (
						<Button variant="primary" disabled={adding} onClick={() => void add()}>
							{editing ? "I Understand, Save Changes" : "I Understand, Add Repository"}
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
				{initial && !editing && (
					<div className="notice warn">
						This repository was opened from a link. Nothing has been requested yet. Review the address and headers below, then choose Check Repository.
					</div>
				)}
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
				<div className="header-toggle">
					<Switch
						label="Use authentication headers"
						checked={useHeaders}
						onChange={(value) => {
							setUseHeaders(value);
							setState({ phase: "idle" });
						}}
					/>
					<span>This repository needs authentication headers</span>
				</div>
				{useHeaders && (
					<div className="header-rows">
						{headers.map((header, index) => (
							<div key={index} className="header-row">
								<input
									type="text"
									value={header.name}
									placeholder="Header name"
									aria-label="Header name"
									onChange={(event) => changeHeaders(headers.map((item, at) => (at === index ? { ...item, name: event.target.value } : item)))}
								/>
								<input
									type={isSensitiveHeader(header.name) ? "password" : "text"}
									value={header.value}
									placeholder="Value"
									aria-label="Header value"
									autoComplete="off"
									onChange={(event) => changeHeaders(headers.map((item, at) => (at === index ? { ...item, value: event.target.value } : item)))}
								/>
								<button
									type="button"
									className="icon-btn remove"
									aria-label="Remove header"
									onClick={() => changeHeaders(headers.length > 1 ? headers.filter((_, at) => at !== index) : [{ name: "", value: "" }])}
								>
									<Icon name="minusCircle" size={18} />
								</button>
							</div>
						))}
						<div className="header-foot">
							<button type="button" className="link-btn" onClick={() => changeHeaders([...headers, { name: "", value: "" }])}>
								Add another header
							</button>
							<span className="muted">Saved on this computer in plain text, the same way VCC stores them.</span>
						</div>
					</div>
				)}
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
