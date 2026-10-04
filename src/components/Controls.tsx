import { useEffect, useId, useRef, useState } from "react";
import type { InputHTMLAttributes, ReactNode } from "react";
import { createPortal } from "react-dom";
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
		<div className="tabs" role="tablist">
			{tabs.map((tab) => (
				<button
					key={tab.id}
					type="button"
					role="tab"
					aria-selected={tab.id === active}
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
				<div className="menu" role="listbox">
					{options.map((option) => (
						<button
							key={option.value}
							type="button"
							role="option"
							aria-selected={option.value === value}
							className={`menu-item${option.value === value ? " selected" : ""}`}
							onClick={() => {
								setOpen(false);
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
}: {
	title: string;
	onClose: () => void;
	children: ReactNode;
	actions: ReactNode;
}) {
	useEffect(() => {
		const onKey = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				onClose();
			}
		};
		document.addEventListener("keydown", onKey);
		return () => document.removeEventListener("keydown", onKey);
	}, [onClose]);
	return createPortal(
		<div className="modal-backdrop" onMouseDown={onClose}>
			<div
				className="modal"
				role="dialog"
				aria-modal="true"
				aria-label={title}
				onMouseDown={(event) => event.stopPropagation()}
			>
				<div className="modal-head">
					<h2>{title}</h2>
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
	icon: "projects" | "package" | "logs" | "unity" | "learn" | "backups";
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
