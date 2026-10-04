import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Button, IconButton } from "./Button";
import type { ButtonVariant } from "./Button";
import { Icon } from "./Icons";
import type { IconName } from "./Icons";

export interface MenuItem {
	label: string;
	icon?: IconName;
	danger?: boolean;
	disabled?: boolean;
	onSelect: () => void;
}

function useDismiss(open: boolean, close: () => void) {
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
				close();
			}
		};
		document.addEventListener("mousedown", onPointer);
		document.addEventListener("keydown", onKey);
		return () => {
			document.removeEventListener("mousedown", onPointer);
			document.removeEventListener("keydown", onKey);
		};
	}, [open, close]);
	return ref;
}

function MenuList({ items, close }: { items: MenuItem[]; close: () => void }) {
	return (
		<div className="menu" role="menu">
			{items.map((item) => (
				<button
					key={item.label}
					type="button"
					role="menuitem"
					disabled={item.disabled}
					className={`menu-item${item.danger ? " danger" : ""}`}
					onClick={() => {
						close();
						item.onSelect();
					}}
				>
					{item.icon && <Icon name={item.icon} size={17} />}
					{item.label}
				</button>
			))}
		</div>
	);
}

export function MoreMenu({ items, label = "More actions" }: { items: MenuItem[]; label?: string }) {
	const [open, setOpen] = useState(false);
	const ref = useDismiss(open, () => setOpen(false));
	return (
		<div className="menu-anchor" ref={ref}>
			<IconButton icon="more" label={label} onClick={() => setOpen((value) => !value)} />
			{open && <MenuList items={items} close={() => setOpen(false)} />}
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
	const ref = useDismiss(open, () => setOpen(false));
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
			{open && <MenuList items={items} close={() => setOpen(false)} />}
		</div>
	);
}
