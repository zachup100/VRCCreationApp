import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { formatBytes } from "../format";

interface Progress {
	bytes: number;
	name: string;
}

export function DownloadStatus({ active }: { active: boolean }) {
	const [progress, setProgress] = useState<Progress>({ bytes: 0, name: "" });

	useEffect(() => {
		if (!active) {
			return;
		}
		setProgress({ bytes: 0, name: "" });
		let unlisten: (() => void) | null = null;
		let cancelled = false;
		listen<Progress>("download-progress", (event) => setProgress(event.payload))
			.then((stop) => {
				if (cancelled) {
					stop();
				} else {
					unlisten = stop;
				}
			})
			.catch(() => undefined);
		return () => {
			cancelled = true;
			unlisten?.();
		};
	}, [active]);

	if (!active) {
		return null;
	}

	return (
		<div className="download-status" role="status">
			<div className="progress indeterminate" aria-hidden="true">
				<div className="progress-fill" />
			</div>
			<span className="muted">
				{progress.bytes > 0 ? `Downloaded ${formatBytes(progress.bytes)}${progress.name ? ` (${progress.name})` : ""}` : "Preparing packages..."}
			</span>
		</div>
	);
}
