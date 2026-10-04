import type { PackageSource } from "./api";

export type RepoFilter = "all" | PackageSource;

export const filterOptions: { value: RepoFilter; label: string }[] = [
	{ value: "all", label: "Multiple Repositories" },
	{ value: "official", label: "Official" },
	{ value: "curated", label: "Curated" },
	{ value: "community", label: "Community" },
	{ value: "local", label: "Local User Packages" },
];

export const sourceLabels: Record<PackageSource, string> = {
	official: "Official",
	curated: "Curated",
	community: "Community",
	local: "Local",
};
