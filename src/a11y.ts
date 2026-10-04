import type { KeyboardEvent as ReactKeyboardEvent } from "react";

export const FOCUSABLE =
	'a[href], button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

function enabledItems(container: HTMLElement, selector: string): HTMLElement[] {
	return Array.from(container.querySelectorAll<HTMLElement>(selector)).filter((item) => !item.hasAttribute("disabled"));
}

export function focusItem(container: HTMLElement | null, selector: string, which: "first" | "selected"): void {
	if (!container) {
		return;
	}
	const items = enabledItems(container, selector);
	const selected = which === "selected" ? items.find((item) => item.getAttribute("aria-selected") === "true") : undefined;
	(selected ?? items[0])?.focus();
}

export function listKeys(
	event: ReactKeyboardEvent<HTMLElement>,
	container: HTMLElement | null,
	selector: string,
	keys: { next: string[]; previous: string[] },
	activate = false,
): boolean {
	if (!container) {
		return false;
	}
	const items = enabledItems(container, selector);
	if (items.length === 0) {
		return false;
	}
	const current = items.indexOf(document.activeElement as HTMLElement);
	let target = -1;
	if (keys.next.includes(event.key)) {
		target = current < 0 ? 0 : (current + 1) % items.length;
	} else if (keys.previous.includes(event.key)) {
		target = current < 0 ? items.length - 1 : (current - 1 + items.length) % items.length;
	} else if (event.key === "Home") {
		target = 0;
	} else if (event.key === "End") {
		target = items.length - 1;
	}
	if (target < 0) {
		return false;
	}
	event.preventDefault();
	items[target].focus();
	if (activate) {
		items[target].click();
	}
	return true;
}

export function radioGroupKeys(event: ReactKeyboardEvent<HTMLElement>): void {
	listKeys(event, event.currentTarget, '[role="radio"]', { next: ["ArrowRight", "ArrowDown"], previous: ["ArrowLeft", "ArrowUp"] }, true);
}
