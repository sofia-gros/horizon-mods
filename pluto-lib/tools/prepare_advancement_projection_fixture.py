#!/usr/bin/env python3
"""Prepare a bounded real-official parser/index fixture without host edits.

The official advancements.rs bytes are an exact prefix of the dependency's
source; only projection/test-support child declarations are appended. Native
core potion and NBT functions are real reexports, not substitutes. The outer
Game ownership shell is explicitly modeled; this is not a game/runtime test.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
OFFICIAL_ORIGIN = "https://github.com/horizon-rust/horizon.git"
LOCAL = {("pluto-advancement-projection-tests", "0.0.0", None, None),
         ("pluto-projection-native-fixture", "0.0.0", None, None),
         ("pluto-inventory-api", "0.1.0", None, None)}
HOST_SOURCE = "crates/horizon-client/src/advancements.rs"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def identity(package):
    return tuple(package.get(k) for k in ("name", "version", "source", "checksum"))


def metadata(output, locked):
    command = ["cargo", "metadata", "--offline", "--manifest-path", str((output / "Cargo.toml").resolve()), "--format-version", "1"]
    if locked:
        command.append("--locked")
    return json.loads(subprocess.check_output(command, text=True))


def check_paths(resolved, output, host):
    local = {"pluto-advancement-projection-tests": output / "Cargo.toml",
             "pluto-projection-native-fixture": output / "native/Cargo.toml",
             "pluto-inventory-api": output / "inventory-api/Cargo.toml"}
    found = set()
    for package in resolved["packages"]:
        path = Path(package["manifest_path"]).resolve()
        if package["name"] in local:
            if package["source"] is not None or path != local[package["name"]].resolve():
                raise ValueError("Local fixture/API package did not resolve to its exact source")
            found.add(package["name"])
        elif package["source"] is None and not path.is_relative_to(host):
            raise ValueError("A path dependency is outside the approved official checkout")
    if found != set(local):
        raise ValueError("A required exact local package is missing")


def clean_official(host, pin):
    def git(*args):
        return subprocess.check_output(["git", "-C", str(host), *args], text=True).strip()
    if (git("remote", "get-url", "origin") != OFFICIAL_ORIGIN or git("rev-parse", "HEAD") != pin
            or git("status", "--porcelain", "--untracked-files=all")):
        raise ValueError("Require the clean selected official Horizon checkout")


def verify(output):
    evidence = json.loads((output / "fixture-sources.json").read_text())
    host = Path(evidence["official_source"])
    clean_official(host, evidence["official_revision"])
    original = (host / HOST_SOURCE).read_bytes()
    if digest(original) != evidence["official_advancements_sha256"]:
        raise ValueError("Official advancements source drifted")
    if not (output / "native/advancements.rs").read_bytes().startswith(original):
        raise ValueError("The complete official source is not the fixture's exact prefix")
    host_lock = (host / "Cargo.lock").read_bytes()
    if digest(host_lock) != evidence["official_lock_sha256"]:
        raise ValueError("Official lock drifted")
    for path, sha in evidence["files_sha256"].items():
        if digest((output / path).read_bytes()) != sha:
            raise ValueError(f"Fixture input drifted: {path}")
    official = {identity(p) for p in tomllib.loads(host_lock.decode())["package"]}
    resolved = {identity(p) for p in tomllib.loads((output / "Cargo.lock").read_text())["package"]}
    if resolved - official - LOCAL or not LOCAL.issubset(resolved):
        raise ValueError("Fixture dependency identities differ from official lock/local packages")
    check_paths(metadata(output, locked=True), output, host)
    return evidence


def prepare(host, output):
    host = host.resolve()
    pin = tomllib.loads((ROOT / "mixins/pluto_native_creative/horizon-mixin.toml").read_text())["compatibility"]["horizon_revision"]
    clean_official(host, pin)
    if output.exists():
        raise ValueError("Output must be new")
    output.mkdir(parents=True)
    native = output / "native"
    native.mkdir()
    api = ROOT / "mixins/pluto_native_creative/inventory-api"
    shutil.copytree(api, output / "inventory-api", ignore=shutil.ignore_patterns("Cargo.lock", "target"))
    helpers = ROOT / "mixins/pluto_native_creative"
    shutil.copyfile(helpers / "advancement-projection.rs", native / "projection.rs")
    shutil.copyfile(helpers / "advancement-projection-world.rs", output / "game-adapter.rs")
    shutil.copyfile(ROOT / "tests/native/advancement_projection.rs", output / "lib.rs")
    original = (host / HOST_SOURCE).read_bytes()
    suffix = b'''\n#[path="projection.rs"] pub mod pluto_advancement_projection;
pub mod fixture_support {
    pub fn tree(docs: &[(&str, serde_json::Value)]) -> super::Tree {
        let mut tree = super::Tree::default();
        for (id, value) in docs {
            let goal = super::parse(id, value).expect("fixture source definition");
            let at = tree.list.len();
            for (ci, criterion) in goal.criteria.iter().enumerate() {
                tree.by_trigger.entry(criterion.trigger.clone()).or_default().push((at, ci));
            }
            tree.by_id.insert((*id).to_owned(), at);
            tree.list.push(goal);
        }
        tree.index_recipes(); tree.index_looking_at(); tree.index_nbt(); tree.index_player_facts();
        tree
    }
}
'''
    (native / "advancements.rs").write_bytes(original + suffix)
    (native / "lib.rs").write_text('pub use horizon_client_core::{potions as brewing, nbt_path};\npub mod advancements;\n')
    (native / "Cargo.toml").write_text('[package]\nname="pluto-projection-native-fixture"\nversion="0.0.0"\nedition="2024"\n[lib]\npath="lib.rs"\n[dependencies]\n'
        'serde_json="1"\nlog="0.4"\npluto-inventory-api={path="../inventory-api"}\n'
        'horizon-client-core={path=' + json.dumps(str(host / "crates/horizon-client-core")) + '}\n'
        'horizon-world={path=' + json.dumps(str(host / "crates/horizon-world")) + '}\n')
    (output / "Cargo.toml").write_text('[package]\nname="pluto-advancement-projection-tests"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\nserde_json="1"\n'
        'pluto-inventory-api={path="inventory-api"}\npluto-projection-native-fixture={path="native"}\n')
    patches = tomllib.loads((host / "Cargo.toml").read_text()).get("patch", {}).get("crates-io", {})
    with (output / "Cargo.toml").open("a") as manifest:
        manifest.write("\n[patch.crates-io]\n")
        for name, entry in patches.items():
            if set(entry) != {"path"}:
                raise ValueError("Only approved official local patches are supported")
            path = (host / entry["path"]).resolve()
            if not path.is_relative_to(host):
                raise ValueError("Official patch escapes checkout")
            manifest.write(json.dumps(name) + "={path=" + json.dumps(str(path)) + "}\n")
    host_lock = (host / "Cargo.lock").read_bytes()
    evidence = {"official_source": str(host), "official_revision": pin,
                "official_advancements_sha256": digest(original), "official_lock_sha256": digest(host_lock),
                "files_sha256": {str(p.relative_to(output)): digest(p.read_bytes()) for p in output.rglob("*") if p.is_file()},
                "scope": "actual native parser/tree/index and production adapters; modeled outer Game ownership only; no package apply/game runtime"}
    (output / "Cargo.lock").write_bytes(host_lock)
    check_paths(metadata(output, locked=False), output, host)
    (output / "fixture-sources.json").write_text(json.dumps(evidence, indent=2) + "\n")
    verify(output)
    print(output.resolve())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--horizon-source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify-lock", action="store_true")
    args = parser.parse_args()
    if args.verify_lock:
        verify(args.output)
        print("Exact official source prefix, API/helper hashes and dependency identities verified")
    elif args.horizon_source is None:
        parser.error("--horizon-source is required")
    else:
        prepare(args.horizon_source, args.output)
