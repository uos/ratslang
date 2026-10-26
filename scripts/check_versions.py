#!/usr/bin/env python3
"""Check that versions named in the docs and READMEs match their sources.

Every version the docs name is read from the source tree:

- the ratslang crate version and minimum Rust versions, as Cargo resolves them
  for ratslang and ratslang-lsp from their manifests and the workspace
- the tree-sitter-ratslang version the LSP resolves to, the grammar commit it
  was published from (its .cargo_vcs_info.json) and its parser ABI (its
  src/parser.c). The commit also pins the grammar the docs highlighter builds.
- the minimum VS Code version, from vscode/package.json

Run with --fix to rewrite stale versions, otherwise any mismatch exits with
status 1. bump_version.sh runs it with --fix.

Usage:
    python3 scripts/check_versions.py         # check only
    python3 scripts/check_versions.py --fix   # rewrite stale versions
"""
from pathlib import Path
import json
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def metadata():
    return json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1"], cwd=ROOT))


def package(packages, name):
    """Version and rust-version of a workspace crate, with inherited fields resolved."""
    found = next(p for p in packages if p["name"] == name)
    return {"version": found["version"], "rust-version": found["rust_version"]}


def grammar(packages):
    """The tree-sitter-ratslang package the LSP resolves to: version, commit and ABI."""
    found = next(p for p in packages if p["name"] == "tree-sitter-ratslang")
    folder = Path(found["manifest_path"]).parent
    commit = json.loads((folder / ".cargo_vcs_info.json").read_text())["git"]["sha1"]
    abi = re.search(r"#define LANGUAGE_VERSION (\d+)", (folder / "src/parser.c").read_text()).group(1)
    return found["version"], commit, abi


def vscode_version():
    engine = json.loads((ROOT / "vscode/package.json").read_text())["engines"]["vscode"]
    major, minor = re.match(r"\D*(\d+)\.(\d+)", engine).groups()
    return f"{major}.{minor}"


def main():
    fix = "--fix" in sys.argv[1:]
    packages = metadata()["packages"]
    core = package(packages, "ratslang")
    lsp = package(packages, "ratslang-lsp")
    grammar_version, grammar_commit, abi = grammar(packages)
    # Each pattern has two groups: the fixed text before the version, and the version.
    rules = [
        (r'^(ratslang = (?:\{ version = )?")(\d[^"]*)(?=")', core["version"], "ratslang crate version"),
        (r"(core compiler requires Rust\s+)(\d[\d.]*\d)", core["rust-version"], "core rust-version"),
        (r"(LSP requires Rust\s+)(\d[\d.]*\d)", lsp["rust-version"], "LSP rust-version"),
        (r"(Install Rust\s+)(\d[\d.]*\d)", lsp["rust-version"], "LSP rust-version"),
        (r"(`tree-sitter-ratslang`\s+)(\d[\d.]*\d)", grammar_version, "tree-sitter-ratslang version"),
        (r'(tree-sitter-ratslang", rev = ")([0-9a-f]+)', grammar_commit, "tree-sitter-ratslang commit"),
        (r"(tree-sitter-ratslang checkout )([0-9a-f]+)", grammar_commit, "tree-sitter-ratslang commit"),
        (r"(tree-sitter-ratslang@)([0-9a-f]+)", grammar_commit, "tree-sitter-ratslang commit"),
        (r"(ABI\s+)(\d+)", abi, "parser ABI"),
        (r"(VS Code\s+)(\d+\.\d+)(?= or newer)", vscode_version(), "VS Code version"),
    ]
    rules = [(re.compile(pattern, re.M), expected, what) for pattern, expected, what in rules]
    files = [ROOT / "README.md", ROOT / "ratslang-lsp/README.md", ROOT / "vscode/README.md"]
    files += [ROOT / "docs/pygments-ratslang/pyproject.toml"]
    files += sorted((ROOT / "docs/docs").rglob("*.md"))
    stale = []
    for path in files:
        if not path.is_file():
            continue
        text = path.read_text()
        updated = text
        for pattern, expected, what in rules:
            for match in pattern.finditer(text):
                if match.group(2) != expected:
                    line = text.count("\n", 0, match.start(2)) + 1
                    stale.append(f"{path.relative_to(ROOT)}:{line}: {what} is {match.group(2)}, expected {expected}")
            updated = pattern.sub(lambda m: m.group(1) + expected, updated)
        if fix and updated != text:
            path.write_text(updated)
    for entry in stale:
        print(("fixed " if fix else "") + entry)
    if stale and not fix:
        print("Run `python3 scripts/check_versions.py --fix` to update them.")
        sys.exit(1)


if __name__ == "__main__":
    main()
