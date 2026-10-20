#!/usr/bin/env python3
"""Inventory locally resolved Cargo/npm licences. No licence lookup service is used."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

MARKER = Path(__file__).resolve().parents[1]
ROOT = MARKER.parent
ALLOWED = {
    "MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib",
    "Unicode-3.0", "Unicode-DFS-2016", "0BSD",
}


def command_json(args: list[str], cwd: Path) -> dict:
    result = subprocess.run(args, cwd=cwd, encoding="utf-8", capture_output=True)
    if result.returncode:
        raise RuntimeError(f"{' '.join(args[:3])}: {result.stderr.strip()}")
    return json.loads(result.stdout)


def permitted(expression: str) -> bool:
    """Evaluate SPDX AND/OR with permissive alternatives, rather than substring matching."""
    tokens = re.findall(r"\(|\)|AND|OR|WITH|[A-Za-z0-9.+-]+", expression.replace("/", " OR "))
    index = 0

    def atom() -> bool:
        nonlocal index
        if index >= len(tokens):
            raise ValueError("Incomplete SPDX expression")
        token = tokens[index]
        index += 1
        if token == "(":
            value = either()
            if index >= len(tokens) or tokens[index] != ")":
                raise ValueError("Unclosed SPDX expression")
            index += 1
            return value
        value = token in ALLOWED
        if index < len(tokens) and tokens[index] == "WITH":
            index += 1
            if index >= len(tokens):
                raise ValueError("Missing SPDX exception")
            # LLVM's exception adds permissions to Apache-2.0.
            value = value and tokens[index] == "LLVM-exception"
            index += 1
        return value

    def both() -> bool:
        nonlocal index
        value = atom()
        while index < len(tokens) and tokens[index] == "AND":
            index += 1
            right = atom()
            value = value and right
        return value

    def either() -> bool:
        nonlocal index
        value = both()
        while index < len(tokens) and tokens[index] == "OR":
            index += 1
            right = both()
            value = value or right
        return value

    try:
        return either() and index == len(tokens)
    except ValueError:
        return False


def selected_ids(metadata: dict, names: set[str] | None) -> set[str]:
    selected = set(metadata["workspace_members"]) if names is None else {
        p["id"] for p in metadata["packages"] if p["name"] in names
    }
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    pending = list(selected)
    while pending:
        for dependency in nodes[pending.pop()]["dependencies"]:
            if dependency not in selected:
                selected.add(dependency)
                pending.append(dependency)
    return selected


def licence_files(folder: Path) -> list[Path]:
    paths = []
    for candidate in folder.iterdir():
        if candidate.is_file() and re.match(r"^(licen[cs]e|copying|notice)([._-]|$)", candidate.name, re.I):
            paths.append(candidate)
    return sorted(paths)


def resolve_npm(name: str, folder: Path) -> Path | None:
    for parent in [folder, *folder.parents]:
        candidate = parent / "node_modules" / name / "package.json"
        if candidate.is_file():
            return candidate.resolve()
    return None


def source_npm_manifests():
    # Prune before traversal. Never enter private data or generated package trees.
    excluded = {"node_modules", "target", "dist", "pkg"}
    for folder, directories, files in os.walk(MARKER, followlinks=False):
        directories[:] = sorted(name for name in directories
                                if name not in excluded and not name.startswith(".")
                                and not name.lower().startswith("private"))
        if "package.json" in files:
            yield Path(folder) / "package.json"


def npm_packages(seeds: list[Path]):
    """Follow installed dependency edges; do not include unrelated BETA Time packages."""
    seen = set()
    pending = [(p.resolve(), "node_modules" not in p.parts) for p in seeds]
    root = json.loads((ROOT / "package.json").read_text(encoding="utf-8-sig"))
    for name in root.get("devDependencies", {}):
        manifest = resolve_npm(name, ROOT)
        if manifest is None:
            raise RuntimeError(f"Missing root development dependency {name}")
        pending.append((manifest, False))
    while pending:
        manifest, own = pending.pop()
        if manifest in seen:
            continue
        seen.add(manifest)
        yield manifest
        package = json.loads(manifest.read_text(encoding="utf-8-sig"))
        keys = ["dependencies", "optionalDependencies", "peerDependencies"]
        if own:
            keys.append("devDependencies")
        for key in keys:
            for name in package.get(key, {}):
                dependency = resolve_npm(name, manifest.parent)
                if dependency is not None:
                    pending.append((dependency, False))
                elif key not in {"optionalDependencies", "peerDependencies"} and name not in package.get("optionalDependencies", {}):
                    raise RuntimeError(f"Missing npm dependency {package['name']} -> {name}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=MARKER / "THIRD-PARTY-LICENSES.md")
    parser.add_argument("--texts", type=Path, help="Write upstream licence/notice texts outside source control")
    parser.add_argument("--json", type=Path, help="Write machine-readable audit outside source control")
    parser.add_argument("--offline", action="store_true", help="Require cached Cargo metadata sources")
    parser.add_argument("--include-optional", action="store_true", help="Also inventory standalone GPU and pattern tooling workspaces")
    args = parser.parse_args()
    records = {}
    all_resolved = {}
    loader_checked = False
    scopes = [
        ("marker", "x86_64-pc-windows-gnu", None, "Windows GNU workspace"),
        ("marker", "x86_64-pc-windows-msvc", None, "Windows MSVC workspace"),
        ("marker", "x86_64-unknown-linux-gnu", {"beta-marker-cli"}, "Linux CLI"),
        ("marker", "wasm32-unknown-unknown", {"beta-marker-wasm", "beta-marker-floor"}, "Marker/Floor WASM"),
        ("pattern", "x86_64-pc-windows-gnu", None, "Pattern workspace"),
        ("pattern", "x86_64-unknown-linux-gnu", None, "Pattern Linux"),
        ("pattern", "wasm32-unknown-unknown", {"beta-pattern-wasm"}, "Pattern WASM"),
    ]
    if (MARKER / "bench/public/Cargo.toml").is_file():
        scopes.append(("marker/bench/public", "x86_64-unknown-linux-gnu", None, "Public benchmark tooling"))
    if args.include_optional:
        scopes += [
            ("marker/crates/beta-marker-gpu", "x86_64-pc-windows-gnu", None, "Optional GPU experiment"),
            ("marker/tools/open-patterns/rust-check", "x86_64-pc-windows-gnu", None, "Open-pattern tooling"),
        ]
    for workspace, target, names, scope in scopes:
        print(f"Reading {scope} metadata", file=sys.stderr)
        command = ["cargo", "metadata", "--manifest-path", str(ROOT / workspace / "Cargo.toml"),
                   "--format-version", "1", "--filter-platform", target]
        if args.offline:
            command.append("--offline")
        metadata = command_json(command, ROOT)
        selected = selected_ids(metadata, names)
        for pkg in metadata["packages"]:
            folder = Path(pkg["manifest_path"]).parent
            if pkg["name"] == "webview2-com-sys" and pkg["id"] in selected:
                if pkg["version"] != "0.39.1" or hashlib.sha256((folder / "x64/WebView2Loader.dll").read_bytes()).hexdigest() != "86545b66cdb0603bc26b626fb9ad610cb6e71f28d468f5ea66df23b03dda96d5":
                    raise RuntimeError("Re-audit the bundled WebView2 SDK loader and its licence after an SDK change")
                loader_checked = True
            license_value = pkg["license"] or "UNDECLARED"
            if pkg["name"] == "sparrow" and pkg["version"] == "0.3.0" and license_value == "UNDECLARED":
                if not (folder / "LICENSE").read_text(encoding="utf-8").startswith("MIT License"):
                    raise RuntimeError("Sparrow's local MIT licence text did not match")
                license_value = "MIT (LICENSE file; manifest omits licence)"
            key = ("Rust", pkg["name"], pkg["version"], pkg["source"] or "local")
            record = all_resolved.setdefault(key, {
                "ecosystem": "Rust", "name": pkg["name"], "version": pkg["version"],
                "license": license_value, "source": pkg["source"] or "local BETA source",
                "authors": pkg.get("authors", []), "repository": pkg.get("repository"),
                "folder": str(folder), "scopes": set(),
            })
            if pkg["id"] in selected:
                record["scopes"].add(scope)
                records[key] = record
    manifests = sorted(source_npm_manifests())
    tools = os.environ.get("BETA_TOOLS_DIR")
    if tools:
        manifests.append(Path(tools) / "desktop/node_modules/@tauri-apps/cli/package.json")
    for manifest in npm_packages(manifests):
        pkg = json.loads(manifest.read_text(encoding="utf-8-sig"))
        value = pkg.get("license", "UNDECLARED")
        if isinstance(value, dict):
            value = value.get("type", "UNDECLARED")
        if value == "UNDECLARED" and "licenses" in pkg:
            declarations = pkg["licenses"]
            if isinstance(declarations, (str, dict)):
                declarations = [declarations]
            value = " OR ".join(v if isinstance(v, str) else v.get("type", "UNDECLARED") for v in declarations)
        key = ("npm", pkg["name"], pkg.get("version", "unversioned"), "npm")
        is_own = "node_modules" not in manifest.parts or pkg["name"].startswith("@beta/")
        records.setdefault(key, {
            "ecosystem": "npm", "name": pkg["name"], "version": pkg.get("version", "unversioned"),
            "license": value, "source": "local BETA source" if is_own else f"https://www.npmjs.com/package/{pkg['name']}/v/{pkg.get('version', '')}",
            "folder": str(manifest.parent), "scopes": {"npm workspace/build tools"},
            "authors": pkg.get("author", []), "repository": pkg.get("repository"),
        })
    # Do not silently claim uninstalled offline fixture tools were audited.
    missing_tools = [str(p.relative_to(ROOT)) for p in [MARKER / "tools/open-patterns/freesewing", MARKER / "tools/open-patterns/garmentcode"] if not (p / "node_modules").is_dir()]
    if missing_tools:
        raise RuntimeError("Install tool dependencies before the full inventory: " + ", ".join(missing_tools))
    rejected = []
    for record in records.values():
        expression = record["license"]
        exception = record["name"] == "jagua-rs" and record["version"] == "1.0.0" and expression == "MPL-2.0"
        accepted = permitted(expression) or expression.startswith("MIT (LICENSE file;")
        record["decision"] = "MPL exception (unmodified)" if exception else "permissive" if accepted else "REVIEW REQUIRED"
        if not accepted and not exception:
            rejected.append(f"{record['name']} {record['version']}: {expression}")
    ordered = sorted(records.values(), key=lambda r: (r["ecosystem"], r["name"].lower(), r["version"]))
    if not loader_checked:
        raise RuntimeError("The Windows SDK loader audit was not reached")
    excluded = sorted((r for k, r in all_resolved.items() if k not in records), key=lambda r: (r["name"], r["version"]))
    lines = [
        "# Third-party licences: BETA Cutting Room 0.4.0", "",
        "Generated from Cargo metadata, local npm package manifests and installed package.json files.",
        "No network licence service is used. Re-run after lockfile or dependency changes.", "",
        "## Scope and decisions", "",
        "The main inventory covers the Windows GNU/MSVC Rust workspace graphs, Linux CLI,",
        "Pattern workspace, all three WASM modules, Marker npm workspaces and their installed build",
        "dependencies. It also includes optional GPU and open-pattern tooling when requested.",
        "The desktop CLI tool is included when BETA_TOOLS_DIR is set. OS libraries and the",
        "WebView2 Runtime are separately installed prerequisites. The bundled SDK loader",
        "is audited separately below. NSIS's selected compressor is zlib.", "",
        "npm traversal follows Marker package dependency edges plus the root's shared",
        "development tools. Unrelated BETA Time workspace dependencies are not included.", "",
        "SPDX expressions are preserved. The gate requires an allowed MIT, Apache-2.0,",
        "BSD, ISC, Zlib or Unicode alternative. Unlicense/CC0/Boost may appear in dual",
        "declarations; an allowed branch is chosen rather than relabelling the package.",
        "jagua-rs 1.0.0 is the sole accepted MPL-2.0 exception and is used unmodified.",
        "sparrow 0.3.0 omits a manifest licence; its local upstream LICENSE is MIT and was read.",
        "Original Apache-2.0 dom_query 0.28.0 and option-ext 0.2.0 adapters retain upstream",
        "version constraints. They are not copies of upstream MPL helper implementations.", "",
        f"Inventoried packages: **{len(ordered)}**. Review-required selected packages: **{len(rejected)}**.", "",
        "| Ecosystem | Package | Version | Declared licence | Decision |", "|---|---|---|---|---|",
    ]
    for r in ordered:
        safe = lambda s: str(s).replace("|", "\\|").replace("\n", " ")
        lines.append("| " + " | ".join(safe(r[k]) for k in ("ecosystem", "name", "version", "license", "decision")) + " |")
    lines += ["", "## Bundled Windows SDK loader", "",
              "The NSIS desktop package also contains **WebView2Loader.dll 1.0.3800.47**.",
              "Its bytes match the x64 DLL in Microsoft's official SDK of that version.",
              "The SDK licence grants redistribution under BSD three-clause terms; this",
              "is separate from the Rust binding crate's MIT declaration. The loader is",
              "permissive and adds no copyleft exception. It is not the WebView2 Runtime.",
              "The Runtime is a separately installed Microsoft prerequisite, not bundled.", "",
              "- [Exact SDK licence](https://www.nuget.org/packages/Microsoft.Web.WebView2/1.0.3800.47/License).",
              "- [Bundled licence text](deploy/licenses/WebView2-SDK-1.0.3800.47.txt).",
              "- Loader SHA-256: `86545b66cdb0603bc26b626fb9ad610cb6e71f28d468f5ea66df23b03dda96d5`.", ""]
    lines += ["", "## Resolved outside the shipped target graphs", "",
              "The following metadata-only packages are not in the selected build graphs.",
              "In particular, a Linux Tauri desktop is not shipped. Its GTK/WebKit platform",
              "stack has additional system-library terms and needs a separate review.",
              "Do not describe the unrestricted all-platform desktop graph as permissive-only.", "",
              "| Package | Version | Declared licence |", "|---|---|---|"]
    lines.extend(f"| {r['name']} | {r['version']} | {r['license']} |" for r in excluded)
    lines += ["", "## Source and notice delivery", "",
              "Package sources are fixed by the release Cargo/npm lockfiles. Cargo registry",
              "sources are at crates.io; npm package sources are at npmjs.com. The sparrow git",
              "revision is recorded in Cargo.lock. The exact unmodified jagua-rs source is",
              "available from https://crates.io/api/v1/crates/jagua-rs/1.0.0/download.",
              "Retain access to that source when distributing executables under MPL-2.0.",
              "Release packages include THIRD-PARTY-NOTICES.txt with local upstream licence",
              "and notice files. Dataset attributions are separate in NOTICE and fixture READMEs.", "",
              "Regenerate from the repository root with installed npm/tool dependencies:", "",
              "```sh", "python marker/deploy/generate-licenses.py --include-optional --texts <output>/THIRD-PARTY-NOTICES.txt --json <output>/licenses.json", "```", ""]
    args.output.write_text("\n".join(lines), encoding="utf-8")
    if args.texts:
        args.texts.parent.mkdir(parents=True, exist_ok=True)
        with args.texts.open("w", encoding="utf-8") as output:
            output.write("BETA Cutting Room 0.4.0 - third-party licence and notice texts\n\n")
            output.write("Mozilla Public License 2.0 (jagua-rs 1.0.0, unmodified)\n")
            output.write((MARKER / "deploy/licenses/MPL-2.0.txt").read_text(encoding="utf-8") + "\n\n")
            output.write("Microsoft WebView2 SDK loader 1.0.3800.47 - BSD three-clause terms\n")
            output.write((MARKER / "deploy/licenses/WebView2-SDK-1.0.3800.47.txt").read_text(encoding="utf-8") + "\n\n")
            for r in ordered:
                output.write(f"\n{'=' * 72}\n{r['ecosystem']}: {r['name']} {r['version']}\nLicence: {r['license']}\nSource: {r['source']}\n")
                output.write(f"Authors (package manifest): {json.dumps(r.get('authors', []), ensure_ascii=False)}\nRepository: {json.dumps(r.get('repository'), ensure_ascii=False)}\n")
                files = licence_files(Path(r["folder"]))
                if not files and r["source"] == "local BETA source":
                    files = [ROOT / "LICENSE"]
                for path in files:
                    output.write(f"\n--- {path.name} ---\n{path.read_text(encoding='utf-8', errors='replace')}\n")
                if not files:
                    output.write("\nNo root licence file in the local package; see its source distribution and declared SPDX terms.\n")
    if args.json:
        # Omit machine-specific source/cache paths from the transferable audit.
        serializable = [{k: sorted(v) if isinstance(v, set) else v for k, v in r.items() if k != "folder"} for r in ordered]
        args.json.write_text(json.dumps(serializable, indent=2) + "\n", encoding="utf-8")
    print(f"Inventoried {len(ordered)} selected packages; {len(excluded)} metadata-only packages; {len(rejected)} require review.")
    if rejected:
        print("\n".join(rejected), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
