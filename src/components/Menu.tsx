import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { focusItem, listKeys } from "../a11y";
import { Button, IconButton } from "./Button";
import type { ButtonVariant } from "./Button";
import { Icon } from "./Icons";
import type { IconName } from "./Icons";

export interface MenuItem {
	label: string;
	icon?: IconName;
	danger?: boolean;
	disabled?: boolean;
	onSelect?: () => void;
	submenu?: MenuItem[];
}

const ITEM_SELECTOR = ':scope > [role="menuitem"], :scope > .submenu-anchor > [role="menuitem"]';
const SUBMENU_ITEM_SELECTOR = ':scope > [role="menuitem"]';

function SubMenuItem({ item, closeAndFocus }: { item: MenuItem; closeAndFocus: () => void }) {
	const [open, setOpen] = useState(false);
	const [focusFirst, setFocusFirst] = useState(false);
	const triggerRef = useRef<HTMLButtonElement>(null);
	const listRef = useRef<HTMLDivElement>(null);
	useEffect(() => {
		if (open && focusFirst) {
			focusItem(listRef.current, SUBMENU_ITEM_SELECTOR, "first");
			setFocusFirst(false);
		}
	}, [open, focusFirst]);
	const closeSub = () => {
		setOpen(false);
		triggerRef.current?.focus();
	};
	return (
		<div className="submenu-anchor" onMouseEnter={() => !item.disabled && setOpen(true)} onMouseLeave={() => setOpen(false)}>
			<button
				ref={triggerRef}
				type="button"
				role="menuitem"
				aria-haspopup="menu"
				aria-expanded={open}
				disabled={item.disabled}
				className={`menu-item${item.danger ? " danger" : ""}`}
				onClick={() => setOpen((value) => !value)}
				onKeyDown={(event) => {
					if (event.key === "ArrowLeft" || event.key === "Enter" || event.key === " ") {
						event.preventDefault();
						setOpen(true);
						setFocusFirst(true);
					}
				}}
			>
				{item.icon && <Icon name={item.icon} size={17} />}
				{item.label}
				<Icon name="chevronLeft" size={16} className="push" />
			</button>
			{open && (
				<div
					className="menu submenu"
					role="menu"
					ref={listRef}
					onKeyDown={(event) => {
						event.stopPropagation();
						if (event.key === "Escape" || event.key === "ArrowRight") {
							event.preventDefault();
							closeSub();
							return;
						}
						if (event.key === "Tab") {
							closeSub();
							return;
						}
						listKeys(event, listRef.current, SUBMENU_ITEM_SELECTOR, { next: ["ArrowDown"], previous: ["ArrowUp"] });
					}}
				>
					{(item.submenu ?? []).map((sub) => (
						<button
							key={sub.label}
							type="button"
							role="menuitem"
							disabled={sub.disabled}
							className={`menu-item${sub.danger ? " danger" : ""}`}
							onClick={() => {
								closeAndFocus();
								sub.onSelect?.();
							}}
						>
							{sub.icon && <Icon name={sub.icon} size={17} />}
							{sub.label}
						</button>
					))}
				</div>
			)}
		</div>
	);
}

function useDismiss(open: boolean, close: () => void, onEscape?: () => void) {
	const ref = useRef<HTMLDivElement>(null);
	useEffect(() => {
		if (!open) {
			return;
		}
		const onPointer = (event: MouseEvent) => {
			if (ref.current && !ref.current.contains(event.target as Node)) {
				close();
			}
		};
		const onKey = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				(onEscape ?? close)();
			}
		};
		document.addEventListener("mousedown", onPointer);
		document.addEventListener("keydown", onKey);
		return () => {
			document.removeEventListener("mousedown", onPointer);
			document.removeEventListener("keydown", onKey);
		};
	}, [open, close, onEscape]);
	return ref;
}

function MenuList({ items, close, closeAndFocus }: { items: MenuItem[]; close: () => void; closeAndFocus: () => void }) {
	const ref = useRef<HTMLDivElement>(null);
	useEffect(() => {
		focusItem(ref.current, ITEM_SELECTOR, "first");
	}, []);
	return (
		<div
			className="menu"
			role="menu"
			ref={ref}
			onKeyDown={(event) => {
				if (event.key === "Tab") {
					close();
					return;
				}
				listKeys(event, ref.current, ITEM_SELECTOR, { next: ["ArrowDown"], previous: ["ArrowUp"] });
			}}
		>
			{items.map((item) =>
				item.submenu ? (
					<SubMenuItem key={item.label} item={item} closeAndFocus={closeAndFocus} />
				) : (
				<button
					key={item.label}
					type="button"
					role="menuitem"
					disabled={item.disabled}
					className={`menu-item${item.danger ? " danger" : ""}`}
					onClick={() => {
						closeAndFocus();
						item.onSelect?.();
					}}
				>
					{item.icon && <Icon name={item.icon} size={17} />}
					{item.label}
				</button>
				),
			)}
		</div>
	);
}

export function MoreMenu({ items, label = "More actions" }: { items: MenuItem[]; label?: string }) {
	const [open, setOpen] = useState(false);
	const closeAndFocus = () => {
		setOpen(false);
		ref.current?.querySelector<HTMLElement>("button")?.focus();
	};
	const ref = useDismiss(open, () => setOpen(false), closeAndFocus);
	return (
		<div className="menu-anchor" ref={ref}>
			<IconButton icon="more" label={label} aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen((value) => !value)} />
			{open && <MenuList items={items} close={() => setOpen(false)} closeAndFocus={closeAndFocus} />}
		</div>
	);
}

interface SplitButtonProps {
	variant?: ButtonVariant;
	disabled?: boolean;
	children: ReactNode;
	onClick: () => void;
	items: MenuItem[];
}

export function SplitButton({ variant = "primary", disabled, children, onClick, items }: SplitButtonProps) {
	const [open, setOpen] = useState(false);
	const closeAndFocus = () => {
		setOpen(false);
		ref.current?.querySelector<HTMLElement>(".split-toggle")?.focus();
	};
	const ref = useDismiss(open, () => setOpen(false), closeAndFocus);
	return (
		<div className="split menu-anchor" ref={ref}>
			<Button variant={variant} className="split-main" disabled={disabled} onClick={onClick}>
				{children}
			</Button>
			<Button
				variant={variant}
				className="split-toggle"
				disabled={disabled}
				aria-label="More options"
				aria-haspopup="menu"
				aria-expanded={open}
				onClick={() => setOpen((value) => !value)}
			>
				<Icon name="chevronDown" size={16} />
			</Button>
			{open && <MenuList items={items} close={() => setOpen(false)} closeAndFocus={closeAndFocus} />}
		</div>
	);
}
