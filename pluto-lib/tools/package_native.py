#!/usr/bin/env python3
"""Package PlutoLib's verified native components as one Mixins bundle."""
import argparse
import json
from pathlib import Path
import sys
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent.parent
COMPONENTS = (
    "pluto_native_creative",
    "pluto_transport_3a",
    "pluto_tool_rules",
    "pluto_component_display_3a",
    "pluto_component_bar",
    "pluto_items_b2",
)
API_FILES = ("api/Cargo.toml", "api/src/lib.rs", "api/src/creative.rs", "api/src/item.rs")


def _value(value):
    """Encode the manifest value types used by the official Mixins format."""
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, list):
        return "[" + ", ".join(_value(v) for v in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{k} = {_value(v)}" for k, v in value.items()) + " }"
    raise ValueError(f"unsupported manifest value: {type(value).__name__}")


def _regular(path: Path) -> bytes:
    if path.is_symlink() or not path.is_file() or any(p.is_symlink() for p in path.parents if p != ROOT.parent):
        raise ValueError(f"expected a regular source file: {path}")
    return path.read_bytes()


def _relative(value: str) -> str:
    path = Path(value)
    if path.is_absolute() or not path.parts or any(part in (".", "..") for part in path.parts):
        raise ValueError(f"invalid bundle path: {value}")
    return path.as_posix()


def _bundle(manifest: dict) -> tuple[bytes, dict[str, bytes]]:
    lines = ["[mod]", f'id = {_value(manifest["id"])}',
             f'version = {_value(manifest["version"])}', "priority = 100", "",
             "[compatibility]", f'horizon_revision = {_value(manifest["horizon"])}', ""]
    files: dict[str, bytes] = {}
    for name in COMPONENTS:
        root = ROOT / "mixins" / name
        mixin = tomllib.loads(_regular(root / "horizon-mixin.toml").decode("utf-8"))
        if (set(mixin) - {"mod", "compatibility", "crate", "module", "injection"}
                or set(mixin["mod"]) - {"id", "version", "priority"}
                or set(mixin["compatibility"]) != {"horizon_revision"}):
            raise ValueError(f"{name}: unsupported manifest section or field")
        if (mixin["mod"]["id"] != name or mixin["mod"]["version"] != manifest["version"]
                or mixin["compatibility"]["horizon_revision"] != manifest["horizon"]):
            raise ValueError(f"{name}: identity, version or Horizon revision differs")
        for section in ("crate", "module", "injection"):
            for original in mixin.get(section, []):
                entry = dict(original)
                entry["id"] = f'{name}-{entry["id"]}'
                if section == "injection":
                    entry.setdefault("priority", mixin["mod"].get("priority", 100))
                for key in ("source", "helper", "path"):
                    if key not in entry:
                        continue
                    rel = _relative(entry[key])
                    entry[key] = f"{name}/{rel}"
                    if key == "path":
                        if name != "pluto_native_creative" or rel != "api":
                            raise ValueError(f"{name}: unsupported crate path {rel}")
                        for item in API_FILES:
                            files[f"{name}/{item}"] = _regular(root / item)
                    else:
                        files[f"{name}/{rel}"] = _regular(root / rel)
                lines.append(f"[[{section}]]")
                lines.extend(f"{key} = {_value(value)}" for key, value in entry.items())
                lines.append("")
    lines.append("")
    result = "\n".join(lines).encode("utf-8")
    tomllib.loads(result.decode("utf-8"))
    return result, files


def build(output: Path) -> None:
    if output.suffix != ".hnmod":
        raise ValueError("output must have the .hnmod extension")
    manifest_bytes = _regular(ROOT / "horizon-native.json")
    manifest = json.loads(manifest_bytes)
    if manifest["id"] != "pluto_native_creative":
        raise ValueError("unexpected native bundle identity")
    source, files = _bundle(manifest)
    prefix = f'mixins/{manifest["id"]}/'
    entries = {"horizon-native.json": manifest_bytes, prefix + "horizon-mixin.toml": source}
    entries.update({prefix + path: content for path, content in files.items()})
    entries[prefix + "NOTICE.md"] = (
        b"Historical source attribution; the legacy network adapters are not shipped in this bundle.\n\n"
        + _regular(ROOT / "mixins/pluto_items/NOTICE.md")
    )
    with output.open("xb") as destination:
        with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for name, content in sorted(entries.items()):
                entry = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
                entry.compress_type = zipfile.ZIP_DEFLATED
                entry.external_attr = 0o100644 << 16
                archive.writestr(entry, content)


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
