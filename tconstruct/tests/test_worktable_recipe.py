import json
import unittest
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

class WorktableRecipe(unittest.TestCase):
    def test_worktable_variants_use_original_pattern_and_inputs(self):
        with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            source = json.loads(archive.read('data/tconstruct/recipes/tables/modifier_worktable.json'))
            extract = json.loads(archive.read('data/tconstruct/recipes/tools/modifiers/worktable/extract/upgrade.json'))
        source_tags = ROOT / 'content/reference/tconstruct-tags/data/forge/tags/items'
        rocks = {'minecraft:tuff', 'minecraft:dripstone_block', 'minecraft:calcite'}
        for tag in ('normal_stone', 'blackstone', 'granite', 'diorite', 'andesite', 'deepslate', 'basalt'):
            rocks.update(json.loads((source_tags / (tag + '.json')).read_text())['values'])
        for rock in rocks:
            short = rock.split(':')[1]
            local = json.loads((ROOT / 'content/data/tconstruct/recipe' / ('modifier_worktable_' + short + '.json')).read_text())
            self.assertEqual(local['pattern'], source['pattern'])
            self.assertEqual(local['key'], {'s': '#tconstruct:seared_blocks', 'r': rock})
            self.assertEqual(local['result']['id'], source['result']['item'])
            data = local['result']['components']['minecraft:custom_data']['tconstruct:anvil']
            self.assertEqual(data['Rock'], rock)
            self.assertEqual(data['Variant'], short)
            self.assertEqual(local['result']['components']['minecraft:item_model'], 'tconstruct:modifier_worktable_' + short)
            model = json.loads((ROOT / 'content/assets/tconstruct/models/block/table' / ('modifier_worktable_' + short + '.json')).read_text())
            self.assertEqual(model['textures']['leg'], data['Texture'])
            self.assertEqual(model['textures']['legBottom'], data['TextureTop'])
        self.assertEqual([entry['item'] for entry in extract['inputs']], ['tconstruct:sky_slime_crystal', 'minecraft:wet_sponge'])
        self.assertEqual(extract['leftovers'], ['minecraft:sponge'])
