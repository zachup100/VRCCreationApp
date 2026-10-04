import type { ProjectCategory, ProjectInfo, UnityEditorInfo } from "../api";
import { Icon } from "./Icons";

const labels: Record<ProjectCategory, string> = {
	avatar: "Avatar",
	world: "World",
	"legacy-avatar": "Avatar",
	"legacy-world": "World",
	other: "Unknown",
};

export function TypeBadge({ category }: { category: ProjectCategory }) {
	const legacy = category.startsWith("legacy");
	return (
		<div className="type-badge">
			<Icon name={category.endsWith("avatar") ? "avatar" : category.endsWith("world") ? "world" : "unity"} size={22} />
			<div className="type-text">
				{legacy && <span className="legacy">Legacy</span>}
				<span>{labels[category]}</span>
			</div>
		</div>
	);
}

export function hasEditor(project: ProjectInfo, editors: UnityEditorInfo[]): boolean {
	return project.unityVersion !== null && editors.some((editor) => editor.version === project.unityVersion);
}

export function VersionChip({ project, editors, loaded }: { project: ProjectInfo; editors: UnityEditorInfo[]; loaded: boolean }) {
	if (!project.unityVersion) {
		return <span className="version-chip muted">Unknown</span>;
	}
	const installed = hasEditor(project, editors);
	return (
		<span
			className={`version-chip${loaded && !installed ? " missing" : ""}`}
			title={loaded && !installed ? "This Unity version is not installed" : "Installed Unity version"}
		>
			{loaded && !installed && <Icon name="alert" size={14} />}
			{project.unityVersion}
		</span>
	);
}
