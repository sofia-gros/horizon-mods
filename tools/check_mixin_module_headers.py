#!/usr/bin/env python3
"""Statically inspect manifest-referenced Mixins support-module headers."""

import argparse
from pathlib import Path
import re
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN = re.compile(r"^[ \t]*(?:#!|//!|/\*!)", re.MULTILINE)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", action="append", type=Path, help="inspect this manifest; repeatable")
    args = parser.parse_args()
    if args.manifest:
        manifests = [path if path.is_absolute() else ROOT / path for path in args.manifest]
    else:
        manifests = sorted(path for root in (ROOT / "tconstruct", ROOT / "pluto-lib")
                           if root.exists() for path in root.rglob("horizon-mixin.toml"))
    if not manifests:
        print("No Mixins manifests found", file=sys.stderr)
        return 1
    checked = 0
    failed = False
    for manifest in manifests:
        try:
            data = tomllib.loads(manifest.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
            print(f"{manifest}: cannot read manifest: {error}", file=sys.stderr)
            failed = True
            continue
        for module in data.get("module", []):
            source = module.get("source")
            if not source:
                continue
            path = (manifest.parent / source).resolve()
            try:
                path.relative_to(ROOT)
                text = path.read_text(encoding="utf-8")
            except (ValueError, OSError, UnicodeError) as error:
                print(f"{manifest}: invalid module source {source}: {error}", file=sys.stderr)
                failed = True
                continue
            checked += 1
            for match in FORBIDDEN.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                print(f"{path.relative_to(ROOT)}:{line}: module source has an inner doc, inner attribute or shebang", file=sys.stderr)
                failed = True
    print(f"Inspected {checked} module references in {len(manifests)} manifests")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
