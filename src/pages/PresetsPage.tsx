import { useCallback, useEffect, useState } from "react";
import { useApp } from "../AppContext";
import { deletePreset, listPresets } from "../api";
import type { Preset } from "../api";
import { Button } from "../components/Button";
import { EmptyState } from "../components/Controls";
import { PresetEditor } from "../components/PresetEditor";
import { PresetPackages } from "../components/PresetPackages";
import { TypeBadge } from "../components/ProjectParts";
import { errorMessage } from "../format";

export function PresetsPage() {
	const { notify } = useApp();
	const [presets, setPresets] = useState<Preset[]>([]);
	const [loading, setLoading] = useState(true);
	const [editing, setEditing] = useState<{ preset: Preset | null } | null>(null);

	const load = useCallback(async () => {
		setLoading(true);
		try {
			setPresets(await listPresets());
		} catch (error) {
			notify("error", errorMessage(error));
		} finally {
			setLoading(false);
		}
	}, [notify]);

	useEffect(() => {
		void load();
	}, [load]);

	const remove = async (preset: Preset) => {
		try {
			await deletePreset(preset.id);
			notify("info", `Removed preset ${preset.name}`);
		} catch (error) {
			notify("error", errorMessage(error));
		}
		await load();
	};

	if (editing) {
		return (
			<PresetEditor
				key={editing.preset?.id ?? "new"}
				preset={editing.preset}
				onClose={() => {
					setEditing(null);
					void load();
				}}
			/>
		);
	}

	return (
		<div className="page">
			<header className="page-header">
				<h1>Presets</h1>
				<span className="spacer" />
				<Button variant="primary" icon="plus" onClick={() => setEditing({ preset: null })}>
					Create Preset
				</Button>
			</header>

			{presets.length === 0 && !loading ? (
				<div className="card">
					<EmptyState
						icon="presets"
						title="No presets yet"
						actions={
							<Button variant="primary" icon="plus" onClick={() => setEditing({ preset: null })}>
								Create Preset
							</Button>
						}
					>
						A preset is a saved list of packages for a World or Avatar project. Pick one when you create a new project to install them all at once.
					</EmptyState>
				</div>
			) : (
				<div className="card table">
					<div className="table-body">
						{presets.map((preset) => (
							<div key={preset.id} className="row preset-row">
								<div className="col-name">
									<strong title={preset.name}>{preset.name}</strong>
								</div>
								<div>
									<TypeBadge category={preset.kind} />
								</div>
								<PresetPackages preset={preset} />
								<div className="col-actions">
									<Button onClick={() => setEditing({ preset })}>Edit</Button>
									<Button variant="danger" onClick={() => void remove(preset)}>
										Remove
									</Button>
								</div>
							</div>
						))}
					</div>
				</div>
			)}
		</div>
	);
}
