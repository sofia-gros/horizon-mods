#!/usr/bin/env python3
"""Prepare a locked fixture using actual official Text and production codec.

Optionally include the read-only actual play-packet projection. No mock host
types, native screen state, callback phase execution or network IO.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
OFFICIAL = "https://github.com/horizon-rust/horizon.git"
TEXT_SOURCE = "crates/horizon-client-core/src/text.rs"
LOCAL = {("pluto-guide-text-tests", "0.0.0", None, None), ("pluto-inventory-api", "0.1.0", None, None)}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def identity(package):
    return tuple(package.get(key) for key in ("name", "version", "source", "checksum"))


def check_official(host, pin):
    def git(*args):
        return subprocess.check_output(["git", "-C", str(host), *args], text=True).strip()
    if git("remote", "get-url", "origin") != OFFICIAL or git("rev-parse", "HEAD") != pin or git("status", "--porcelain", "--untracked-files=all"):
        raise ValueError("Require the clean selected official Horizon source")


def metadata(output, locked):
    command = ["cargo", "metadata", "--offline", "--manifest-path", str(output / "Cargo.toml"), "--format-version", "1"]
    if locked:
        command.append("--locked")
    return json.loads(subprocess.check_output(command, text=True))


def verify(output):
    evidence = json.loads((output / "fixture-sources.json").read_text())
    host = Path(evidence["official_source"])
    check_official(host, evidence["official_revision"])
    if digest((host / TEXT_SOURCE).read_bytes()) != evidence["official_text_sha256"] or digest((host / "Cargo.lock").read_bytes()) != evidence["official_lock_sha256"]:
        raise ValueError("Official Text/lock drifted")
    for name, sha in evidence.get("official_extra_sha256", {}).items():
        if digest((host/name).read_bytes()) != sha:
            raise ValueError(f"Official extra source drifted: {name}")
    for name, sha in evidence["files_sha256"].items():
        if digest((output / name).read_bytes()) != sha:
            raise ValueError(f"Fixture input drifted: {name}")
    allowed = {identity(p) for p in tomllib.loads((host / "Cargo.lock").read_text())["package"]}
    actual = {identity(p) for p in tomllib.loads((output / "Cargo.lock").read_text())["package"]}
    if actual - allowed - LOCAL or not LOCAL.issubset(actual):
        raise ValueError("Fixture lock differs from official/local package identities")
    resolved = metadata(output, True)
    local_paths = {"pluto-guide-text-tests": output / "Cargo.toml", "pluto-inventory-api": output / "inventory-api/Cargo.toml"}
    found = set()
    for package in resolved["packages"]:
        path = Path(package["manifest_path"]).resolve()
        if package["name"] in local_paths:
            if package["source"] is not None or path != local_paths[package["name"]].resolve():
                raise ValueError("Local package does not resolve to exact fixture source")
            found.add(package["name"])
        elif package["source"] is None and not path.is_relative_to(host):
            raise ValueError("Path dependency outside the official source")
    if found != set(local_paths):
        raise ValueError("Required real API/fixture package missing")
    return evidence


def prepare(host, output, include_payload=False, include_correlation=False):
    include_payload = include_payload or include_correlation
    host, output = host.resolve(), output.resolve()
    pin = tomllib.loads((ROOT / "mixins/pluto_native_creative/horizon-mixin.toml").read_text())["compatibility"]["horizon_revision"]
    check_official(host, pin)
    if output.exists():
        raise ValueError("Output must be new")
    output.mkdir(parents=True)
    source = ROOT / "mixins/pluto_native_creative"
    shutil.copytree(source / "inventory-api", output / "inventory-api", ignore=shutil.ignore_patterns("Cargo.lock", "target"))
    shutil.copyfile(source / "guide-text.rs", output / "guide-text.rs")
    (output / "lib.rs").write_text('pub use horizon_client_core::text;\npub use pluto_inventory_api as pluto_inventory;\n#[path="guide-text.rs"] pub mod guide_text;\n')
    manifest = '[package]\nname="pluto-guide-text-tests"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\npluto-inventory-api={path="inventory-api"}\nhorizon-client-core={path=' + json.dumps(str(host / "crates/horizon-client-core")) + '}\n'
    if include_payload:
        shutil.copyfile(source / "client-payload.rs", output / "client-payload.rs")
        with (output / "lib.rs").open("a") as file:
            file.write('#[path="client-payload.rs"] pub mod client_payload;\n')
        manifest += 'horizon-protocol={path=' + json.dumps(str(host / "crates/horizon-protocol")) + '}\n'
    if include_correlation:
        shutil.copyfile(ROOT / "tests/native/guide_request_projection.rs", output / "guide-request-projection.rs")
        with (output / "lib.rs").open("a") as file:
            file.write('#[cfg(test)] #[path="guide-request-projection.rs"] mod guide_request_projection;\n')
    patches = tomllib.loads((host / "Cargo.toml").read_text()).get("patch", {}).get("crates-io", {})
    manifest += '\n[patch.crates-io]\n'
    for name, entry in patches.items():
        if set(entry) != {"path"}:
            raise ValueError("Unsupported official patch")
        path = (host / entry["path"]).resolve()
        if not path.is_relative_to(host):
            raise ValueError("Patch escapes official source")
        manifest += json.dumps(name) + '={path=' + json.dumps(str(path)) + '}\n'
    (output / "Cargo.toml").write_text(manifest)
    (output / "Cargo.lock").write_bytes((host / "Cargo.lock").read_bytes())
    metadata(output, False)
    evidence = {"official_source": str(host), "official_revision": pin,
        "official_text_sha256": digest((host / TEXT_SOURCE).read_bytes()),
        "official_extra_sha256": {name: digest((host/name).read_bytes()) for name in
            (["crates/horizon-protocol/src/packets/play.rs", "crates/horizon-protocol/src/packets/configuration.rs"] if include_payload else [])},
        "official_lock_sha256": digest((host / "Cargo.lock").read_bytes()),
        "files_sha256": {str(p.relative_to(output)): digest(p.read_bytes()) for p in output.rglob("*") if p.is_file()},
        "scope": "Actual official Text, actual packaged API and production semantic converter" + (" plus actual play-packet projection" if include_payload else "") + (" plus explicit in-process request correlation" if include_correlation else "") + "; no native package apply, observer-phase, screen or network runtime."}
    (output / "fixture-sources.json").write_text(json.dumps(evidence, indent=2) + "\n")
    verify(output)
    print(output)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--horizon-source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify-lock", action="store_true")
    parser.add_argument("--include-payload", action="store_true")
    parser.add_argument("--include-correlation", action="store_true")
    args = parser.parse_args()
    if args.verify_lock:
        verify(args.output.resolve())
        print("Actual official Text, helper/API source and lock/path identities verified")
    elif args.horizon_source is None:
        parser.error("--horizon-source is required")
    else:
        prepare(args.horizon_source, args.output, args.include_payload, args.include_correlation)
