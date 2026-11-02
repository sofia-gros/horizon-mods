#!/usr/bin/env python3
"""Build the source-only PlutoLib native bundle; never include demo hooks."""
import argparse
import json
from pathlib import Path
import sys
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent.parent
BUNDLE = ROOT / "mixins" / "pluto_native_creative"
FILES = ("horizon-mixin.toml", "api/Cargo.toml", "api/src/lib.rs", "api/src/creative.rs", "api/src/item.rs")


def build(output: Path) -> None:
    if output.suffix != ".hnmod":
        raise ValueError("output must have the .hnmod extension")
    manifest = json.loads((ROOT / "horizon-native.json").read_text(encoding="utf-8"))
    mixin = tomllib.loads((BUNDLE / FILES[0]).read_text(encoding="utf-8"))
    if (manifest["id"] != BUNDLE.name or manifest["id"] != mixin["mod"]["id"]
            or manifest["version"] != mixin["mod"]["version"]
            or manifest["horizon"] != mixin["compatibility"]["horizon_revision"]):
        raise ValueError("native manifest and mixin identity or revision differ")
    sources = [("horizon-native.json", ROOT / "horizon-native.json")]
    sources += [(f"mixins/{manifest['id']}/{name}", BUNDLE / name) for name in FILES]
    for _, path in sources:
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"expected a regular source file: {path}")
    # Exclusive creation prevents accidental replacement of an existing package.
    with output.open("xb") as destination:
        with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for name, source in sources:
                entry = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
                entry.compress_type = zipfile.ZIP_DEFLATED
                entry.external_attr = 0o100644 << 16
                archive.writestr(entry, source.read_bytes())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="new .hnmod path; parent must exist")
    args = parser.parse_args()
    try:
        build(args.output)
    except (OSError, ValueError, KeyError, tomllib.TOMLDecodeError) as error:
        print(f"package error: {error}", file=sys.stderr)
        return 1
    print(f"Built {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
