#!/usr/bin/env python3
"""Run production Drain interaction with real Horizon state types and mocked world.

HORIZON_HOST must name the pinned host checkout. Cargo dependencies are real;
world storage, structure detection and inventory access are fixtures.
"""
import os, pathlib, subprocess, tempfile, tomllib
root = pathlib.Path(__file__).resolve().parents[1]
host = pathlib.Path(os.environ["HORIZON_HOST"]).resolve()
expected = "0c7b5b61a5e746d621afffba919b12aeccf9c0fa"
actual = subprocess.check_output(["git", "-C", str(host), "rev-parse", "HEAD"], text=True).strip()
dirty = subprocess.check_output(["git", "-C", str(host), "status", "--porcelain", "--untracked-files=no"], text=True)
if actual != expected or dirty:
    raise SystemExit(f"HORIZON_HOST must have a clean tracked tree at {expected}")
source = (root / "mixins/tconstruct/source-machine-runtime.rs").read_text()
route = source[source.index("pub(super) fn interact_drain("):source.index("pub(super) fn ensure(")]
template = (root / "tests/rust/drain_container_fixture.rs").read_text()
template = template.replace("__MIXINS__", str(root / "mixins/tconstruct")).replace("__PLUTO__",str(root.parent / "pluto-lib/mixins/pluto_native_creative/stack-fluid.rs")).replace("// __PRODUCTION_ROUTE__", route)
with tempfile.TemporaryDirectory(prefix="drain-container-") as tmp:
    path = pathlib.Path(tmp)
    (path / "src").mkdir()
    (path / "src/lib.rs").write_text(template)
    (path / "Cargo.toml").write_text('[package]\nname="drain-container-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n' % (host / "crates/horizon-items", host / "crates/horizon-world", host / "crates/horizon-client-core"))
    # Resolve from the host lock and reject any changed transitive identity.
    host_lock = (host / "Cargo.lock").read_text()
    (path / "Cargo.lock").write_text(host_lock + '\n[[package]]\nname="drain-container-fixture"\nversion="0.1.0"\ndependencies=["horizon-items", "horizon-world", "horizon-client-core"]\n')
    subprocess.run(["cargo", "metadata", "--offline", "--manifest-path", str(path / "Cargo.toml"), "--format-version", "1"], check=True, stdout=subprocess.DEVNULL)
    def identity(package):
        return tuple(package.get(key) for key in ("name", "version", "source", "checksum"))
    allowed = {identity(package) for package in tomllib.loads(host_lock)["package"]}
    unknown = [identity(package) for package in tomllib.loads((path / "Cargo.lock").read_text())["package"]
               if package["name"] != "drain-container-fixture" and identity(package) not in allowed]
    if unknown:
        raise SystemExit(f"Fixture dependencies differ from pinned host lock: {unknown}")
    result = subprocess.run(["cargo", "test", "--locked", "--offline", "--manifest-path", str(path / "Cargo.toml"), "--", "--nocapture"], check=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(result.stdout)
    for case in ("full_tank_and_empty_tank_consume_container_action_without_mutation", "noncontainers_and_unformed_drains_still_pass", "successful_and_rejected_save_paths_preserve_existing_item_transaction"):
        if f"test {case} ... ok" not in result.stdout.splitlines():
            raise SystemExit(f"Required route test did not execute: {case}")
