const INVALID = /[\\/:*?"<>|\u0000-\u001f]/;
const RESERVED = /^(con|prn|aux|nul|com[0-9]|lpt[0-9])$/i;
const MAX_LENGTH = 100;

export function projectNameProblem(value: string): string | null {
	const trimmed = value.trim();
	if (!trimmed) {
		return "Enter a project name";
	}
	if (INVALID.test(trimmed)) {
		return "The name contains characters that are not allowed";
	}
	if (/[. ]$/.test(trimmed)) {
		return "The name cannot end with a dot or a space";
	}
	if (RESERVED.test(trimmed.split(".")[0])) {
		return "Windows does not allow this name for a folder";
	}
	if ([...trimmed].length > MAX_LENGTH) {
		return "The name is too long";
	}
	return null;
}
