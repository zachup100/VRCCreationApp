import { useEffect, useState } from "react";
import { listUnityEditors, openUnityDownloadPage } from "./api";
import type { UnityEditorInfo } from "./api";

type LoadState =
	| { status: "loading" }
	| { status: "error"; message: string }
	| { status: "ready"; editors: UnityEditorInfo[] };

export function App() {
	const [state, setState] = useState<LoadState>({ status: "loading" });

	useEffect(() => {
		listUnityEditors()
			.then((editors) => setState({ status: "ready", editors }))
			.catch((error) => setState({ status: "error", message: String(error) }));
	}, []);

	return (
		<main className="shell">
			<h1>VRCreationApp</h1>
			<h2>Detected Unity editors</h2>
			{state.status === "loading" && <p>Scanning...</p>}
			{state.status === "error" && (
				<p className="error">Could not read Unity installs: {state.message}</p>
			)}
			{state.status === "ready" && state.editors.length === 0 && (
				<p>
					No Unity editors found. Install Unity Hub and a Unity editor, then restart.{" "}
					<button type="button" onClick={() => void openUnityDownloadPage()}>
						Open Unity download page
					</button>
				</p>
			)}
			{state.status === "ready" && state.editors.length > 0 && (
				<ul className="editors">
					{state.editors.map((editor) => (
						<li key={`${editor.version}-${editor.path}`}>
							<strong>{editor.version}</strong>
							{editor.architecture && <span> ({editor.architecture})</span>}
							<div className="path">{editor.path}</div>
						</li>
					))}
				</ul>
			)}
		</main>
	);
}
