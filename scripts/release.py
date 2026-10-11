#!/usr/bin/env python3
import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BRANCH = "master"

FILES = [
    ("package.json", r'("name": "vrccreationapp",\s*"version": ")%s(")', 1),
    ("package-lock.json", r'("name": "vrccreationapp",\s*"version": ")%s(")', 2),
    ("src-tauri/Cargo.toml", r'(?m)^(version = ")%s(")$', 1),
    ("src-tauri/Cargo.lock", r'(name = "vrccreationapp"\nversion = ")%s(")', 1),
    ("src-tauri/tauri.conf.json", r'(?m)^(\s*"version": ")%s(",?)$', 1),
]


def parse_version(text):
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", text)
    if not match:
        return None
    return tuple(int(part) for part in match.groups())


def bump_text(text, old, new, pattern, count):
    expression = re.compile(pattern % re.escape(old))
    found = len(expression.findall(text))
    if found != count:
        raise ValueError(f"expected {count} place(s) with version {old}, found {found}")
    return expression.sub(lambda match: match.group(1) + new + match.group(2), text)


def commit_lines(entries):
    lines = []
    for short, subject in entries:
        subject = subject.strip()
        if not subject or re.match(r"Release \d", subject):
            continue
        lines.append(f"- {subject} ({short})")
    return lines


def make_notes(description, lines, fallback=""):
    parts = []
    if description.strip():
        parts.append(description.strip())
    if lines:
        parts.append("### Changes\n" + "\n".join(lines))
    elif fallback.strip():
        parts.append(fallback.strip())
    return "\n\n".join(parts) + "\n"


def changelog_section(text, version):
    heading = re.compile(rf"(?m)^## {re.escape(version)}( \(unreleased\))?[ \t]*$")
    match = heading.search(text)
    if not match:
        return ""
    rest = text[match.end():]
    following = re.search(r"(?m)^## ", rest)
    return (rest[: following.start()] if following else rest).strip()


def update_changelog(text, version, notes):
    unreleased = re.compile(rf"(?m)^## {re.escape(version)} \(unreleased\)[ \t]*$")
    if unreleased.search(text):
        return unreleased.sub(f"## {version}", text, count=1)
    if re.search(rf"(?m)^## {re.escape(version)}[ \t]*$", text):
        raise ValueError(f"CHANGELOG.md already has a section for {version}")
    section = f"## {version}\n\n{notes.strip()}\n\n"
    marker = "# Changelog\n\n"
    if text.startswith(marker):
        return marker + section + text[len(marker):]
    return section + text


def latest_version(tags):
    versions = [(parse_version(tag[1:]), tag) for tag in tags if tag.startswith("v") and parse_version(tag[1:])]
    if not versions:
        return None
    return max(versions)[1]


def git(*args, check=True):
    result = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True)
    if check and result.returncode != 0:
        raise RuntimeError(f"git {' '.join(args)} failed: {result.stderr.strip() or result.stdout.strip()}")
    return result.stdout.strip()


def current_version():
    path = ROOT / "src-tauri" / "tauri.conf.json"
    return json.loads(path.read_text(encoding="utf-8"))["version"]


def fail(message):
    print(f"Error: {message}", file=sys.stderr)
    sys.exit(1)


def preflight(version):
    new = parse_version(version)
    if new is None:
        fail("the version must look like 1.2.3")
    changed = git("status", "--porcelain", "--untracked-files=no")
    if changed:
        fail("there are uncommitted changes to tracked files. Commit or stash them first:\n" + changed)
    branch = git("rev-parse", "--abbrev-ref", "HEAD")
    if branch != BRANCH:
        fail(f"releases are made from {BRANCH}, but you are on {branch}")
    try:
        git("fetch", "origin", "--tags", "--quiet")
    except RuntimeError as error:
        fail(f"could not reach GitHub to check for new work: {error}")
    behind = git("rev-list", "--count", f"HEAD..origin/{BRANCH}")
    if behind != "0":
        fail(f"origin/{BRANCH} has {behind} commit(s) you do not have. Pull them first")
    tags = git("tag", "--list").splitlines()
    if f"v{version}" in tags:
        fail(f"the tag v{version} already exists")
    previous = latest_version(tags)
    if previous and new <= parse_version(previous[1:]):
        fail(f"{version} is not newer than the latest release {previous[1:]}")
    have = current_version()
    have_tuple = parse_version(have)
    if have_tuple and new < have_tuple:
        fail(f"the app is already at {have}, which is newer than {version}")
    return previous, have


def main():
    parser = argparse.ArgumentParser(description="Tag and push a release. GitHub Actions builds and publishes it.")
    parser.add_argument("version", help="the new version, like 1.0.1")
    parser.add_argument("description", nargs="?", default="", help="optional text for the top of the release notes")
    parser.add_argument("--dry-run", action="store_true", help="show what would happen and change nothing")
    parser.add_argument("--yes", action="store_true", help="do not ask before committing and pushing")
    args = parser.parse_args()

    version = args.version.removeprefix("v")
    previous, have = preflight(version)

    if previous:
        entries = [
            line.split("\x1f", 1)
            for line in git("log", f"{previous}..HEAD", "--no-merges", "--format=%h%x1f%s").splitlines()
            if "\x1f" in line
        ]
        notes = make_notes(args.description, commit_lines(entries))
    else:
        changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
        notes = make_notes(args.description, [], changelog_section(changelog, version))

    print(f"Release v{version} (the app is at {have}, previous release: {previous or 'none'})\n")
    print("Release notes:\n")
    print(notes)
    plan = [f"bump the version to {version}" if have != version else f"keep the version at {version}"]
    plan += ["update CHANGELOG.md", f'commit "Release {version}"', f"tag v{version}", f"push {BRANCH} and v{version}"]
    print("This will: " + ", ".join(plan) + ".")
    if args.dry_run:
        print("\nDry run, nothing was changed.")
        return
    if not args.yes and input("\nContinue? [y/N] ").strip().lower() != "y":
        print("Cancelled, nothing was changed.")
        return

    touched = []
    if have != version:
        for name, pattern, count in FILES:
            path = ROOT / name
            try:
                updated = bump_text(path.read_text(encoding="utf-8"), have, version, pattern, count)
            except ValueError as error:
                fail(f"{name}: {error}. Nothing was committed; restore the files with git checkout.")
            path.write_text(updated, encoding="utf-8", newline="")
            touched.append(name)
    changelog_path = ROOT / "CHANGELOG.md"
    try:
        changelog = update_changelog(changelog_path.read_text(encoding="utf-8"), version, notes)
    except ValueError as error:
        fail(f"{error}. Nothing was committed; restore the files with git checkout.")
    changelog_path.write_text(changelog, encoding="utf-8", newline="")
    touched.append("CHANGELOG.md")

    git("add", *touched)
    git("commit", "-m", f"Release {version}")
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False, encoding="utf-8") as handle:
        handle.write(notes)
        notes_file = handle.name
    git("tag", "-a", f"v{version}", "-F", notes_file, "--cleanup=verbatim")
    Path(notes_file).unlink(missing_ok=True)
    print(f"Committed and tagged v{version}. Pushing...")
    try:
        git("push", "origin", BRANCH)
        git("push", "origin", f"v{version}")
    except RuntimeError as error:
        fail(f"{error}\nThe commit and tag exist locally. Retry with: git push origin {BRANCH} v{version}")
    print(f"Done. GitHub Actions is now building v{version}; the release appears when the builds finish.")
    print(f"Progress: https://github.com/zachup100/VRCCreationApp/actions")


if __name__ == "__main__":
    main()
