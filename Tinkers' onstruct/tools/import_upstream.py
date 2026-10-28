"""Import an explicit MIT subset from an immutable TConstruct Git revision.

No network access. --check verifies committed bytes without a source checkout.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
REV = "f4e77d43f39ba1047bf93ae816b7e1beaab514c2"
PREFIX = "src/generated/resources/data/tconstruct/"
JAVA = "src/main/java/slimeknights/tconstruct/"
FILES = ["LICENSE", "gradle.properties"]
FILES += [PREFIX + f"tinkering/materials/{kind}/{material}.json"
          for material in ("wood", "flint") for kind in ("definition", "stats", "traits")]
FILES += [PREFIX + "tinkering/tool_definitions/pickaxe.json"]
FILES += [PREFIX + "tags/items/modifiable/melee/weapon.json",
          PREFIX + "tags/items/modifiable/melee/primary.json"]
FILES += [PREFIX + f"tinkering/modifiers/{name}.json" for name in ("cultivated", "jagged", "pierce")]
FILES += [PREFIX + f"recipes/tools/parts/builder/{name}.json"
          for name in ("pick_head", "tool_handle", "tool_binding")]
FILES += [PREFIX + "recipes/tools/building/pickaxe.json",
          PREFIX + "recipes/tools/materials/wood/planks/default.json",
          PREFIX + "recipes/tools/materials/flint/flint.json"]
FILES += [JAVA + path for path in (
    "library/tools/stat/FloatToolStat.java", "library/tools/stat/ToolStats.java",
    "library/tools/nbt/ToolStack.java", "library/tools/nbt/ModifierNBT.java",
    "library/tools/definition/module/material/MaterialStatsModule.java",
    "library/tools/definition/module/build/SetStatsModule.java",
    "tools/stats/HeadMaterialStats.java", "tools/stats/HandleMaterialStats.java",
    "library/modifiers/modules/behavior/RepairModule.java",
    "library/tools/definition/module/mining/MiningSpeedToolHook.java",
    "library/tools/definition/module/mining/IsEffectiveToolHook.java",
    "library/tools/helper/ToolHarvestLogic.java",
    "library/tools/helper/ToolAttackUtil.java",
    "library/recipe/material/IMaterialValue.java",
    "library/recipe/material/MaterialRecipe.java",
    "tables/recipe/TinkerStationRepairRecipe.java",
    "library/tools/helper/ToolDamageUtil.java",
)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    lock_path = ROOT / "upstream-lock.json"
    destination = ROOT / "third_party/tconstruct"
    if args.check:
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
        if lock["revision"] != REV or {f["path"] for f in lock["files"]} != set(FILES):
            raise ValueError("Lock file revision or file set differs from the import specification")
        for entry in lock["files"]:
            content = (destination / entry["path"]).read_bytes()
            if hashlib.sha256(content).hexdigest() != entry["sha256"]:
                raise ValueError("Upstream snapshot changed: " + entry["path"])
        if (ROOT / "LICENSE").read_bytes() != (destination / "LICENSE").read_bytes():
            raise ValueError("Root LICENSE differs from the preserved upstream MIT license")
        print(f"Verified {len(FILES)} upstream files at {REV}")
        return
    if args.source is None:
        parser.error("--source is required unless --check is used")
    # git show reads the pinned object, never uncommitted working tree files.
    contents = {path: subprocess.run(["git", "-C", str(args.source), "show", f"{REV}:{path}"],
                                     capture_output=True, check=True).stdout for path in FILES}
    for path, content in contents.items():
        target = destination / path
        if target.exists() and target.read_bytes() != content:
            raise ValueError(f"Refusing to overwrite changed snapshot: {target}")
    for path, content in contents.items():
        target = destination / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)
    (ROOT / "LICENSE").write_bytes(contents["LICENSE"])
    lock = {"schema": 1, "repository": "https://github.com/SlimeKnights/TinkersConstruct",
            "revision": REV, "branch": "1.20.1", "license": "MIT",
            "files": [{"path": path, "sha256": hashlib.sha256(contents[path]).hexdigest()}
                      for path in sorted(contents)]}
    lock_path.write_text(json.dumps(lock, indent=2) + "\n", encoding="utf-8")
    print(f"Imported {len(FILES)} files from pinned Git objects")


if __name__ == "__main__":
    main()
