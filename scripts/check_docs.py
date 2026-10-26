#!/usr/bin/env python3
"""Check documentation links and versions, compile .rl examples, and type-check Rust examples."""
from pathlib import Path
import json
import os
import re
import subprocess
import sys
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
PAGES = ROOT / "docs/docs"
FENCE = re.compile(r"^(`{3,}|~{3,})(\w+)([^\n]*)\n(.*?)^\1[ \t]*$", re.M | re.S)

def run(args, **kwargs):
    subprocess.run(args, check=True, cwd=ROOT, **kwargs)

def main():
    run([sys.executable, "scripts/check_versions.py"])
    config = tomllib.loads((ROOT / "docs/zensical.toml").read_text())["project"]
    def nav_entries(value):
        if isinstance(value, str):
            if value.startswith(("https://", "http://")): return
            assert (PAGES / value).is_file(), f"Missing navigation page: {value}"
        elif isinstance(value, dict):
            for child in value.values(): nav_entries(child)
        elif isinstance(value, list):
            for child in value: nav_entries(child)
    nav_entries(config["nav"])
    examples = []
    rust = []
    for page in sorted(PAGES.rglob("*.md")):
        source = page.read_text()
        assert len(re.findall(r"^(?:`{3,}|~{3,})", source, re.M)) % 2 == 0, f"Unclosed code fence: {page}"
        prose = FENCE.sub("", source)
        for target in re.findall(r"\]\(([^\s)]+)", prose):
            if re.match(r"[a-zA-Z][a-zA-Z0-9+.-]*:", target) or target.startswith("#"): continue
            name = target.split("#", 1)[0]
            assert (page.parent / name).exists(), f"Broken link in {page.relative_to(ROOT)}: {target}"
        blocks = list(FENCE.finditer(source))
        examples.append((page, [(b.group(2), b.group(3), b.group(4)) for b in blocks if b.group(2) in ("ratslang", "rl", "awk")]))
        rust.extend((page, b.group(4)) for b in blocks if b.group(2) == "rust")
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT))
    target = Path(metadata["target_directory"])
    run(["cargo", "build", "-p", "ratslang-cli", "--quiet"])
    binary = target / "debug" / ("ratslang.exe" if os.name == "nt" else "ratslang")
    count = 0
    for page, blocks in examples:
        with tempfile.TemporaryDirectory(prefix="ratslang-docs-rl-") as folder:
            folder = Path(folder)
            empty_schema = folder / "empty-schema.rl"
            empty_schema.write_text("\n")
            # Named snippets supply include/schema fixtures for the other snippets on this page.
            for _, attrs, source in blocks:
                title = re.search(r'title="([^"/]+\.rl)"', attrs)
                if title: (folder / title.group(1)).write_text(source)
            for index, (_, attrs, source) in enumerate(blocks):
                title = re.search(r'title="([^"/]+\.rl)"', attrs)
                path = folder / (title.group(1) if title else f"example-{index}.rl")
                path.write_text(source)
                result = subprocess.run([str(binary), "check", str(path), "--schema", str(empty_schema)], cwd=ROOT, capture_output=True, text=True)
                if result.returncode:
                    raise SystemExit(f"{page.relative_to(ROOT)}, example {index + 1}:\n{result.stderr}")
                count += 1
    with tempfile.TemporaryDirectory(prefix="ratslang-docs-rust-") as folder:
        folder = Path(folder)
        (folder / "src/bin").mkdir(parents=True)
        (folder / "Cargo.toml").write_text(
            '[package]\nname = "ratslang-doc-examples"\nversion = "0.0.0"\nedition = "2024"\n'
            '[dependencies]\n'
            f'ratslang = {{ path = {json.dumps(str(ROOT / "ratslang"))}, features = ["serde"] }}\n'
            'serde = { version = "1", features = ["derive"] }\nanyhow = "1"\n'
        )
        for index, (page, source) in enumerate(rust):
            (folder / f"src/bin/example{index}.rs").write_text(
                f"// {page.relative_to(ROOT)}\n#![allow(unused)]\n"
                "fn main() -> anyhow::Result<()> {\n"
                + source + "\nOk(())\n}\n"
            )
        run(["cargo", "check", "--manifest-path", str(folder / "Cargo.toml"), "--bins", "--quiet", "--offline"],
            env={**os.environ, "CARGO_TARGET_DIR": str(target)})
    print(f"Documentation checked: {len(examples)} pages, {count} Ratslang examples, {len(rust)} Rust examples.")

if __name__ == "__main__":
    main()
