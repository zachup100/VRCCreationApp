import { beforeEach, describe, expect, it, vi } from "vitest";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import { listKeys, radioGroupKeys } from "./a11y";

function press(key: string): ReactKeyboardEvent<HTMLElement> {
	return { key, preventDefault: vi.fn() } as unknown as ReactKeyboardEvent<HTMLElement>;
}

function build(html: string): HTMLElement {
	document.body.innerHTML = html;
	return document.body.firstElementChild as HTMLElement;
}

const keys = { next: ["ArrowDown"], previous: ["ArrowUp"] };

beforeEach(() => {
	document.body.innerHTML = "";
});

describe("listKeys", () => {
	it("moves focus to the next and previous item and wraps around", () => {
		const list = build('<div><button id="a" role="menuitem">A</button><button id="b" role="menuitem">B</button><button id="c" role="menuitem">C</button></div>');
		const [a, b, c] = ["a", "b", "c"].map((id) => document.getElementById(id) as HTMLElement);
		a.focus();
		expect(listKeys(press("ArrowDown"), list, '[role="menuitem"]', keys)).toBe(true);
		expect(document.activeElement).toBe(b);
		listKeys(press("ArrowDown"), list, '[role="menuitem"]', keys);
		expect(document.activeElement).toBe(c);
		listKeys(press("ArrowDown"), list, '[role="menuitem"]', keys);
		expect(document.activeElement).toBe(a);
		listKeys(press("ArrowUp"), list, '[role="menuitem"]', keys);
		expect(document.activeElement).toBe(c);
	});

	it("supports Home and End", () => {
		const list = build('<div><button id="a" role="menuitem">A</button><button id="b" role="menuitem">B</button><button id="c" role="menuitem">C</button></div>');
		document.getElementById("b")?.focus();
		listKeys(press("End"), list, '[role="menuitem"]', keys);
		expect(document.activeElement?.id).toBe("c");
		listKeys(press("Home"), list, '[role="menuitem"]', keys);
		expect(document.activeElement?.id).toBe("a");
	});

	it("skips disabled items", () => {
		const list = build('<div><button id="a" role="menuitem">A</button><button id="b" role="menuitem" disabled>B</button><button id="c" role="menuitem">C</button></div>');
		document.getElementById("a")?.focus();
		listKeys(press("ArrowDown"), list, '[role="menuitem"]', keys);
		expect(document.activeElement?.id).toBe("c");
	});

	it("ignores other keys and does not prevent them", () => {
		const list = build('<div><button id="a" role="menuitem">A</button></div>');
		const event = press("x");
		expect(listKeys(event, list, '[role="menuitem"]', keys)).toBe(false);
		expect(event.preventDefault).not.toHaveBeenCalled();
	});

	it("returns false for an empty container", () => {
		const list = build("<div></div>");
		expect(listKeys(press("ArrowDown"), list, '[role="menuitem"]', keys)).toBe(false);
		expect(listKeys(press("ArrowDown"), null, '[role="menuitem"]', keys)).toBe(false);
	});

	it("clicks the item it moves to when asked to activate", () => {
		const list = build('<div><button id="a" role="tab">A</button><button id="b" role="tab">B</button></div>');
		const onClick = vi.fn();
		document.getElementById("b")?.addEventListener("click", onClick);
		document.getElementById("a")?.focus();
		listKeys(press("ArrowRight"), list, '[role="tab"]', { next: ["ArrowRight"], previous: ["ArrowLeft"] }, true);
		expect(onClick).toHaveBeenCalledOnce();
	});
});

describe("radioGroupKeys", () => {
	it("selects the next radio with the arrow keys", () => {
		const group = build('<div><button id="a" role="radio">A</button><button id="b" role="radio">B</button></div>');
		const onClick = vi.fn();
		document.getElementById("b")?.addEventListener("click", onClick);
		document.getElementById("a")?.focus();
		const event = { key: "ArrowRight", preventDefault: vi.fn(), currentTarget: group } as unknown as ReactKeyboardEvent<HTMLElement>;
		radioGroupKeys(event);
		expect(document.activeElement?.id).toBe("b");
		expect(onClick).toHaveBeenCalledOnce();
	});
});
