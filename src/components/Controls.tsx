import { useEffect, useId, useRef, useState } from "react";
import type { InputHTMLAttributes, ReactNode } from "react";
import { createPortal } from "react-dom";
import { FOCUSABLE, focusItem, listKeys, radioGroupKeys } from "../a11y";
import { Button } from "./Button";
import { Icon } from "./Icons";

export function SearchInput({
	value,
	onChange,
	placeholder,
}: {
	value: string;
	onChange: (value: string) => void;
	placeholder: string;
}) {
	return (
		<label className="search">
			<Icon name="search" size={18} />
			<input
				type="text"
				value={value}
				placeholder={placeholder}
				onChange={(event) => onChange(event.target.value)}
			/>
			{value && (
				<button type="button" className="search-clear" aria-label="Clear search" onClick={() => onChange("")}>
					<Icon name="x" size={14} />
				</button>
			)}
		</label>
	);
}

export function TextField({
	label,
	valid,
	...rest
}: InputHTMLAttributes<HTMLInputElement> & { label: string; valid?: boolean }) {
	const id = useId();
	return (
		<div className="field">
			<label htmlFor={id}>{label}</label>
			<div className="field-input">
				<input id={id} type="text" {...rest} />
				{valid && <Icon name="check" size={18} className="field-valid" />}
			</div>
		</div>
	);
}

export function Checkbox({
	checked,
	onChange,
	disabled,
	label,
}: {
	checked: boolean;
	onChange?: (checked: boolean) => void;
	disabled?: boolean;
	label: string;
}) {
	return (
		<button
			type="button"
			role="checkbox"
			aria-checked={checked}
			aria-label={label}
			disabled={disabled}
			className={`checkbox${checked ? " checked" : ""}`}
			onClick={() => onChange?.(!checked)}
		>
			{checked && <Icon name="check" size={14} strokeWidth={3} />}
		</button>
	);
}

export function Switch({
	checked,
	onChange,
	disabled,
	label,
}: {
	checked: boolean;
	onChange: (checked: boolean) => void;
	disabled?: boolean;
	label: string;
}) {
	return (
		<button
			type="button"
			role="switch"
			aria-checked={checked}
			aria-label={label}
			disabled={disabled}
			className={`switch${checked ? " on" : ""}`}
			onClick={() => onChange(!checked)}
		>
			<span className="switch-thumb" />
		</button>
	);
}

export function Segmented<T extends string>({
	value,
	options,
	onChange,
	label,
}: {
	value: T;
	options: { value: T; label: string }[];
	onChange: (value: T) => void;
	label: string;
}) {
	return (
		<div className="segmented" role="radiogroup" aria-label={label} onKeyDown={radioGroupKeys}>
			{options.map((option) => (
				<button
					key={option.value}
					type="button"
					role="radio"
					aria-checked={option.value === value}
					tabIndex={option.value === value ? 0 : -1}
					className={`segment${option.value === value ? " active" : ""}`}
					onClick={() => onChange(option.value)}
				>
					{option.label}
				</button>
			))}
		</div>
	);
}

export interface Tab<T extends string> {
	id: T;
	label: string;
}

export function Tabs<T extends string>({
	tabs,
	active,
	onChange,
}: {
	tabs: Tab<T>[];
	active: T;
	onChange: (id: T) => void;
}) {
	return (
		<div
			className="tabs"
			role="tablist"
			onKeyDown={(event) =>
				listKeys(event, event.currentTarget, '[role="tab"]', { next: ["ArrowRight"], previous: ["ArrowLeft"] }, true)
			}
		>
			{tabs.map((tab) => (
				<button
					key={tab.id}
					type="button"
					role="tab"
					aria-selected={tab.id === active}
					tabIndex={tab.id === active ? 0 : -1}
					className={`tab${tab.id === active ? " active" : ""}`}
					onClick={() => onChange(tab.id)}
				>
					{tab.label}
				</button>
			))}
		</div>
	);
}

export interface SelectOption<T extends string> {
	value: T;
	label: string;
}

export function Select<T extends string>({
	value,
	options,
	onChange,
	label,
	disabled,
	placeholder,
	muted,
}: {
	value: T;
	options: SelectOption<T>[];
	onChange: (value: T) => void;
	label: string;
	disabled?: boolean;
	placeholder?: string;
	muted?: boolean;
}) {
	const [open, setOpen] = useState(false);
	const ref = useRef<HTMLDivElement>(null);
	const listRef = useRef<HTMLDivElement>(null);
	useEffect(() => {
		if (open) {
			focusItem(listRef.current, '[role="option"]', "selected");
		}
	}, [open]);
	useEffect(() => {
		if (!open) {
			return;
		}
		const onPointer = (event: MouseEvent) => {
			if (ref.current && !ref.current.contains(event.target as Node)) {
				setOpen(false);
			}
		};
		const onKey = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				setOpen(false);
				ref.current?.querySelector<HTMLElement>(".select-trigger")?.focus();
			}
		};
		document.addEventListener("mousedown", onPointer);
		document.addEventListener("keydown", onKey);
		return () => {
			document.removeEventListener("mousedown", onPointer);
			document.removeEventListener("keydown", onKey);
		};
	}, [open]);
	const current = options.find((option) => option.value === value);
	return (
		<div className="select menu-anchor" ref={ref}>
			<button
				type="button"
				className={`select-trigger${muted ? " muted" : ""}`}
				aria-label={label}
				aria-haspopup="listbox"
				aria-expanded={open}
				disabled={disabled}
				onClick={() => setOpen((state) => !state)}
			>
				<span>{muted && placeholder ? placeholder : (current?.label ?? "")}</span>
				<Icon name="chevronDown" size={16} />
			</button>
			{open && (
				<div
					className="menu"
					role="listbox"
					ref={listRef}
					aria-label={label}
					onKeyDown={(event) => {
						if (event.key === "Tab") {
							setOpen(false);
							return;
						}
						listKeys(event, listRef.current, '[role="option"]', { next: ["ArrowDown"], previous: ["ArrowUp"] });
					}}
				>
					{options.map((option) => (
						<button
							key={option.value}
							type="button"
							role="option"
							aria-selected={option.value === value}
							className={`menu-item${option.value === value ? " selected" : ""}`}
							onClick={() => {
								setOpen(false);
								ref.current?.querySelector<HTMLElement>(".select-trigger")?.focus();
								onChange(option.value);
							}}
						>
							{option.label}
							{option.value === value && <Icon name="check" size={16} className="push" />}
						</button>
					))}
				</div>
			)}
		</div>
	);
}

export function Modal({
	title,
	onClose,
	children,
	actions,
	wide,
}: {
	title: string;
	onClose: () => void;
	children: ReactNode;
	actions: ReactNode;
	wide?: boolean;
}) {
	const titleId = useId();
	const dialogRef = useRef<HTMLDivElement>(null);
	useEffect(() => {
		const onKey = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				onClose();
			}
		};
		document.addEventListener("keydown", onKey);
		return () => document.removeEventListener("keydown", onKey);
	}, [onClose]);
	useEffect(() => {
		const previous = document.activeElement as HTMLElement | null;
		const dialog = dialogRef.current;
		if (dialog && !dialog.contains(document.activeElement)) {
			const target =
				dialog.querySelector<HTMLElement>(`.modal-body ${FOCUSABLE.split(", ").join(", .modal-body ")}`) ??
				dialog.querySelector<HTMLElement>(".modal-actions button:not(:disabled)") ??
				dialog.querySelector<HTMLElement>(FOCUSABLE) ??
				dialog;
			target.focus();
		}
		return () => previous?.focus?.();
	}, []);
	return createPortal(
		<div className="modal-backdrop" onMouseDown={onClose}>
			<div
				className={`modal${wide ? " wide" : ""}`}
				role="dialog"
				aria-modal="true"
				aria-labelledby={titleId}
				tabIndex={-1}
				ref={dialogRef}
				onMouseDown={(event) => event.stopPropagation()}
				onKeyDown={(event) => {
					if (event.key !== "Tab" || !dialogRef.current) {
						return;
					}
					const focusable = Array.from(dialogRef.current.querySelectorAll<HTMLElement>(FOCUSABLE));
					if (focusable.length === 0) {
						event.preventDefault();
						return;
					}
					const first = focusable[0];
					const last = focusable[focusable.length - 1];
					if (event.shiftKey && (document.activeElement === first || document.activeElement === dialogRef.current)) {
						event.preventDefault();
						last.focus();
					} else if (!event.shiftKey && document.activeElement === last) {
						event.preventDefault();
						first.focus();
					}
				}}
			>
				<div className="modal-head">
					<h2 id={titleId}>{title}</h2>
					<button type="button" className="icon-btn" aria-label="Close" onClick={onClose}>
						<Icon name="x" size={20} />
					</button>
				</div>
				<div className="modal-body">{children}</div>
				<div className="modal-actions">{actions}</div>
			</div>
		</div>,
		document.body,
	);
}

export function ConfirmModal({
	title,
	confirmLabel,
	danger,
	onConfirm,
	onClose,
	children,
}: {
	title: string;
	confirmLabel: string;
	danger?: boolean;
	onConfirm: () => void;
	onClose: () => void;
	children: ReactNode;
}) {
	return (
		<Modal
			title={title}
			onClose={onClose}
			actions={
				<>
					<Button onClick={onClose}>Cancel</Button>
					<Button
						variant={danger ? "danger" : "primary"}
						onClick={() => {
							onClose();
							onConfirm();
						}}
					>
						{confirmLabel}
					</Button>
				</>
			}
		>
			{children}
		</Modal>
	);
}

export function EmptyState({
	icon,
	title,
	children,
	actions,
}: {
	icon: "projects" | "package" | "logs" | "unity" | "learn" | "backups" | "presets";
	title: string;
	children?: ReactNode;
	actions?: ReactNode;
}) {
	return (
		<div className="empty">
			<div className="empty-art">
				<Icon name={icon} size={40} />
			</div>
			<h3>{title}</h3>
			{children && <p>{children}</p>}
			{actions && <div className="empty-actions">{actions}</div>}
		</div>
	);
}
