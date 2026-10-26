"""Inventory the selected upstream data and explicitly deferred game hooks."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "third_party/tconstruct/src/generated/resources/data/tconstruct"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    read = lambda path: json.loads((DATA / path).read_text(encoding="utf-8"))
    selected = ("tconstruct:head", "tconstruct:handle", "tconstruct:binding")
    dependencies = {"tconstruct:pierce"}
    materials = {}
    for name in ("wood", "flint"):
        traits = read(f"tinkering/materials/traits/{name}.json")
        materials[name] = {}
        for stat in selected:
            entries = traits.get("perStat", {}).get(stat, traits["default"])
            materials[name][stat] = entries
            dependencies.update(entry["name"] for entry in entries)
    modifiers = {}
    for modifier in sorted(dependencies):
        name = modifier.split(":")[1]
        modules = read(f"tinkering/modifiers/{name}.json")["modules"]
        modifiers[modifier] = [{"type": module["type"],
                                "core": "declarative effect" if module["type"] == "tconstruct:weapon_mob_effect" else "evaluated",
                                "host_hook": "not implemented"} for module in modules]
    report = {"schema": 1, "revision": "f4e77d43f39ba1047bf93ae816b7e1beaab514c2",
              "gameplay_ready": False, "scope": "explicit wood/flint pickaxe states; no upgrades",
              "materials": materials, "modifiers": modifiers,
              "tool_modules": [{"type": module["type"],
                                "status": "evaluated" if module["type"] in (
                                    "tconstruct:part_stats", "tconstruct:base_stats", "tconstruct:traits")
                                else "pinned and validated; host/random/UI execution deferred"}
                               for module in read("tinkering/tool_definitions/pickaxe.json")["modules"]],
              "deferred": ["ammo material stats and traits", "upgrade recipe application", "random default material selection",
                           "tool action and AOE execution", "effect registration/application", "item host hooks",
                           "part builder material conditions and transactions", "station menus", "rendering", "smeltery"],
              "recipes": [{"path": str(path.relative_to(DATA)).replace("\\", "/"),
                           "type": json.loads(path.read_text())["type"], "status": "snapshot only; not executable"}
                          for path in sorted((DATA / "recipes").rglob("*.json"))]}
    target = ROOT / "port-support.json"
    text = json.dumps(report, indent=2) + "\n"
    if args.check:
        if target.read_text(encoding="utf-8") != text:
            raise ValueError("Dependency inventory drifted")
    else:
        target.write_text(text, encoding="utf-8")
    print("Verified dependency inventory" if args.check else "Wrote dependency inventory")


if __name__ == "__main__":
    main()
