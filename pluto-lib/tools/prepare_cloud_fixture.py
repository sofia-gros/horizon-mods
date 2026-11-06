#!/usr/bin/env python3
"""Prepare the actual shared cloud descriptor against exact official core types."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib
from prepare_render_fixture import clean_official, verify

FIXTURE_IDENTITY = ("pluto-cloud-descriptor", "0.0.0", None, None)


def prepare(horizon: Path, output: Path):
    root = Path(__file__).resolve().parents[1]
    horizon = horizon.resolve()
    pin = tomllib.loads((root / "mixins/pluto_native_creative/horizon-mixin.toml").read_text())["compatibility"]["horizon_revision"]
    head = clean_official(horizon, pin)
    official_lock = (horizon / "Cargo.lock").read_bytes()
    if output.exists():
        raise ValueError("Fixture output must not already exist")
    output.mkdir(parents=True)
    sources = {"cloud-descriptor.rs": root / "mixins/pluto_native_creative/cloud-descriptor.rs",
               "potion-facts.rs": root / "mixins/pluto_native_creative/potion-facts.rs",
               "tests.rs": root / "tests/native/cloud-descriptor.rs"}
    for name, source in sources.items():
        shutil.copyfile(source, output / name)
    (output / "Cargo.toml").write_text(
        '[package]\nname = "pluto-cloud-descriptor"\nversion = "0.0.0"\nedition = "2024"\n'
        '[workspace]\n[lib]\npath = "lib.rs"\n'
        '[dependencies]\nhorizon-client-core = { path = ' + json.dumps(str(horizon / 'crates/horizon-client-core')) + ' }\n')
    patches = tomllib.loads((horizon / "Cargo.toml").read_text()).get("patch", {}).get("crates-io", {})
    with (output / "Cargo.toml").open("a") as manifest:
        manifest.write("\n[patch.crates-io]\n")
        for name, entry in patches.items():
            if set(entry) != {"path"}:
                raise ValueError(f"Unsupported official patch declaration: {name}")
            path = (horizon / entry["path"]).resolve()
            if not path.is_relative_to(horizon):
                raise ValueError(f"Official patch escapes its checkout: {name}")
            manifest.write(json.dumps(name) + " = { path = " + json.dumps(str(path)) + " }\n")
    (output / "lib.rs").write_text(
        '#![forbid(unsafe_code)]\nextern crate self as horizon_client;\n'
        'pub mod pluto_potions { include!("potion-facts.rs"); }\n'
        'pub mod pluto_cloud { include!("cloud-descriptor.rs"); #[cfg(test)] mod tests { include!("tests.rs"); } }\n')
    evidence = {"horizon_source": str(horizon), "horizon_revision": head,
        "official_lock_sha256": hashlib.sha256(official_lock).hexdigest(),
        "sources_sha256": {name: hashlib.sha256((output / name).read_bytes()).hexdigest()
                           for name in (*sources, "Cargo.toml", "lib.rs")},
        "validation": "Exact official dependencies resolved offline. Descriptor fixture only; native game factory and world behavior are not exercised."}
    (output / "Cargo.lock").write_bytes(official_lock)
    subprocess.run(["cargo", "metadata", "--offline", "--manifest-path", str((output / "Cargo.toml").resolve()),
                    "--format-version", "1"], check=True, stdout=subprocess.DEVNULL)
    (output / "fixture-sources.json").write_text(json.dumps(evidence, indent=2) + "\n")
    verify(output, FIXTURE_IDENTITY)
    print(output.resolve())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--horizon-source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify-lock", action="store_true")
    args = parser.parse_args()
    if args.verify_lock:
        verify(args.output, FIXTURE_IDENTITY)
        print("Cloud fixture inputs and exact official dependency identities verified")
    elif args.horizon_source is None:
        parser.error("--horizon-source is required when preparing a fixture")
    else:
        prepare(args.horizon_source, args.output)
