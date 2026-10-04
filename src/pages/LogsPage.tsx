import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";
import { useApp } from "../AppContext";
import type { NoticeKind } from "../AppContext";
import { clearBackendLogs, getLogs } from "../api";
import type { BackendLogEntry } from "../api";
import { Button } from "../components/Button";
import { EmptyState, Segmented } from "../components/Controls";
import { Icon } from "../components/Icons";

type Filter = "all" | "issues";

interface Line {
	key: string;
	time: number;
	kind: NoticeKind;
	text: string;
}

export function LogsPage() {
	const { log, clearLog, notify } = useApp();
	const [backend, setBackend] = useState<BackendLogEntry[]>([]);
	const [filter, setFilter] = useState<Filter>("all");

	useEffect(() => {
		let unlisten: (() => void) | null = null;
		let cancelled = false;
		getLogs()
			.then((entries) => {
				if (!cancelled) {
					setBackend(entries);
				}
			})
			.catch(() => undefined);
		listen<BackendLogEntry>("app-log", (event) => setBackend((current) => [...current, event.payload].slice(-500)))
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
	}, []);

	const lines = useMemo(() => {
		const merged: Line[] = [
			...backend.map((entry) => ({ key: `b-${entry.id}`, time: entry.time, kind: entry.level, text: entry.message })),
			...log.map((entry) => ({ key: `f-${entry.id}`, time: entry.time, kind: entry.kind, text: entry.text })),
		];
		merged.sort((a, b) => b.time - a.time);
		return filter === "issues" ? merged.filter((line) => line.kind === "warning" || line.kind === "error") : merged;
	}, [backend, log, filter]);

	const clear = () => {
		clearLog();
		setBackend([]);
		void clearBackendLogs();
	};

	const copy = async () => {
		const text = lines.map((line) => `${new Date(line.time).toISOString()} [${line.kind}] ${line.text}`).join("\n");
		try {
			await navigator.clipboard.writeText(text);
			notify("info", "Log copied to the clipboard");
		} catch {
			notify("error", "Could not copy the log");
		}
	};

	return (
		<div className="page">
			<header className="page-header">
				<h1>Logs</h1>
				<span className="spacer" />
				<Segmented<Filter>
					label="Log filter"
					value={filter}
					onChange={setFilter}
					options={[
						{ value: "all", label: "All" },
						{ value: "issues", label: "Warnings and errors" },
					]}
				/>
				<Button variant="ghost" icon="copy" disabled={lines.length === 0} onClick={() => void copy()}>
					Copy
				</Button>
				<Button variant="ghost" icon="trash" disabled={backend.length === 0 && log.length === 0} onClick={clear}>
					Clear
				</Button>
			</header>
			<div className="card">
				{lines.length === 0 ? (
					<EmptyState icon="logs" title={filter === "issues" ? "No warnings or errors" : "Nothing logged yet"}>
						Package changes, project creation, backups, migrations and any problems are recorded here for this session.
					</EmptyState>
				) : (
					<ul className="log-list">
						{lines.map((line) => (
							<li key={line.key} className={`log-${line.kind}`}>
								<Icon name={line.kind === "error" || line.kind === "warning" ? "alert" : line.kind === "success" ? "check" : "info"} size={16} />
								<time dateTime={new Date(line.time).toISOString()}>{new Date(line.time).toLocaleTimeString()}</time>
								<span className="log-text">{line.text}</span>
							</li>
						))}
					</ul>
				)}
			</div>
		</div>
	);
}
