import hashlib
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TAURI = os.path.join(ROOT, "src-tauri")
TARGET = "x86_64-pc-windows-msvc"
LICENSE_NAME = re.compile(r"^(licen[sc]e|copying|unlicense|notice)([-_. ].*)?$", re.IGNORECASE)
OUTPUT = os.path.join(ROOT, "THIRD-PARTY-LICENSES.txt")


def read_text(path):
    with open(path, "rb") as handle:
        return handle.read().decode("utf-8", errors="replace").replace("\r\n", "\n").strip()


def license_files(directory, search_up):
    found = []
    current = directory
    for _ in range(search_up + 1):
        if os.path.isdir(current):
            for name in sorted(os.listdir(current)):
                path = os.path.join(current, name)
                if os.path.isfile(path) and LICENSE_NAME.match(name):
                    found.append(path)
        if found:
            break
        parent = os.path.dirname(current)
        if parent == current:
            break
        current = parent
    return found


def rust_packages():
    output = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", TARGET, "--manifest-path", os.path.join(TAURI, "Cargo.toml")],
        check=True,
        capture_output=True,
        encoding="utf-8",
        errors="replace",
    ).stdout
    metadata = json.loads(output)
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root = metadata["resolve"]["root"]
    seen = set()
    stack = [root]
    while stack:
        current = stack.pop()
        if current in seen:
            continue
        seen.add(current)
        for dependency in nodes[current]["deps"]:
            kinds = [entry.get("kind") for entry in dependency["dep_kinds"]]
            if None in kinds:
                stack.append(dependency["pkg"])
    result = []
    for identifier in seen:
        if identifier == root:
            continue
        package = packages[identifier]
        directory = os.path.dirname(package["manifest_path"])
        from_git = (package.get("source") or "").startswith("git+")
        result.append(
            {
                "name": package["name"],
                "version": package["version"],
                "license": package.get("license") or "see license files",
                "repository": package.get("repository") or "",
                "authors": ", ".join(author.split("<")[0].strip() for author in package.get("authors") or []),
                "files": license_files(directory, 2 if from_git else 0),
                "kind": "Rust",
            }
        )
    return result


def npm_packages():
    with open(os.path.join(ROOT, "package-lock.json"), encoding="utf-8") as handle:
        lock = json.load(handle)
    result = []
    for path, entry in lock.get("packages", {}).items():
        if not path or entry.get("dev") or entry.get("optional") and entry.get("dev"):
            continue
        directory = os.path.join(ROOT, path)
        manifest_path = os.path.join(directory, "package.json")
        if not os.path.isfile(manifest_path):
            continue
        with open(manifest_path, encoding="utf-8") as handle:
            manifest = json.load(handle)
        license_value = manifest.get("license") or entry.get("license") or "see license files"
        if isinstance(license_value, dict):
            license_value = license_value.get("type", "see license files")
        repository = manifest.get("repository")
        if isinstance(repository, dict):
            repository = repository.get("url", "")
        result.append(
            {
                "name": manifest.get("name", os.path.basename(path)),
                "version": manifest.get("version", entry.get("version", "")),
                "license": str(license_value),
                "repository": repository or "",
                "files": license_files(directory, 0),
                "kind": "npm",
            }
        )
    return result


MIT_TEMPLATE = """MIT License

Copyright (c) {holder}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."""

BSD3_TEMPLATE = """BSD 3-Clause License

Copyright (c) {holder}

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE."""


def find_text(texts, marker):
    for digest, text in texts.items():
        if marker in text[:400]:
            return digest
    return None


def fallback_hashes(package, texts):
    tokens = [token for token in re.split(r"[\s()/]+|\bOR\b|\bAND\b|\bWITH\b", package["license"]) if token]
    holder = package.get("authors") or f"the {package['name']} authors"
    found = []
    for token in tokens:
        text = None
        if token == "MIT":
            text = MIT_TEMPLATE.format(holder=holder)
        elif token == "BSD-3-Clause":
            text = BSD3_TEMPLATE.format(holder=holder)
        elif token == "Apache-2.0":
            digest = find_text(texts, "Apache License")
            if digest:
                found.append(digest)
            continue
        elif token == "MPL-2.0":
            digest = find_text(texts, "Mozilla Public License")
            if digest:
                found.append(digest)
            continue
        if text:
            digest = hashlib.sha256(text.encode("utf-8")).hexdigest()
            texts.setdefault(digest, text)
            found.append(digest)
    return found


def main():
    packages = sorted(rust_packages() + npm_packages(), key=lambda item: (item["kind"], item["name"].lower(), item["version"]))
    texts = {}
    usage = {}
    missing = []
    for package in packages:
        hashes = []
        for path in package["files"]:
            text = read_text(path)
            if not text:
                continue
            digest = hashlib.sha256(text.encode("utf-8")).hexdigest()
            texts.setdefault(digest, text)
            hashes.append(digest)
        package["hashes"] = list(dict.fromkeys(hashes))
        if not package["hashes"]:
            missing.append(package)
        for digest in package["hashes"]:
            usage.setdefault(digest, []).append(f"{package['name']} {package['version']}")

    still_missing = []
    for package in missing:
        hashes = fallback_hashes(package, texts)
        package["hashes"] = list(dict.fromkeys(hashes))
        for digest in package["hashes"]:
            usage.setdefault(digest, []).append(f"{package['name']} {package['version']}")
        if not package["hashes"]:
            still_missing.append(package)
    missing = still_missing

    lines = []
    lines.append("THIRD-PARTY LICENSES")
    lines.append("")
    lines.append("This application includes the open-source software listed below.")
    lines.append("Rust crates are those compiled into the Windows build, and npm packages are those bundled into the interface.")
    lines.append("Where a crate ships no license file, the standard text for its declared license is given with the authors named in its manifest.")
    lines.append("")
    lines.append("=" * 78)
    lines.append("PACKAGES")
    lines.append("=" * 78)
    for package in packages:
        lines.append("")
        lines.append(f"{package['name']} {package['version']} ({package['kind']})")
        lines.append(f"  License: {package['license']}")
        if package["repository"]:
            lines.append(f"  Source: {package['repository']}")
    lines.append("")
    lines.append("=" * 78)
    lines.append("LICENSE TEXTS")
    lines.append("=" * 78)
    notices = os.path.join(ROOT, "THIRD-PARTY-NOTICES.md")
    if os.path.isfile(notices):
        lines.append("")
        lines.append("-" * 78)
        lines.append("vrc-get, ALCOM project templates (bundled project templates)")
        lines.append("-" * 78)
        lines.append("")
        lines.append(read_text(notices))
    for digest, text in sorted(texts.items(), key=lambda item: usage[item[0]][0].lower()):
        names = usage[digest]
        shown = ", ".join(names[:12]) + (f" and {len(names) - 12} more" if len(names) > 12 else "")
        lines.append("")
        lines.append("-" * 78)
        lines.append(f"Used by: {shown}")
        lines.append("-" * 78)
        lines.append("")
        lines.append(text)
    lines.append("")
    with open(OUTPUT, "w", encoding="utf-8", newline="\n") as handle:
        handle.write("\n".join(lines))
    print(f"{len(packages)} packages, {len(texts)} unique license texts, {len(missing)} without a license file")
    for package in missing:
        print(f"  no license file: {package['kind']} {package['name']} {package['version']} ({package['license']})")


if __name__ == "__main__":
    sys.exit(main())
