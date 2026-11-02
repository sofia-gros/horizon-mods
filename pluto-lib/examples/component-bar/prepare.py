"""Populate the isolated Tool Rules demo tab with durability examples.

Pass the creative-inventory.json of a disposable resource overlay produced by
PlutoLib's Tool Rules demo. The file is updated in place.
"""
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
inventory = json.loads(path.read_text(encoding="utf-8"))
tab = next(t for t in inventory["tabs"] if t["id"] == "pluto_tool_rules_demo:tools")
tab["items"] = [
    '{id:"minecraft:diamond_pickaxe",count:1,components:{"minecraft:max_damage":60,"minecraft:damage":0}}',
    '{id:"minecraft:diamond_pickaxe",count:1,components:{"minecraft:max_damage":60,"minecraft:damage":1}}',
    '{id:"minecraft:diamond_pickaxe",count:1,components:{"minecraft:max_damage":60,"minecraft:damage":30}}',
    '{id:"minecraft:diamond_pickaxe",count:1,components:{"minecraft:max_damage":60,"minecraft:damage":59}}',
    '{id:"minecraft:diamond_pickaxe",count:1,components:{"minecraft:damage":30}}',
    '{id:"minecraft:diamond_pickaxe",count:1,components:{"minecraft:max_damage":60,"minecraft:damage":30,"minecraft:unbreakable":{}}}',
    '{id:"minecraft:diamond_pickaxe",count:1,components:{"minecraft:max_damage":0,"minecraft:damage":30}}',
    '{id:"minecraft:bundle",count:1,components:{"minecraft:max_damage":60,"minecraft:damage":30,"minecraft:bundle_contents":[{id:"minecraft:stone",count:32}]}}',
]
path.write_text(json.dumps(inventory, ensure_ascii=False), encoding="utf-8")
