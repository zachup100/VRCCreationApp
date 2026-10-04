import { useEffect, useState } from "react";
import { useApp } from "../AppContext";
import { getLicenses } from "../api";
import { errorMessage } from "../format";
import { Button } from "./Button";
import { Modal } from "./Controls";

export function LicensesModal({ onClose }: { onClose: () => void }) {
	const { notify } = useApp();
	const [text, setText] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		getLicenses()
			.then(setText)
			.catch((failure) => setError(errorMessage(failure)));
	}, []);

	const copy = async () => {
		if (!text) {
			return;
		}
		try {
			await navigator.clipboard.writeText(text);
			notify("info", "Licenses copied to the clipboard");
		} catch {
			notify("error", "Could not copy the licenses");
		}
	};

	return (
		<Modal
			title="Open Source Licenses"
			wide
			onClose={onClose}
			actions={
				<>
					<Button icon="copy" disabled={text === null} onClick={() => void copy()}>
						Copy
					</Button>
					<Button variant="primary" onClick={onClose}>
						Close
					</Button>
				</>
			}
		>
			{error && <p className="field-error">{error}</p>}
			{!error && text === null && <p>Loading...</p>}
			{text !== null && <pre className="license-text">{text}</pre>}
		</Modal>
	);
}
