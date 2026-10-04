import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { useApp } from "../AppContext";
import { createProject, listPresets, listTemplates } from "../api";
import type { Preset, TemplateInfo } from "../api";
import { Button } from "../components/Button";
import { TextField } from "../components/Controls";
import { Icon } from "../components/Icons";
import { errorMessage } from "../format";
import { loadLastLocation, saveLastLocation } from "../theme";

const invalidName = /[\\/:*?"<>|\u0000-\u001f]/;

function joinPath(location: string, name: string): string {
	const separator = location.includes("\\") && !location.includes("/") ? "\\" : "/";
	return `${location.replace(/[\\/]+$/, "")}${separator}${name}`;
}

export function NewProjectPage() {
	const { navigate, notify, editors, editorsLoading, refreshProjects, setPackageOperation, settings } = useApp();
	const [templates, setTemplates] = useState<TemplateInfo[]>([]);
	const [presets, setPresets] = useState<Preset[]>([]);
	const [presetId, setPresetId] = useState("");
	const [template, setTemplate] = useState("");
	const [name, setName] = useState("New Project");
	const [location, setLocation] = useState(() => settings.defaultProjectLocation ?? loadLastLocation());
	const [creating, setCreating] = useState(false);
	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		listTemplates()
			.then((items) => {
				setTemplates(items);
				setTemplate((current) => current || (items[0]?.id ?? ""));
			})
			.catch((failure) => setError(errorMessage(failure)));
	}, []);

	useEffect(() => {
		listPresets()
			.then(setPresets)
			.catch((failure) => setError(errorMessage(failure)));
	}, []);

	useEffect(() => {
		setPackageOperation(creating);
		return () => setPackageOperation(false);
	}, [creating, setPackageOperation]);

	const selected = templates.find((item) => item.id === template);
	const kindPresets = presets.filter((item) => item.kind === selected?.kind);
	const activePreset = kindPresets.find((item) => item.id === presetId);
	const unityMissing = selected !== undefined && !editorsLoading && !editors.some((editor) => editor.version === selected.unityVersion);

	const trimmed = name.trim();
	const nameError = !trimmed
		? "Enter a project name"
		: invalidName.test(trimmed)
			? "The name contains characters that are not allowed"
			: /[. ]$/.test(trimmed)
				? "The name cannot end with a dot or a space"
				: null;
	const canCreate = !nameError && location.trim() !== "" && template !== "" && !creating;

	const pickFolder = async () => {
		try {
			const folder = await open({ directory: true, multiple: false, title: "Choose where to create the project" });
			if (typeof folder === "string") {
				setLocation(folder);
				setError(null);
			}
		} catch (failure) {
			notify("error", errorMessage(failure));
		}
	};

	const create = async () => {
		setCreating(true);
		setError(null);
		try {
			const created = await createProject(template, trimmed, location.trim(), activePreset?.id);
			saveLastLocation(location.trim());
			notify("success", `Created ${created.project.name}`);
			if (created.warnings.length > 0) {
				notify("warning", `Some preset packages were skipped: ${created.warnings.join("; ")}`);
			}
			await refreshProjects();
			setPackageOperation(false);
			navigate({ page: "projects" });
		} catch (failure) {
			setError(errorMessage(failure));
			setCreating(false);
		}
	};

	return (
		<div className="page">
			<header className="page-header">
				<h1>New Project</h1>
			</header>

			<h2 className="section-title">Select template</h2>
			<div className="template-grid" role="radiogroup" aria-label="Project template">
				{templates.map((item) => (
					<button
						key={item.id}
						type="button"
						role="radio"
						aria-checked={template === item.id}
						disabled={creating}
						className={`card template${template === item.id ? " selected" : ""}`}
						onClick={() => setTemplate(item.id)}
					>
						<div className={`template-art ${item.kind}`}>
							<Icon name={item.kind === "world" ? "world" : "avatar"} size={44} />
							<span className="template-kind">{item.kind === "world" ? "Worlds" : "Avatars"}</span>
							<span className="template-year">{item.year}</span>
						</div>
						<div className="template-body">
							<strong>{item.title}</strong>
							<span>{item.blurb}</span>
						</div>
					</button>
				))}
			</div>

			<h2 className="section-title">Preset</h2>
			{kindPresets.length === 0 ? (
				<p className="muted preset-hint">
					No {selected?.kind ?? ""} presets yet.{" "}
					<button type="button" className="link-btn" disabled={creating} onClick={() => navigate({ page: "presets" })}>
						Create a preset
					</button>
				</p>
			) : (
				<div className="preset-picker" role="radiogroup" aria-label="Preset">
					{[{ id: "", name: "No preset", packages: [] as Preset["packages"] }, ...kindPresets].map((item) => (
						<button
							key={item.id || "none"}
							type="button"
							role="radio"
							aria-checked={(activePreset?.id ?? "") === item.id}
							disabled={creating}
							className={`radio-option${(activePreset?.id ?? "") === item.id ? " selected" : ""}`}
							onClick={() => setPresetId(item.id)}
						>
							<span className="radio-dot" />
							<strong>{item.name}</strong>
							{item.id !== "" && (
								<span className="muted">
									{item.packages.length} {item.packages.length === 1 ? "package" : "packages"}
								</span>
							)}
						</button>
					))}
				</div>
			)}

			<h2 className="section-title">Configure Project</h2>
			<div className="form">
				<TextField label="Project Name" value={name} disabled={creating} onChange={(event) => setName(event.target.value)} valid={!nameError} />
				{nameError && <p className="field-error">{nameError}</p>}
				<div className="field-row">
					<TextField
						label="Project Location"
						value={location}
						disabled={creating}
						placeholder="Choose a folder"
						onChange={(event) => setLocation(event.target.value)}
					/>
					<Button icon="folderOpen" variant="ghost" disabled={creating} onClick={() => void pickFolder()}>
						Pick New Folder
					</Button>
				</div>
				<p className="final-location">
					<span>Final project location:</span>
					<code>{location.trim() ? joinPath(location.trim(), trimmed || "...") : "Choose a location first"}</code>
				</p>
				{unityMissing && selected && (
					<div className="notice warn">
						<Icon name="alert" size={18} />
						<span>
							Unity {selected.unityVersion} is not installed. You can still create the project, but you need that version to open it.
						</span>
					</div>
				)}
				{creating && (
					<div className="notice">
						<Icon name="refresh" size={18} className="spin" />
						<span>Creating the project and installing the VRChat packages. The first time can take a minute while the packages download.</span>
					</div>
				)}
				{error && <p className="field-error">{error}</p>}
				<div className="form-actions">
					<Button variant="primary" icon="rocket" disabled={!canCreate} onClick={() => void create()}>
						{creating ? "Creating..." : "Create Project"}
					</Button>
					<Button variant="ghost" disabled={creating} onClick={() => navigate({ page: "projects" })}>
						Cancel
					</Button>
				</div>
			</div>
		</div>
	);
}
