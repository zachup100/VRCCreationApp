import type { ReactNode, SVGProps } from "react";

const paths = {
	projects: (
		<>
			<rect x="3.5" y="3.5" width="7" height="7" rx="2" />
			<rect x="13.5" y="3.5" width="7" height="7" rx="2" />
			<rect x="3.5" y="13.5" width="7" height="7" rx="2" />
			<rect x="13.5" y="13.5" width="7" height="7" rx="2" />
		</>
	),
	learn: (
		<>
			<path d="M6 3.5h11a1.5 1.5 0 0 1 1.5 1.5v14A1.5 1.5 0 0 1 17 20.5H6z" />
			<path d="M6 3.5v17M6 20.5H5a1.5 1.5 0 0 1-1.5-1.5V5A1.5 1.5 0 0 1 5 3.5h1" />
			<path d="M10.5 9a1.7 1.7 0 1 1 2.4 1.5c-.6.3-.9.7-.9 1.3" />
			<path d="M12 14.4v.1" />
		</>
	),
	tools: (
		<>
			<rect x="3" y="7.5" width="18" height="12" rx="2" />
			<path d="M8.5 7.5v-2A1.5 1.5 0 0 1 10 4h4a1.5 1.5 0 0 1 1.5 1.5v2" />
			<path d="M3 13h18M10.5 13v2h3v-2" />
		</>
	),
	logs: (
		<>
			<path d="M7 3h7l4 4v13a1.5 1.5 0 0 1-1.5 1.5h-9.5A1.5 1.5 0 0 1 5.5 20V4.5A1.5 1.5 0 0 1 7 3z" />
			<path d="M14 3v4h4M8.5 12h7M8.5 15h7M8.5 18h4" />
		</>
	),
	settings: (
		<>
			<circle cx="12" cy="12" r="3" />
			<path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M18.7 5.3l-2.1 2.1M7.4 16.6l-2.1 2.1" />
		</>
	),
	search: (
		<>
			<circle cx="11" cy="11" r="6.5" />
			<path d="m16 16 4.5 4.5" />
		</>
	),
	refresh: (
		<>
			<path d="M20 11a8 8 0 0 0-14.4-4.3L4 8.5" />
			<path d="M4 4v4.5h4.5" />
			<path d="M4 13a8 8 0 0 0 14.4 4.3L20 15.5" />
			<path d="M20 20v-4.5h-4.5" />
		</>
	),
	list: (
		<>
			<path d="M9 6h11M9 12h11M9 18h11" />
			<circle cx="4.5" cy="6" r="1" />
			<circle cx="4.5" cy="12" r="1" />
			<circle cx="4.5" cy="18" r="1" />
		</>
	),
	grid: (
		<>
			<rect x="3.5" y="3.5" width="7" height="7" rx="1.5" />
			<rect x="13.5" y="3.5" width="7" height="7" rx="1.5" />
			<rect x="3.5" y="13.5" width="7" height="7" rx="1.5" />
			<rect x="13.5" y="13.5" width="7" height="7" rx="1.5" />
		</>
	),
	chevronDown: <path d="m6 9 6 6 6-6" />,
	chevronUp: <path d="m6 15 6-6 6 6" />,
	arrowLeft: <path d="M19 12H5m6-6-6 6 6 6" />,
	arrowUp: <path d="M12 19V5m-6 6 6-6 6 6" />,
	arrowDown: <path d="M12 5v14m-6-6 6 6 6-6" />,
	plus: <path d="M12 5v14M5 12h14" />,
	plusCircle: (
		<>
			<circle cx="12" cy="12" r="9" />
			<path d="M12 8v8M8 12h8" />
		</>
	),
	minusCircle: (
		<>
			<circle cx="12" cy="12" r="9" />
			<path d="M8 12h8" />
		</>
	),
	world: (
		<>
			<circle cx="12" cy="12" r="9" />
			<ellipse cx="12" cy="12" rx="4" ry="9" />
			<path d="M3 12h18M4.5 7.5h15M4.5 16.5h15" />
		</>
	),
	avatar: (
		<>
			<circle cx="12" cy="12" r="9" />
			<circle cx="12" cy="10" r="3" />
			<path d="M6.5 18.2c1.2-2.2 3-3.2 5.5-3.2s4.3 1 5.5 3.2" />
		</>
	),
	star: (
		<path d="m12 3.5 2.6 5.4 5.9.8-4.3 4.1 1 5.9-5.2-2.8-5.2 2.8 1-5.9-4.3-4.1 5.9-.8z" />
	),
	more: (
		<>
			<circle cx="5" cy="12" r="1.2" />
			<circle cx="12" cy="12" r="1.2" />
			<circle cx="19" cy="12" r="1.2" />
		</>
	),
	folder: <path d="M3 7a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />,
	folderOpen: (
		<>
			<path d="M3.5 18.5V6.5a1.5 1.5 0 0 1 1.5-1.5h4l2 2.5h6.5A1.5 1.5 0 0 1 19 9v2" />
			<path d="M3.5 18.5 6 11.5h15.5l-2.7 7z" />
		</>
	),
	package: (
		<>
			<path d="m12 3 8 4.5v9L12 21l-8-4.5v-9z" />
			<path d="m4 7.5 8 4.5 8-4.5M12 12v9" />
		</>
	),
	check: <path d="m5 12.5 4.5 4.5L19 7.5" />,
	x: <path d="M6 6l12 12M18 6 6 18" />,
	upload: (
		<>
			<circle cx="12" cy="12" r="9" />
			<path d="M12 16V8m-3.5 3.5L12 8l3.5 3.5" />
		</>
	),
	external: (
		<>
			<path d="M14 4h6v6M20 4l-9 9" />
			<path d="M18 14v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4" />
		</>
	),
	copy: (
		<>
			<rect x="8.5" y="8.5" width="11" height="11" rx="2" />
			<path d="M15.5 8.5V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7.5a2 2 0 0 0 2 2h2.5" />
		</>
	),
	trash: (
		<>
			<path d="M4 7h16M10 7V4.5h4V7M6.5 7l.8 12.5a1.5 1.5 0 0 0 1.5 1.4h6.4a1.5 1.5 0 0 0 1.5-1.4L17.5 7" />
			<path d="M10 11v6M14 11v6" />
		</>
	),
	info: (
		<>
			<circle cx="12" cy="12" r="9" />
			<path d="M12 11v5.5M12 7.8v.1" />
		</>
	),
	alert: (
		<>
			<path d="M12 3.5 21.5 20h-19z" />
			<path d="M12 10v4.5M12 17.3v.1" />
		</>
	),
	rocket: (
		<>
			<path d="M14 4c3.5.2 6 2.7 6 6-2.2 3.2-5 5.3-8.5 6.5L7.5 12.5C8.7 9 10.8 6.2 14 4z" />
			<circle cx="14.5" cy="9.5" r="1.5" />
			<path d="M7.5 12.5 4 13l2-3.5M11.5 16.5 11 20l3.5-2M5 19c0-1.7.8-2.5 2.5-2.5" />
		</>
	),
	download: (
		<>
			<path d="M12 4v11m-4-4 4 4 4-4" />
			<path d="M4.5 19.5h15" />
		</>
	),
	book: (
		<>
			<path d="M4 5.5A1.5 1.5 0 0 1 5.5 4H12v16H5.5A1.5 1.5 0 0 1 4 18.5z" />
			<path d="M20 5.5A1.5 1.5 0 0 0 18.5 4H12v16h6.5a1.5 1.5 0 0 0 1.5-1.5z" />
		</>
	),
	backups: (
		<>
			<rect x="3" y="4" width="18" height="5" rx="1.5" />
			<path d="M5 9v9.5A1.5 1.5 0 0 0 6.5 20h11a1.5 1.5 0 0 0 1.5-1.5V9" />
			<path d="M10 13h4" />
		</>
	),
	unity: (
		<>
			<path d="m12 3 7.5 4.3v9.4L12 21l-7.5-4.3V7.3z" />
			<path d="m12 12 7.5-4.4M12 12 4.5 7.6M12 12v9" />
		</>
	),
} satisfies Record<string, ReactNode>;

export type IconName = keyof typeof paths;

interface IconProps extends Omit<SVGProps<SVGSVGElement>, "name"> {
	name: IconName;
	size?: number;
	filled?: boolean;
}

export function Icon({ name, size = 20, filled = false, ...rest }: IconProps) {
	return (
		<svg
			width={size}
			height={size}
			viewBox="0 0 24 24"
			fill={filled ? "currentColor" : "none"}
			stroke="currentColor"
			strokeWidth="1.7"
			strokeLinecap="round"
			strokeLinejoin="round"
			aria-hidden="true"
			{...rest}
		>
			{paths[name]}
		</svg>
	);
}
