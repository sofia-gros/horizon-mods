import json
import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
from import_part_visuals import MATERIAL_MODELS

SCRIPT = Path(__file__).resolve().parents[1] / 'tools/resources.py'

class ResourceOverlay(unittest.TestCase):
    def test_overlay_keeps_input_and_registers_items(self):
        with tempfile.TemporaryDirectory() as tmp:
            src, out = Path(tmp) / 'src', Path(tmp) / 'out'
            src.mkdir()
            raw = {'minecraft:flint': {}, 'minecraft:stick': {}, 'minecraft:paper': {}, 'minecraft:honey_bottle': {},
                   'minecraft:stone_pickaxe': {'minecraft:tool': {'rules': [{}, {'speed': 4}]}},
                   'minecraft:stone_axe': {}, 'minecraft:stone_shovel': {}, 'minecraft:stone_hoe': {}}
            raw['minecraft:stick']['minecraft:cooking_fuel'] = {'burn_time': 100}
            creative = {'tabs': [{'id': 'minecraft:hotbar', 'type': 'HOTBAR', 'row': 'TOP', 'column': 5}]}
            (src / 'creative-inventory.json').write_text(json.dumps(creative))
            (src / 'items.json').write_text(json.dumps(raw))
            (src / 'command-registries.json').write_text(json.dumps({'items': list(raw), 'by_registry': {'minecraft:item': {'ids': list(raw), 'tags': []}}}))
            (src / 'registries.json').write_text(json.dumps({'minecraft:item': {'entries': {'minecraft:air': {'protocol_id': 0}}}}))
            (src / 'assets/minecraft/textures/gui').mkdir(parents=True)
            subprocess.run([sys.executable, str(SCRIPT), str(src), str(out)], check=True)
            self.assertEqual(json.loads((src / 'items.json').read_text()), raw)
            self.assertEqual(json.loads((src / 'creative-inventory.json').read_text()), creative)
            tabs = json.loads((out / 'creative-inventory.json').read_text())['tabs']
            self.assertEqual(tabs[-1]['id'], 'tconstruct:tools')
            self.assertEqual(tabs[-1]['items'][0], '{count:1,id:"tconstruct:flint_and_brick"}')
            items = json.loads((out / 'items.json').read_text())
            ids = json.loads((out / 'registries.json').read_text())['minecraft:item']['entries']
            names = ['tconstruct:flint_pick_head', 'tconstruct:wood_binding', 'tconstruct:wood_handle', 'tconstruct:flint_pickaxe', 'tconstruct:pick_head', 'tconstruct:tool_handle', 'tconstruct:tool_binding', 'tconstruct:pickaxe']
            self.assertTrue(all(name in items and name in ids for name in names))
            self.assertTrue(all('tconstruct:' + name in items for name in ('mattock', 'pickadze', 'kama')))
            equipment = {'slimy_chestplate', 'crystalshot'}
            with zipfile.ZipFile(Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-tool-definitions.zip') as archive:
                equipment.update(('slimy_chestplate' if Path(name).stem == 'slime_chestplate' else Path(name).stem)
                                 for name in archive.namelist() if name.endswith('.json') and Path(name).stem != 'minotaur_axe')
            self.assertEqual(len(equipment), 45)
            self.assertTrue(all('tconstruct:' + name in items and 'tconstruct:' + name in ids for name in equipment))
            for name in equipment - {'pickaxe'}:
                model = json.loads((out / 'assets/tconstruct/models/item' / (name + '.json')).read_text())
                self.assertTrue(model.get('textures'), name)
                for texture in model['textures'].values():
                    self.assertTrue((out / 'assets/tconstruct/textures' / (texture.removeprefix('tconstruct:') + '.png')).is_file(), (name, texture))
            for suffix, slot in (('helmet', 'head'), ('chestplate', 'chest'), ('leggings', 'legs'), ('boots', 'feet')):
                for prefix in ('slime' if suffix != 'chestplate' else 'slimy', 'plate', 'travelers'):
                    self.assertEqual(items[f'tconstruct:{prefix}_{suffix}']['minecraft:equippable']['slot'], slot)
            self.assertEqual(items['tconstruct:slime_wings']['minecraft:equippable']['slot'], 'chest')
            for name, limit in (('dagger', 2), ('shuriken', 16), ('throwing_axe', 16), ('arrow', 64)):
                self.assertEqual(items['tconstruct:' + name]['minecraft:max_stack_size'], limit)
            bottle = items['tconstruct:venom_bottle']
            self.assertEqual(bottle['minecraft:max_stack_size'], 16)
            self.assertEqual(bottle['minecraft:food'], {'can_always_eat': True, 'nutrition': 0, 'saturation': 0.0})
            self.assertEqual(bottle['minecraft:consumable']['consume_seconds'], 1.6)
            self.assertEqual(bottle['minecraft:consumable']['on_consume_effects'][0]['effects'], [
                {'id': 'minecraft:strength', 'duration': 600}, {'id': 'minecraft:poison', 'duration': 250}])
            self.assertEqual(bottle['minecraft:use_remainder'], {'id': 'minecraft:glass_bottle'})
            source_texture = (Path(__file__).resolve().parents[1] / 'assets/tconstruct/textures/item/bottle/venom_contents.png').read_bytes()
            self.assertEqual(hashlib.sha256(source_texture).hexdigest(), '6d3066505a8422952f7bb974a4c71d98503d35e423497bdf3c1bc4ca0ece2243')
            self.assertEqual((out / 'assets/tconstruct/textures/item/bottle/venom_contents.png').read_bytes(), source_texture)
            for short in MATERIAL_MODELS:
                name = 'tconstruct:' + short
                self.assertIn(name, items)
                self.assertIn(name, ids)
                self.assertNotIn('minecraft:cooking_fuel', items[name])
                native = json.loads((out / 'assets/tconstruct/models/item' / (short + '.json')).read_text())
                original = json.loads((out / 'assets/tconstruct/tinkering/source_part_models' / (short + '.json')).read_text())
                self.assertEqual(native['textures']['layer0'], original['textures']['texture'])
            tag_file = Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-tags/data/tconstruct/tags/items/parts/bartered.json'
            for item in json.loads(tag_file.read_text())['values']:
                self.assertIn(item, ids)
            for name in ('tconstruct:tool_handle', 'tconstruct:tool_binding', 'tconstruct:wood_handle', 'tconstruct:wood_binding'):
                self.assertNotIn('minecraft:cooking_fuel', items[name])
            command_ids = json.loads((out / 'command-registries.json').read_text())['by_registry']['minecraft:item']['ids']
            self.assertTrue(all(name in command_ids for name in names))
            self.assertEqual(items[names[-1]]['minecraft:max_damage'], 85)
            self.assertEqual(items[names[-1]]['minecraft:tool']['rules'][-1]['speed'], 3.5)
            self.assertEqual(len({ids[name]['protocol_id'] for name in names}), len(names))
            self.assertTrue((out / 'assets/tconstruct/textures').is_dir())
            self.assertFalse((out / 'assets/tconstruct/textures').is_symlink())
            self.assertTrue((out / 'assets/tconstruct/textures/item/tool/pickaxe/head_tconstruct_flint.png').is_file())
            self.assertTrue((out / 'assets/tconstruct/tinkering/materials/flint.json').is_file())
            with zipfile.ZipFile(Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-part-visuals.zip') as archive:
                for code in ('en_us', 'ja_jp'):
                    name = f'assets/tconstruct/lang/{code}.json'
                    self.assertEqual((out / name).read_bytes(), archive.read(name))
            self.assertTrue((out / 'assets/tconstruct/tinkering/source_part_models/pick_head.json').is_file())
            tool_model = json.loads((out / 'assets/tconstruct/tinkering/source_tool_models/pickaxe.json').read_text())
            self.assertEqual([part['index'] for part in tool_model['parts']], [1, 0, 2])
            self.assertTrue((out / 'assets/tconstruct/textures/item/tool/pickaxe/head_tconstruct_flint.png').is_file())
            self.assertTrue((out / 'assets/tconstruct/items/tconstruct_tool/item/tool/pickaxe/head_tconstruct_flint.json').is_file())
            self.assertTrue((out / 'assets/tconstruct/items/tconstruct_part/item/tool/pickaxe/head_tconstruct_flint.json').is_file())
            native = json.loads((out / 'assets/tconstruct/models/item/tconstruct_part/item/tool/pickaxe/head_tconstruct_flint.json').read_text())
            source = json.loads((Path(__file__).resolve().parents[1] / 'content/reference/forge-default-item-model.json').read_text())
            self.assertEqual(native['parent'], 'minecraft:builtin/generated')
            self.assertEqual(native['display'], source['display'])
            self.assertEqual(native['gui_light'], source['gui_light'])
            self.assertEqual(native['textures']['layer0'], 'tconstruct:item/tool/pickaxe/head_tconstruct_flint')
            self.assertEqual(items[names[-1]]['minecraft:item_model'], names[-1])
            for alias in ('wood_pick_head', 'flint_handle', 'flint_binding'):
                self.assertTrue((out / 'assets/tconstruct/items' / (alias + '.json')).is_file())
                self.assertTrue((out / 'assets/tconstruct/models/item' / (alias + '.json')).is_file())
                self.assertTrue((out / 'assets/tconstruct/textures/item' / (alias + '.png')).is_file())
            icons = (Path(__file__).resolve().parents[1] / 'assets/tconstruct/gui/icons.png').read_bytes()
            self.assertEqual(hashlib.sha256(icons).hexdigest(), 'cb793955c7da6330162a32353d432e5236e0162ef12240ea1ca78858fcbc4b4a')
            self.assertEqual((out / 'assets/minecraft/textures/gui/tconstruct_icons.png').read_bytes(), icons)
            pattern_root = Path(__file__).resolve().parents[1] / 'assets/tconstruct/gui/patterns'
            manifest = Path(__file__).resolve().parents[1] / 'docs/upstream-patterns.sha256'
            rows = manifest.read_text().splitlines()
            self.assertEqual(len(rows), 62)
            for row in rows:
                digest, name = row.split('  ', 1)
                original = (pattern_root / name).read_bytes()
                self.assertEqual(hashlib.sha256(original).hexdigest(), digest)
                self.assertEqual((out / 'assets/minecraft/textures/gui/tconstruct_patterns' / name).read_bytes(), original)
            for name in ('tconstruct', 'mantle', 'forge'):
                self.assertTrue((out / 'THIRD_PARTY_LICENSES' / (name + '.txt')).is_file())
            rejected = subprocess.run([sys.executable, str(SCRIPT), str(src), str(out)], capture_output=True)
            self.assertNotEqual(rejected.returncode, 0)

if __name__ == '__main__':
    unittest.main()
