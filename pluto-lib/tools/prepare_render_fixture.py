#!/usr/bin/env python3
"""Prepare, but do not compile, a real-official-types rendering equivalence fixture.

The supplied Horizon tree must be the approved clean official git checkout. This
script neither fetches repositories nor modifies that tree. Run Cargo only in a
coordinated native build slot with the selected target's environment and cache.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib


OFFICIAL_ORIGIN = "https://github.com/horizon-rust/horizon.git"
FIXTURE_IDENTITY = ("pluto-render-equivalence", "0.0.0", None, None)


def identity(package):
    return tuple(package.get(key) for key in ("name", "version", "source", "checksum"))


def check_identities(host_lock: dict, fixture_lock: dict, fixture_identity=FIXTURE_IDENTITY) -> None:
    allowed = {identity(p) for p in host_lock["package"]} | {fixture_identity}
    resolved = {identity(p) for p in fixture_lock["package"]}
    unknown = resolved - allowed
    if unknown:
        raise ValueError(f"Fixture dependencies differ from official host lock: {unknown}")
    if fixture_identity not in resolved:
        raise ValueError("Resolved lock must contain the exact local fixture identity")


def clean_official(horizon: Path, expected: str) -> str:
    def git(*args):
        return subprocess.check_output(["git", "-C", str(horizon), *args], text=True).strip()
    origin = git("remote", "get-url", "origin")
    head = git("rev-parse", "HEAD")
    dirty = git("status", "--porcelain", "--untracked-files=all")
    if origin != OFFICIAL_ORIGIN or head != expected or dirty:
        raise ValueError(f"Require clean official Horizon checkout at {expected}")
    return head


def verify(output: Path, fixture_identity=FIXTURE_IDENTITY) -> dict:
    evidence = json.loads((output / "fixture-sources.json").read_text())
    host = Path(evidence["horizon_source"])
    clean_official(host, evidence["horizon_revision"])
    host_lock = host / "Cargo.lock"
    if hashlib.sha256(host_lock.read_bytes()).hexdigest() != evidence["official_lock_sha256"]:
        raise ValueError("Official lock changed after fixture preparation")
    for name, expected in evidence["sources_sha256"].items():
        if hashlib.sha256((output / name).read_bytes()).hexdigest() != expected:
            raise ValueError(f"Fixture input changed after preparation: {name}")
    check_identities(tomllib.loads(host_lock.read_text()),
                     tomllib.loads((output / "Cargo.lock").read_text()), fixture_identity)
    return evidence


def prepare(horizon: Path, output: Path) -> None:
    root = Path(__file__).resolve().parents[1]
    horizon = horizon.resolve()
    pin = tomllib.loads((root / "mixins/pluto_native_creative/horizon-mixin.toml").read_text())["compatibility"]["horizon_revision"]
    head = clean_official(horizon, pin)
    official_lock = (horizon / "Cargo.lock").read_bytes()
    locked = tomllib.loads(official_lock.decode())
    glam = [p["version"] for p in locked["package"] if p["name"] == "glam" and p["version"].startswith("0.30.")]
    if len(glam) != 1:
        raise ValueError("Require one official locked glam 0.30 identity")
    render = horizon / "crates/horizon-client-render"
    if not (render / "Cargo.toml").is_file():
        raise ValueError("Horizon source lacks the official client-render crate")
    if output.exists():
        raise ValueError("Fixture output must not already exist")
    output.mkdir(parents=True)
    sources = {
        "rendering.rs": root / "mixins/pluto_native_creative/rendering.rs",
        "oracle.rs": root / "tests/native/rendering/oracle.rs",
        "crop-oracle.rs": root / "tests/native/rendering/crop-oracle.rs",
        "panel-oracle.rs": root / "tests/native/rendering/panel-oracle.rs",
        "station-oracle.rs": root / "tests/native/rendering/station-oracle.rs",
        "side-oracle.rs": root / "tests/native/rendering/side-oracle.rs",
        "tests.rs": root / "tests/native/rendering/tests.rs",
    }
    for name, source in sources.items():
        shutil.copyfile(source, output / name)
    (output / "Cargo.toml").write_text(
        '[package]\nname = "pluto-render-equivalence"\nversion = "0.0.0"\nedition = "2024"\n'
        '[workspace]\n[lib]\npath = "lib.rs"\n'
        '[dependencies]\nhorizon-client = { package = "horizon-client-render", path = '
        + json.dumps(str(render)) + ', default-features = false }\n'
        + 'glam = "=' + glam[0] + '"\n'
    )
    # Root workspace patches do not propagate through path dependencies.
    # Mirror only the approved official checkout's local patch paths.
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
        '#![forbid(unsafe_code)]\n'
        'mod rendering {\n    include!("rendering.rs");\n'
        '    #[cfg(test)] mod tests {\n'
        '        mod oracle { include!("oracle.rs"); }\n'
        '        mod crop_oracle { include!("crop-oracle.rs"); }\n'
        '        mod panel_oracle { include!("panel-oracle.rs"); }\n'
        '        mod station_oracle { include!("station-oracle.rs"); }\n'
        '        mod side_oracle { include!("side-oracle.rs"); }\n'
        '        include!("tests.rs");\n    }\n}\n'
    )
    evidence = {
        "horizon_source": str(horizon),
        "horizon_revision": head,
        "official_lock_sha256": hashlib.sha256(official_lock).hexdigest(),
        "sources_sha256": {name: hashlib.sha256(source.read_bytes()).hexdigest()
                           for name, source in sources.items()},
        "validation": "Official identities resolved offline; no native compilation/test or package apply was run.",
    }
    for name in ("Cargo.toml", "lib.rs"):
        evidence["sources_sha256"][name] = hashlib.sha256((output / name).read_bytes()).hexdigest()
    (output / "Cargo.lock").write_bytes(official_lock)
    subprocess.run(["cargo", "metadata", "--offline", "--manifest-path", str((output / "Cargo.toml").resolve()),
                    "--format-version", "1"], check=True, stdout=subprocess.DEVNULL)
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
        print("Fixture sources and exact official dependency identities verified")
    elif args.horizon_source is None:
        parser.error("--horizon-source is required when preparing a fixture")
    else:
        prepare(args.horizon_source, args.output)
