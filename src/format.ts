const units: [Intl.RelativeTimeFormatUnit, number][] = [
	["year", 365 * 24 * 3600],
	["month", 30 * 24 * 3600],
	["week", 7 * 24 * 3600],
	["day", 24 * 3600],
	["hour", 3600],
	["minute", 60],
];

const relative = new Intl.RelativeTimeFormat("en", { numeric: "auto" });

export function timeAgo(timestampMs: number, now = Date.now()): string {
	const seconds = Math.round((timestampMs - now) / 1000);
	const absolute = Math.abs(seconds);
	for (const [unit, size] of units) {
		if (absolute >= size) {
			return relative.format(Math.round(seconds / size), unit);
		}
	}
	return "just now";
}

export function errorMessage(error: unknown): string {
	return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}
