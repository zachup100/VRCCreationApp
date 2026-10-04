import { useApp } from "../AppContext";
import { Button } from "../components/Button";
import { EmptyState } from "../components/Controls";
import { Icon } from "../components/Icons";

export function LogsPage() {
	const { log, clearLog } = useApp();
	return (
		<div className="page">
			<header className="page-header">
				<h1>Logs</h1>
				<span className="spacer" />
				<Button variant="ghost" icon="trash" disabled={log.length === 0} onClick={clearLog}>
					Clear
				</Button>
			</header>
			<div className="card">
				{log.length === 0 ? (
					<EmptyState icon="logs" title="Nothing logged yet">
						Actions and errors from this session will show up here.
					</EmptyState>
				) : (
					<ul className="log-list">
						{log.map((entry) => (
							<li key={entry.id} className={`log-${entry.kind}`}>
								<Icon name={entry.kind === "error" ? "alert" : entry.kind === "success" ? "check" : "info"} size={16} />
								<time>{new Date(entry.time).toLocaleTimeString()}</time>
								<span>{entry.text}</span>
							</li>
						))}
					</ul>
				)}
			</div>
		</div>
	);
}
