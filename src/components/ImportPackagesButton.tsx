import { useActions } from "../actions";
import { Button } from "./Button";

export function ImportPackagesButton() {
	const actions = useActions();
	const progress = actions.packageImportRunning;
	return (
		<Button icon="package" disabled={progress !== null} onClick={() => void actions.importPackagesVcc()}>
			{progress ? (progress.total > 0 ? `Importing ${progress.done}/${progress.total}...` : "Importing...") : "Import Packages From VCC"}
		</Button>
	);
}
