import { useApp } from "../AppContext";
import { cancelUnityInstall } from "../api";
import type { UnityInstall } from "../api";
import { errorMessage, formatBytes } from "../format";
import { Button } from "./Button";

function title(install: UnityInstall): string {
	if (install.kind === "editor") {
		return `Installing Unity ${install.version}`;
	}
	switch (install.phase) {
		case "verifying":
			return "Checking the Unity Hub installer";
		case "installing":
			return "Installing Unity Hub";
		default:
			return "Downloading Unity Hub";
	}
}

function detail(install: UnityInstall, percent: number | null): string {
	if (install.kind === "editor") {
		const status = install.line || "Unity Hub is working. This can take a long time.";
		return percent === null ? status : `${percent}% overall. ${status}`;
	}
	switch (install.phase) {
		case "verifying":
			return "Checking that the installer is signed by Unity Technologies.";
		case "installing":
			return "Accept the Windows permission prompt if it appears.";
		default:
			return percent === null ? install.line || "Starting..." : `${formatBytes(install.bytes)} of ${formatBytes(install.total)} (${percent}%)`;
	}
}

function cancellable(install: UnityInstall): boolean {
	if (install.kind === "editor") {
		return install.phase === "installing";
	}
	return install.phase === "downloading" || install.phase === "starting";
}

export function UnityInstallStatus({ install }: { install: UnityInstall }) {
	const { notify } = useApp();
	const determinate = install.total > 0 && (install.kind === "editor" || install.phase === "downloading");
	const percent = determinate ? Math.min(100, Math.round((install.bytes / install.total) * 100)) : null;
	const heading = title(install);

	const cancel = async () => {
		try {
			await cancelUnityInstall();
		} catch (error) {
			notify("error", errorMessage(error));
		}
	};

	return (
		<div className="card install-status">
			<div className="install-status-head">
				<strong role="status">{heading}</strong>
				{cancellable(install) && (
					<Button variant="ghost" onClick={() => void cancel()}>
						Cancel
					</Button>
				)}
			</div>
			<div
				className={`progress${determinate ? "" : " indeterminate"}`}
				role="progressbar"
				aria-label={heading}
				aria-valuemin={0}
				aria-valuemax={100}
				aria-valuenow={percent ?? undefined}
			>
				<div className="progress-fill" style={percent === null ? undefined : { width: `${percent}%` }} />
			</div>
			<span className="muted install-detail">{detail(install, percent)}</span>
		</div>
	);
}

export function UnityInstallNotice() {
	const { unityInstall, navigate } = useApp();
	if (!unityInstall) {
		return null;
	}
	return (
		<div className="notice warn" role="status">
			<span>A Unity install is in progress. Projects cannot be opened or changed until it finishes.</span>
			<button type="button" className="link-btn" onClick={() => navigate({ page: "unity" })}>
				View progress
			</button>
		</div>
	);
}
