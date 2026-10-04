import type { Preset } from "../api";

export function PresetPackages({ preset }: { preset: Preset }) {
	const count = preset.packages.length;
	const sorted = [...preset.packages].sort((a, b) =>
		(a.displayName ?? a.name).toLowerCase().localeCompare((b.displayName ?? b.name).toLowerCase()),
	);
	return (
		<div className="hover-anchor">
			<button type="button" className="count-link" aria-label={`Show the ${count} packages in ${preset.name}`}>
				{count} {count === 1 ? "package" : "packages"}
			</button>
			<div className="popover" role="tooltip">
				<ul>
					{sorted.map((item) => (
						<li key={item.name}>
							<span title={item.name}>{item.displayName ?? item.name}</span>
							<span className="muted">{item.version ?? "Latest"}</span>
						</li>
					))}
				</ul>
			</div>
		</div>
	);
}
