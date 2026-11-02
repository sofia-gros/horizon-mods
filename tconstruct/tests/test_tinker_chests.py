"""Pinned source recipes and resource references for the chest feature."""
import json
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "content"

class TinkerChestSourceTest(unittest.TestCase):
    def test_crafting_station_uses_the_source_pattern_and_workbench_tag(self):
        self.assertFalse((ROOT / "data/tconstruct/recipe/crafting_station.json").exists())
        recipe = json.loads((ROOT.parent / "disabled-recipes/crafting_station.json").read_text())
        self.assertEqual(recipe["pattern"], ["p", "w"])
        self.assertEqual(recipe["key"], {"p": "tconstruct:pattern", "w": "#tconstruct:source_forge_workbenches"})
        tag = json.loads((ROOT / "reference/tconstruct-tags/data/forge/tags/items/workbenches.json").read_text())
        self.assertIn("minecraft:crafting_table", tag["values"])

    def test_source_recipes_and_item_declarations(self):
        for name, ingredient in [("part_chest", "#tconstruct:source_forge_rods_wooden"), ("tinkers_chest", "#tconstruct:source_forge_gems_lapis")]:
            recipe = json.loads((ROOT / f"data/tconstruct/recipe/{name}.json").read_text())
            self.assertEqual(recipe["pattern"], [" p ", "xCx", "xwx"])
            self.assertEqual(recipe["key"], {"p": "tconstruct:pattern", "x": ingredient, "C": "#tconstruct:source_forge_chests_wooden", "w": "#minecraft:planks"})
            self.assertEqual(recipe["result"]["id"], f"tconstruct:{name}")
            self.assertTrue((ROOT / f"data/tconstruct/horizon/block/{name}.json").is_file())
            self.assertTrue((ROOT / f"assets/tconstruct/items/{name}.json").is_file())
            self.assertTrue((ROOT / f"data/tconstruct/loot_table/blocks/{name}.json").is_file())
        with zipfile.ZipFile(ROOT / "reference/forge-47.4.0-data.zip") as archive:
            for tag in ["chests/wooden", "rods/wooden", "gems/lapis"]:
                source = json.loads(archive.read(f"data/forge/tags/items/{tag}.json"))
                alias = json.loads((ROOT / f"data/tconstruct/tags/item/source_forge_{tag.replace('/', '_')}.json").read_text())
                self.assertTrue(set(source["values"]).issubset(alias["values"]))
                self.assertIn(f"#forge:{tag}", alias["values"])

    def test_original_facing_models_and_textures(self):
        assets = ROOT / "assets/tconstruct"
        for name, kind in [("part_chest", "part"), ("tinkers_chest", "tinkers")]:
            state = json.loads((assets / f"blockstates/{name}.json").read_text())
            self.assertEqual(set(state["variants"]), {f"facing={face}" for face in ["north", "east", "south", "west"]})
            for texture in ["front", "side", "top", "drawer_front", "drawer_side"]:
                self.assertTrue((assets / f"textures/block/chest/{kind}/{texture}.png").is_file())

    def test_tinkers_chest_uses_the_source_default_dye(self):
        item = json.loads((ROOT / "assets/tconstruct/items/tinkers_chest.json").read_text())
        self.assertEqual(item["model"]["tints"], [{"type": "minecraft:dye", "default": 0xff407686-(1 << 32)}])
        self.assertFalse((ROOT / "data/tconstruct/recipe/tinkers_chest_dyed.json").exists())
        recipe = json.loads((ROOT.parent / "disabled-recipes/tinkers_chest_dyed.json").read_text())
        self.assertEqual((recipe["type"],recipe["target"],recipe["dye"]), ("minecraft:crafting_dye","tconstruct:tinkers_chest","#minecraft:dyes"))

    def test_glazed_template_chests_have_local_and_remote_use_hooks(self):
        mixins = ROOT.parent / "mixins/tconstruct"
        local = (mixins / "source-machine-use.rs").read_text()
        remote = (mixins / "source-machine-reacts.rs").read_text()
        for name in ("tconstruct:part_chest", "tconstruct:tinkers_chest"):
            self.assertIn(name, local)
            self.assertIn(name, remote)

if __name__ == "__main__":
    unittest.main()
