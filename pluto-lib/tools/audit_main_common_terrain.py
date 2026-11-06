#!/usr/bin/env python3
"""Read-only source/guard-policy audit; never runs Mixins, Cargo or Rust.

The constructed plan dictionaries below test only a Python guard's selection
policy. They are NOT planner output and cannot establish native compatibility.
"""
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tomllib
import zipfile

sys.dont_write_bytecode = True
MAIN = "08afa992d5091e03360f3473ea93fd17aa793987"
COMMON = "035273c4805d75111e528c8c26607fa3a8d1d5bf"
BUNDLE = "4f13a6e22e29be159b004e0a99801954acb15581d64851087b4427353de5e11d"
HORIZON = "624c4f6265c02296fbe8a80f029dd0d987fb94c0"
MIXINS = "e8cd0aa38cffef49cb2e088ef716baa107148c02"
MESHER = "crates/horizon-client-render/src/mesher.rs"
MOD = "tconstruct/mixins/tconstruct/"
GUARD_HASHES = {
    "check_terrain_cell_plan.py": "49a7ac329087f8d4e4cf0df7927dd16afad4f668678fb1603be188b747a97ed6",
    "terrain-cell-source.json": "af3cfc65f4d73467880ebd3af0e431af0a2e4e75a953fbabb6a25c247e12f04e",
    "check_terrain_binding_plan.py": "30e38d0cd0c22badd2457812bb78a1fccd67f05b6ca5c09ec35dba9282abbe27",
    "terrain-binding-source.json": "63dc9a768b8d369a47e6f18761a1771a1b394db7e1dddb4a42a8024fdc3f6c19",
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--repo", type=Path, required=True)
    p.add_argument("--common-source", type=Path, required=True)
    p.add_argument("--bundle", type=Path, required=True)
    p.add_argument("--official-source", type=Path, required=True)
    a = p.parse_args()
    inputs = {}
    checks = []

    def git(path):
        data = subprocess.check_output(["git", "-C", str(a.repo), "show", f"{MAIN}:{path}"])
        inputs[f"main:{path}"] = sha(data)
        return data

    manifest = tomllib.loads(git(MOD + "horizon-mixin.toml").decode())
    disabled = tomllib.loads(git("tconstruct/disabled-injections.toml").decode())
    compat = json.loads(git("tconstruct/compatibility.json"))
    modules = [m for m in manifest["module"] if m["file"] == MESHER]
    assert {m["id"] for m in modules} == {
        "storage-render-info-mesher", "connected-texture", "chest-tint-math", "source-slime-colormap"
    }
    assert not [h for h in manifest["injection"] if "::mesher::" in h["target"]]
    checks.append("four active support modules, zero active mesher injections")
    held = [h for h in disabled["injection"] if "::mesher::" in h["target"]]
    assert {h["id"] for h in held} == {
        "storage-mesh-face", "connected-mesh-face", "tinkers-chest-world-tint"
    }
    assert not [h for h in manifest["injection"] + disabled["injection"]
                if h.get("helper") == "source-slime-colormap-face.rs"]
    checks.append("three disabled face injections; slime tint helper not registered in either manifest")
    assert manifest["compatibility"]["horizon_revision"] == compat["horizon_revision"]
    assert compat["horizon_revision"] == "0c7b5b61a5e746d621afffba919b12aeccf9c0fa"
    assert compat["mixins_revision"] == "ddd9700b21997c95d95ac6ee5ecafe3830cf622a"
    checks.append("old main compatibility pins retained, not normalized")

    related = ["storage-atlas", "storage-render-init", "storage-region",
               "connected-pane-face", "source-slime-colormap-load"]
    hooks = [h for h in manifest["injection"] if h["id"] in related]
    assert len(hooks) == len(related)
    for m in modules:
        git(MOD + m["source"])
    for h in hooks + held:
        git(MOD + h["helper"])
    git(MOD + "source-slime-colormap-face.rs")
    package_source = git("tconstruct/tools/package_hnmod.py").decode()
    assert "tracked('tconstruct/mixins/tconstruct')" in package_source
    assert "disabled-injections.toml" not in package_source
    checks.append("packager does not import the external disabled manifest")

    raw = a.bundle.read_bytes()
    assert sha(raw) == BUNDLE
    with zipfile.ZipFile(a.bundle) as z:
        assert len(z.namelist()) == 147
        manifests = [n for n in z.namelist() if n.endswith("horizon-mixin.toml")]
        assert manifests == ["mixins/pluto_native_creative/horizon-mixin.toml"]
        cm = tomllib.loads(z.read(manifests[0]).decode())
        counts = {k: len(cm[k]) for k in ["injection", "module", "state", "crate"]}
        assert counts == {"injection": 34, "module": 69, "state": 3, "crate": 2}
        assert cm["compatibility"]["horizon_revision"] == HORIZON
        terrain = [m for m in cm["module"] if m["file"] == MESHER]
        assert len(terrain) == 5
        for m in terrain:
            archived = z.read("mixins/pluto_native_creative/" + m["source"])
            source = a.common_source / "pluto-lib/mixins/pluto_native_creative" / Path(m["source"]).name
            assert source.read_bytes() == archived
            inputs[f"common:{source.relative_to(a.common_source)}"] = sha(archived)
    checks.append("exact common archive, one manifest, 34/69/3/2 counts and five terrain source bytes")

    guard_path = a.common_source / "pluto-lib/tools/check_terrain_cell_plan.py"
    meta_path = guard_path.with_name("terrain-cell-source.json")
    for path in [guard_path, meta_path,
                 guard_path.with_name("check_terrain_binding_plan.py"),
                 guard_path.with_name("terrain-binding-source.json")]:
        actual = sha(path.read_bytes())
        assert actual == GUARD_HASHES[path.name]
        inputs[f"common:{path.relative_to(a.common_source)}"] = actual
    spec = importlib.util.spec_from_file_location("terrain_guard_audit", guard_path)
    guard = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard)
    meta = guard.metadata()
    for path, expected in meta["source_sha256"].items():
        actual = (a.official_source / path).read_bytes()
        assert sha(actual) == expected
        inputs[f"official:{path}"] = expected
    # Do not call check_source(): it executes rustfmt. No planner is invoked.
    checks.append("seven official guard source hashes match; no regeneration/compiler invoked")
    hook = {k: meta[k] for k in ["target", "function_fingerprint", "bindings", "priority", "kind", "cancellable"]}
    hook.update(meta["location"])
    hook.update(mod_id="pluto_native_creative", injection_id=meta["injection_id"],
                helper_source=(guard.NATIVE / "terrain-cell-head.rs").read_text())
    synthetic = {"schema": 6, "injections": [hook], "modules": [], "inputs": meta["source_sha256"]}
    for m in meta["modules"]:
        synthetic["modules"].append({"mod_id": "pluto_native_creative", "module_id": m["module_id"],
            "file": m["file"], "name": m["name"], "fingerprint": m["fingerprint"],
            "source": (guard.NATIVE / m["source"]).read_text()})
    guard.check(synthetic, meta)
    checks.append("synthetic guard control dictionary accepted (NOT an actual plan)")
    extra = copy.deepcopy(synthetic)
    extra["modules"].extend(dict(m, mod_id="tconstruct") for m in modules)
    guard.check(extra, meta)
    checks.append("four additional module declarations alone do not violate this guard policy")
    for h in held:
        extra = copy.deepcopy(synthetic)
        extra["injections"].append(dict(h, file=MESHER, mod_id="tconstruct"))
        try:
            guard.check(extra, meta)
        except ValueError as error:
            assert str(error) == "terrain cell adapter requires exclusive mesher-file hooks"
        else:
            raise AssertionError("extra mesher injection was accepted")
    checks.append("each of the three disabled face injection candidates is rejected by this guard policy")
    official_files = ["crates/horizon-client-render/src/assets.rs", "crates/horizon-client/src/game.rs",
                      "crates/horizon-world/src/block_render.rs"]
    for path in official_files:
        inputs[f"official:{path}"] = sha((a.official_source / path).read_bytes())
    report = {"schema": 1, "status": "source and synthetic Python guard-policy audit only",
        "main": MAIN, "common_source": COMMON, "common_bundle_sha256": BUNDLE,
        "official": {"horizon": HORIZON, "mixins": MIXINS},
        "main_pins": compat, "common_archive_counts": counts,
        "active_mesher_modules": modules, "disabled_face_injections": held,
        "related_active_injections": hooks, "checks_passed": checks, "input_sha256": inputs,
        "not_run": ["combined native planner", "Mixins apply", "Rust or Cargo", "game/GPU", "new timing"],
        "limits": ["Synthetic dictionaries establish only the exact Python guard behavior, not target resolution.",
                   "No production, manifest, compatibility pin, existing guard or consumer source was changed.",
                   "Other complete-plan guards and actual host compatibility may reject the composition earlier."]}
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
