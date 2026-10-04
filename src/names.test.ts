import { describe, expect, it } from "vitest";
import { projectNameProblem } from "./names";

describe("projectNameProblem", () => {
	it("accepts ordinary names", () => {
		for (const name of ["My Project", "a-b_c.1", "Welt 2", "console", "Com", "lpt10"]) {
			expect(projectNameProblem(name)).toBeNull();
		}
	});

	it("rejects empty names", () => {
		expect(projectNameProblem("")).not.toBeNull();
		expect(projectNameProblem("   ")).not.toBeNull();
	});

	it("rejects characters that cannot be in a folder name", () => {
		for (const name of ["a/b", "a\\b", "a:b", "a*b", "a?b", 'a"b', "a<b", "a>b", "a|b", "a\nb"]) {
			expect(projectNameProblem(name)).not.toBeNull();
		}
	});

	it("rejects names ending in a dot or a space", () => {
		expect(projectNameProblem("name.")).not.toBeNull();
		expect(projectNameProblem("name ")).toBeNull();
	});

	it("rejects the names Windows reserves, with or without an extension", () => {
		for (const name of ["CON", "con", "PRN", "AUX", "NUL", "COM1", "com9", "LPT1", "lpt0", "NUL.txt", "con.anything"]) {
			expect(projectNameProblem(name)).not.toBeNull();
		}
	});

	it("limits the length to 100 characters", () => {
		expect(projectNameProblem("x".repeat(100))).toBeNull();
		expect(projectNameProblem("x".repeat(101))).not.toBeNull();
	});
});
