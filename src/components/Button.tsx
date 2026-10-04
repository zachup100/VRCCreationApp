import type { ButtonHTMLAttributes, ReactNode } from "react";
import { Icon } from "./Icons";
import type { IconName } from "./Icons";

export type ButtonVariant = "primary" | "outline" | "success" | "danger" | "ghost";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
	variant?: ButtonVariant;
	icon?: IconName;
	children?: ReactNode;
}

export function Button({ variant = "outline", icon, children, className, type, ...rest }: ButtonProps) {
	return (
		<button
			type={type ?? "button"}
			className={`btn btn-${variant}${className ? ` ${className}` : ""}`}
			{...rest}
		>
			{icon && <Icon name={icon} size={18} />}
			{children}
		</button>
	);
}

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
	icon: IconName;
	label: string;
	spinning?: boolean;
	filled?: boolean;
	size?: number;
}

export function IconButton({ icon, label, spinning, filled, size = 20, className, type, ...rest }: IconButtonProps) {
	return (
		<button
			type={type ?? "button"}
			className={`icon-btn${spinning ? " spinning" : ""}${className ? ` ${className}` : ""}`}
			aria-label={label}
			title={label}
			{...rest}
		>
			<Icon name={icon} size={size} filled={filled} />
		</button>
	);
}
