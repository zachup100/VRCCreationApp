import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { useApp } from "../AppContext";
import { Button } from "../components/Button";
import { TextField } from "../components/Controls";
import { Icon } from "../components/Icons";
import { errorMessage } from "../format";

type TemplateId = "world-2022" | "avatar-2022" | "avatar-2019" | "world-2019";

const templates: { id: TemplateId; title: string; blurb: string; kind: "world" | "avatar"; year: string }[] = [
	{ id: "world-2022", title: "Unity 2022 World Project", blurb: "A good starting point for a new VRChat World", kind: "world", year: "2022" },
	{ id: "avatar-2022", title: "Unity 2022 Avatar Project", blurb: "A good starting point for a new VRChat Avatar", kind: "avatar", year: "2022" },
	{ id: "avatar-2019", title: "Unity 2019 Avatar Project", blurb: "A good starting point for a new VRChat Avatar", kind: "avatar", year: "2019" },
	{ id: "world-2019", title: "Unity 2019 World Project", blurb: "A good starting point for a new VRChat World", kind: "world", year: "2019" },
];

const invalidName = /[\\/:*?"<>|]/;

function joinPath(location: string, name: string): string {
	const separator = location.includes("\\") && !location.includes("/") ? "\\" : "/";
	return `${location.replace(/[\\/]+$/, "")}${separator}${name}`;
}

export function NewProjectPage() {
	const { navigate, notify } = useApp();
	const [template, setTemplate] = useState<TemplateId>("world-2022");
	const [name, setName] = useState("New Project");
	const [location, setLocation] = useState("");

	const trimmed = name.trim();
	const nameError = !trimmed ? "Enter a project name" : invalidName.test(trimmed) ? "The name contains characters that are not allowed" : null;

	const pickFolder = async () => {
		try {
			const selected = await open({ directory: true, multiple: false, title: "Choose where to create the project" });
			if (typeof selected === "string") {
				setLocation(selected);
			}
		} catch (error) {
			notify("error", errorMessage(error));
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

			<h2 className="section-title">Configure Project</h2>
			<div className="form">
				<TextField label="Project Name" value={name} onChange={(event) => setName(event.target.value)} valid={!nameError} />
				{nameError && <p className="field-error">{nameError}</p>}
				<div className="field-row">
					<TextField
						label="Project Location"
						value={location}
						placeholder="Choose a folder"
						onChange={(event) => setLocation(event.target.value)}
					/>
					<Button icon="folderOpen" variant="ghost" onClick={() => void pickFolder()}>
						Pick New Folder
					</Button>
				</div>
				<p className="final-location">
					<span>Final project location:</span>
					<code>{location ? joinPath(location, trimmed || "...") : "Choose a location first"}</code>
				</p>
				<div className="notice">
					<Icon name="info" size={18} />
					<span>Creating projects from templates is not available yet. You can add an existing Unity project from the Projects page.</span>
				</div>
				<div className="form-actions">
					<Button variant="primary" icon="rocket" disabled title="Not available yet">
						Create Project
					</Button>
					<Button variant="ghost" onClick={() => navigate({ page: "projects" })}>
						Cancel
					</Button>
				</div>
			</div>
		</div>
	);
}
