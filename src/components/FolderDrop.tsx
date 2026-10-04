import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useEffect, useRef, useState } from "react";
import { Icon } from "./Icons";

export function FolderDrop({ enabled, onDrop }: { enabled: boolean; onDrop: (paths: string[]) => void }) {
	const [hovering, setHovering] = useState(false);
	const handler = useRef(onDrop);
	handler.current = onDrop;

	useEffect(() => {
		if (!enabled) {
			setHovering(false);
			return;
		}
		let unlisten: (() => void) | null = null;
		let cancelled = false;
		getCurrentWebview()
			.onDragDropEvent((event) => {
				const payload = event.payload;
				if (payload.type === "enter" || payload.type === "over") {
					setHovering(true);
				} else if (payload.type === "drop") {
					setHovering(false);
					if (payload.paths.length > 0) {
						handler.current(payload.paths);
					}
				} else {
					setHovering(false);
				}
			})
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
	}, [enabled]);

	if (!hovering) {
		return null;
	}

	return (
		<div className="drop-overlay" aria-hidden="true">
			<div className="drop-card">
				<Icon name="folderOpen" size={44} />
				<strong>Drop a Unity project folder to add it</strong>
				<span>You can drop several folders at once.</span>
			</div>
		</div>
	);
}
