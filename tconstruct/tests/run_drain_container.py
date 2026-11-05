#!/usr/bin/env python3
"""Run production Drain interaction with real Horizon state types and mocked world.

HORIZON_HOST must name the pinned host checkout. Cargo dependencies are real;
world storage, structure detection and inventory access are fixtures.
"""
import os, pathlib, subprocess, tempfile
root = pathlib.Path(__file__).resolve().parents[1]
host = pathlib.Path(os.environ["HORIZON_HOST"]).resolve()
source = (root / "mixins/tconstruct/source-machine-runtime.rs").read_text()
route = source[source.index("pub(super) fn interact_drain("):source.index("pub(super) fn ensure(")]
template = (root / "tests/rust/drain_container_fixture.rs").read_text()
template = template.replace("__MIXINS__", str(root / "mixins/tconstruct")).replace("__PLUTO__",str(root.parent / "pluto-lib/mixins/pluto_native_creative/stack-fluid.rs")).replace("// __PRODUCTION_ROUTE__", route)
with tempfile.TemporaryDirectory(prefix="drain-container-") as tmp:
    path = pathlib.Path(tmp)
    (path / "src").mkdir()
    (path / "src/lib.rs").write_text(template)
    (path / "Cargo.toml").write_text('[package]\nname="drain-container-fixture"\nversion="0.1.0"\nedition="2024"\n[dependencies]\nhorizon-items={path="%s"}\nhorizon-world={path="%s"}\nhorizon-client-core={path="%s"}\n' % (host / "crates/horizon-items", host / "crates/horizon-world", host / "crates/horizon-client-core"))
    subprocess.run(["cargo", "generate-lockfile", "--offline", "--manifest-path", str(path / "Cargo.toml")], check=True)
    subprocess.run(["cargo", "test", "--locked", "--offline", "--manifest-path", str(path / "Cargo.toml")], check=True)
