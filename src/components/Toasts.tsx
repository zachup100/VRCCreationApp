import { useApp } from "../AppContext";
import { Icon } from "./Icons";

export function Toasts() {
	const { toasts, dismissToast } = useApp();
	return (
		<div className="toasts" aria-live="polite">
			{toasts.map((toast) => (
				<div key={toast.id} className={`toast toast-${toast.kind}`} role={toast.kind === "error" ? "alert" : "status"}>
					<Icon
						name={toast.kind === "error" || toast.kind === "warning" ? "alert" : toast.kind === "success" ? "check" : "info"}
						size={18}
					/>
					<span>{toast.text}</span>
					<button type="button" className="icon-btn" aria-label="Dismiss" onClick={() => dismissToast(toast.id)}>
						<Icon name="x" size={14} />
					</button>
				</div>
			))}
		</div>
	);
}
