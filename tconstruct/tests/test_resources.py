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
            raw = {'minecraft:flint': {}, 'minecraft:brick': {}, 'minecraft:bone': {}, 'minecraft:stick': {}, 'minecraft:paper': {}, 'minecraft:book': {}, 'minecraft:honey_bottle': {}, 'minecraft:glass_bottle': {'minecraft:max_stack_size': 64}, 'minecraft:mushroom_stew': {}, 'minecraft:copper_ingot': {'minecraft:max_stack_size': 64}, 'minecraft:water_bucket': {'minecraft:max_stack_size': 1},
                   'minecraft:stone_pickaxe': {'minecraft:tool': {'rules': [{}, {'speed': 4}]}},
                   'minecraft:stone_axe': {'minecraft:weapon': {'item_damage_per_attack': 2}, 'minecraft:tool': {'rules': []}}, 'minecraft:stone_shovel': {}, 'minecraft:stone_hoe': {'minecraft:tool': {'rules': []}}}
            raw['minecraft:stick']['minecraft:cooking_fuel'] = {'burn_time': 100}
            raw['minecraft:copper_ingot']['minecraft:provides_trim_material'] = 'minecraft:copper'
            creative = {'tabs': [{'id': 'minecraft:hotbar', 'type': 'HOTBAR', 'row': 'TOP', 'column': 5}]}
            (src / 'creative-inventory.json').write_text(json.dumps(creative))
            (src / 'items.json').write_text(json.dumps(raw))
            (src / 'horizon-mod-craft-remainders.json').write_text(json.dumps({'example:item': 'minecraft:bowl'}))
            (src / 'command-registries.json').write_text(json.dumps({'items': list(raw), 'by_registry': {'minecraft:item': {'ids': list(raw), 'tags': []}}}))
            (src / 'registries.json').write_text(json.dumps({'minecraft:item': {'entries': {'minecraft:air': {'protocol_id': 0}}}}))
            (src / 'assets/minecraft/textures/gui').mkdir(parents=True)
            (src / 'assets/minecraft/textures/item').mkdir(parents=True)
            for gem in ('diamond', 'emerald', 'netherite_ingot', 'slime_ball'):
                (src / 'assets/minecraft/textures/item' / (gem + '.png')).write_bytes(gem.encode())
            subprocess.run([sys.executable, str(SCRIPT), str(src), str(out)], check=True)
            self.assertEqual(json.loads((src / 'items.json').read_text()), raw)
            self.assertEqual(json.loads((src / 'creative-inventory.json').read_text()), creative)
            tabs = json.loads((out / 'creative-inventory.json').read_text())['tabs']
            tools = next(tab for tab in tabs if tab['id'] == 'tconstruct:tools')
            blocks = next(tab for tab in tabs if tab['id'] == 'tconstruct:blocks')
            world = next(tab for tab in tabs if tab['id'] == 'tconstruct:world')
            general = next(tab for tab in tabs if tab['id'] == 'tconstruct:general')
            self.assertEqual((general['page'], general['row'], general['column']), (1, 'TOP', 1))
            self.assertEqual(general['items'], [f'{{count:1,id:"tconstruct:{n}"}}' for n in ('blazing_bone', 'silky_cloth', 'seared_reinforcement', 'bacon', 'materials_and_you', 'puny_smelting', 'mighty_smelting', 'encyclopedia', 'clear_glass', 'clear_tinted_glass', 'soul_glass', 'clear_glass_pane', 'soul_glass_pane', 'gold_bars', 'gold_platform', 'obsidian_pane', 'cheese_ingot', 'cheese_block', 'jeweled_apple')])
            self.assertEqual((world['page'], world['row'], world['column']), (1, 'TOP', 0))
            self.assertEqual(len(world['items']), 23)
            self.assertIn('{count:1,id:"tconstruct:earth_congealed_slime"}', world['items'])
            self.assertIn('{count:1,id:"tconstruct:sky_slime_ball"}', world['items'])
            self.assertIn('{count:1,id:"tconstruct:necrotic_bone"}', world['items'])
            self.assertIn('{count:1,id:"tconstruct:earth_slime_sapling"}', world['items'])
            self.assertEqual((tools['row'], tools['column']), ('TOP', 5))
            self.assertEqual((blocks['row'], blocks['column']), ('BOTTOM', 5))
            self.assertEqual(tools['items'][0], '{count:1,id:"tconstruct:flint_and_brick"}')
            self.assertIn('{count:1,id:"tconstruct:seared_bricks"}', blocks['items'])
            registries = json.loads((out / 'registries.json').read_text())['minecraft:item']['entries']
            legacy = json.loads((SCRIPT.parent.parent / 'tests/fixtures/manual-overlay-legacy-ids.json').read_text())
            self.assertEqual({name: registries[name]['protocol_id'] for name in legacy}, legacy)
            self.assertGreater(registries['tconstruct:silky_cloth']['protocol_id'], max(legacy.values()))
            self.assertEqual(registries['tconstruct:necrotic_bone']['protocol_id'], registries['tconstruct:silky_cloth']['protocol_id'] + 1)
            items = json.loads((out / 'items.json').read_text())
            for kind in ('splash', 'lingering'):
                name = kind + '_bottle'
                self.assertEqual(items['tconstruct:' + name]['minecraft:max_stack_size'], 64)
                model = json.loads((out / 'assets/tconstruct/models/item' / (name + '.json')).read_text())
                self.assertEqual(model['textures']['layer0'], 'minecraft:item/' + kind + '_potion')
            self.assertEqual(items['tconstruct:encyclopedia']['minecraft:max_stack_size'], 1)
            self.assertTrue((out / 'assets/tconstruct/textures/item/encyclopedia.png').is_file())
            # Bacon is supplied later by the existing content declaration,
            # not appended here where it could renumber prepared item IDs.
            self.assertNotIn('tconstruct:bacon', items)
            self.assertNotIn('tconstruct:jeweled_apple', items)
            soup = items['tconstruct:meat_soup']
            self.assertEqual(soup['minecraft:max_stack_size'], 1)
            self.assertEqual(soup['minecraft:food'], {'nutrition': 8, 'saturation': 9.6})
            for component in ('minecraft:use_remainder', 'minecraft:crafting_remainder'):
                self.assertEqual(soup[component], {'id': 'minecraft:bowl'})
            self.assertEqual(soup['minecraft:consumable']['animation'], 'drink')
            self.assertEqual((out / 'assets/tconstruct/textures/item/food/meat_soup.png').read_bytes(),
                             (Path(__file__).resolve().parents[1] / 'content/assets/tconstruct/textures/item/food/meat_soup.png').read_bytes())
            self.assertEqual(items['tconstruct:silky_cloth']['minecraft:item_model'], 'tconstruct:silky_cloth')
            self.assertEqual(json.loads((out / 'assets/tconstruct/models/item/silky_cloth.json').read_text())['textures']['layer0'], 'tconstruct:item/materials/silky_cloth')
            self.assertEqual((out / 'assets/tconstruct/textures/item/materials/silky_cloth.png').read_bytes(), (SCRIPT.parent.parent / 'content/assets/tconstruct/textures/item/materials/silky_cloth.png').read_bytes())
            self.assertEqual(items['tconstruct:necrotic_bone']['minecraft:item_model'], 'tconstruct:necrotic_bone')
            model = json.loads((out / 'assets/tconstruct/models/item/necrotic_bone.json').read_text())
            self.assertEqual(model['textures']['layer0'], 'tconstruct:item/materials/necrotic_bone')
            self.assertEqual((out / 'assets/tconstruct/textures/item/materials/necrotic_bone.png').read_bytes(), (SCRIPT.parent.parent / 'content/assets/tconstruct/textures/item/materials/necrotic_bone.png').read_bytes())
            blazing = items['tconstruct:blazing_bone']
            self.assertEqual(blazing['minecraft:item_model'], 'tconstruct:blazing_bone')
            model = json.loads((out / 'assets/tconstruct/models/item/blazing_bone.json').read_text())
            self.assertEqual(model['textures']['layer0'], 'tconstruct:item/materials/blazing_bone')
            self.assertEqual((out / 'assets/tconstruct/textures/item/materials/blazing_bone.png').read_bytes(), (SCRIPT.parent.parent / 'content/assets/tconstruct/textures/item/materials/blazing_bone.png').read_bytes())
            seared = items['tconstruct:seared_reinforcement']
            self.assertNotIn('minecraft:cooking_fuel', seared)
            self.assertEqual(seared['minecraft:item_model'], 'tconstruct:seared_reinforcement')
            model = json.loads((out / 'assets/tconstruct/models/item/seared_reinforcement.json').read_text())
            self.assertEqual(model['textures']['layer0'], 'tconstruct:item/materials/reinforcement/seared')
            source_seared = Path(__file__).resolve().parents[1] / 'content/assets/tconstruct/textures/item/materials/reinforcement/seared.png'
            self.assertEqual((out / 'assets/tconstruct/textures/item/materials/reinforcement/seared.png').read_bytes(), source_seared.read_bytes())
            self.assertIn('tconstruct:seared_reinforcement', json.loads((out / 'command-registries.json').read_text())['items'])
            self.assertIn('tconstruct:emerald_reinforcement', items)
            source = Path(__file__).resolve().parents[1] / 'assets/tconstruct/textures/item/materials/reinforcement/emerald.png'
            self.assertEqual((out / 'assets/tconstruct/textures/item/materials/reinforcement/emerald.png').read_bytes(), source.read_bytes())
            self.assertEqual((out / 'assets/minecraft/textures/gui/tconstruct_modifier_reinforced.png').read_bytes(), source.read_bytes())
            for gem in ('diamond', 'emerald'):
                self.assertEqual((out / 'assets/minecraft/textures/gui' / ('tconstruct_modifier_' + gem + '.png')).read_bytes(), gem.encode())
            self.assertEqual((out / 'assets/minecraft/textures/gui/tconstruct_modifier_netherite.png').read_bytes(), b'netherite_ingot')
            self.assertEqual((out / 'assets/minecraft/textures/gui/tconstruct_modifier_overslime.png').read_bytes(), b'slime_ball')
            self.assertEqual((out / 'assets/minecraft/textures/gui/tconstruct_modifier_overforced.png').read_bytes(), (Path(__file__).resolve().parents[1] / 'assets/tconstruct/textures/item/materials/reinforcement/slimesteel.png').read_bytes())
            for modifier in ('quick_charge', 'trueshot', 'unbreakable', 'book', 'tier'):
                self.assertEqual(
                    (out / 'assets/minecraft/textures/gui' / ('tconstruct_modifier_' + modifier + '.png')).read_bytes(),
                    (Path(__file__).resolve().parents[1] / 'assets/tconstruct/gui/modifiers' / (modifier + '.png')).read_bytes(),
                )
            for short in ('modifier_crystal', 'sky_slime_crystal'):
                self.assertIn('tconstruct:' + short, items)
                model = json.loads((out / 'assets/tconstruct/models/item' / (short + '.json')).read_text())
                self.assertEqual(model['textures']['layer0'], 'tconstruct:item/materials/' + short)
                self.assertTrue((out / 'assets/tconstruct/textures/item/materials' / (short + '.png')).is_file())
            self.assertEqual(items['tconstruct:amethyst_bronze_ingot']['minecraft:max_stack_size'], 64)
            with zipfile.ZipFile(Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-smeltery-assets.zip') as smeltery_assets:
                self.assertEqual((out / 'assets/tconstruct/textures/item/amethyst_bronze_ingot.png').read_bytes(), smeltery_assets.read('assets/tconstruct/textures/item/materials/amethyst_bronze_ingot.png'))
            backpack = items['tconstruct:piggy_backpack']
            self.assertEqual(backpack['minecraft:max_stack_size'], 16)
            self.assertEqual(backpack['minecraft:equippable']['slot'], 'chest')
            self.assertNotIn('minecraft:item_name', backpack)  # use locale-specific item translation
            self.assertEqual(json.loads((out / 'assets/tconstruct/models/item/piggy_backpack.json').read_text())['textures']['layer0'], 'tconstruct:item/gadgets/piggy_backpack')
            self.assertTrue((out / 'assets/tconstruct/textures/item/gadgets/piggy_backpack.png').is_file())
            blood = items['tconstruct:blazing_blood_bucket']
            self.assertEqual(blood['minecraft:cooking_fuel'], {'burn_time': {'type': 'minecraft:div', 'left': 30000, 'right': {'type': 'minecraft:conditional', 'condition': 'minecraft:block/fast_cooking', 'on_false': 'minecraft:cooking/normal_burn_time_reduction_factor', 'on_true': 'minecraft:cooking/fast_burn_time_reduction_factor'}}, 'speed_multiplier': 'minecraft:cooking/speed_default'})
            self.assertNotIn('minecraft:cooking_fuel', items['tconstruct:molten_iron_bucket'])
            self.assertEqual(json.loads((out / 'horizon-mod-craft-remainders.json').read_text()),
                             {'example:item': 'minecraft:bowl', 'tconstruct:blazing_blood_bucket': 'minecraft:bucket'})
            self.assertEqual(json.loads((src / 'horizon-mod-craft-remainders.json').read_text()), {'example:item': 'minecraft:bowl'})
            startup = (Path(__file__).resolve().parents[1] / 'mixins/tconstruct/resource-tables.rs').read_text()
            self.assertIn('"left":30000', startup)
            self.assertIn('"minecraft:cooking/speed_default"', startup)
            self.assertIn('entries.insert("tconstruct:blazing_blood_bucket".into(), serde_json::json!("minecraft:bucket"))', startup)
            with zipfile.ZipFile(Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-fluid-buckets.zip') as buckets:
                for name, definition in json.loads(buckets.read('manifest.json'))['buckets'].items():
                    self.assertEqual(items['tconstruct:' + name]['minecraft:max_stack_size'], 1)
                    self.assertEqual(definition['fluid'], 'tconstruct:' + name.removesuffix('_bucket'))
                    self.assertEqual((out / 'assets/tconstruct/textures/item/fluid_bucket' / (name + '.png')).read_bytes(), buckets.read('textures/' + name + '.png'))
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
            self.assertEqual(items['tconstruct:mattock']['minecraft:weapon'], {'item_damage_per_attack': 2})
            for name in ('mattock', 'hand_axe', 'kama', 'pickadze'):
                self.assertIn('minecraft:tool', items['tconstruct:' + name])
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
            for kind in ('earth', 'sky', 'ichor', 'ender'):
                slime_bottle = items[f'tconstruct:{kind}_slime_bottle']
                self.assertEqual(slime_bottle['minecraft:max_stack_size'], 16)
                self.assertEqual(slime_bottle['minecraft:crafting_remainder'], {'id': 'minecraft:glass_bottle'})
                texture = 'ichor' if kind == 'ichor' else f'{kind}_slime'
                self.assertTrue((out / f'assets/tconstruct/textures/item/bottle/{texture}_contents.png').is_file())
            magma = items['tconstruct:magma_bottle']
            self.assertEqual(magma['minecraft:max_stack_size'], 16)
            self.assertEqual(magma['minecraft:use_remainder'], {'id': 'minecraft:glass_bottle'})
            self.assertEqual(magma['minecraft:crafting_remainder'], {'id': 'minecraft:glass_bottle'})
            self.assertNotIn('minecraft:food', magma)
            self.assertNotIn('minecraft:consumable', magma)
            self.assertTrue((out / 'assets/tconstruct/textures/item/bottle/magma_contents.png').is_file())
            ichor = items['tconstruct:ichor_slime_bottle']
            self.assertEqual(ichor['minecraft:consumable']['on_consume_effects'][0]['effects'], [
                {'id': 'minecraft:levitation', 'duration': 200},
                {'id': 'minecraft:slowness', 'duration': 200, 'amplifier': 1}])
            for name in ('cobalt_ingot', 'cobalt_nugget'):
                self.assertEqual(items['tconstruct:' + name]['minecraft:item_model'], 'tconstruct:' + name)
                self.assertNotIn('minecraft:provides_trim_material', items['tconstruct:' + name])
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
            self.assertEqual(ids['tconstruct:blazing_bone']['protocol_id'], ids['tconstruct:piggy_backpack']['protocol_id'] + 1)
            self.assertTrue((out / 'assets/tconstruct/textures').is_dir())
            self.assertFalse((out / 'assets/tconstruct/textures').is_symlink())
            self.assertTrue((out / 'assets/tconstruct/textures/item/tool/pickaxe/head_tconstruct_flint.png').is_file())
            self.assertTrue((out / 'assets/tconstruct/tinkering/materials/flint.json').is_file())
            with zipfile.ZipFile(Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-part-visuals.zip') as archive:
                for code in ('en_us', 'ja_jp'):
                    name = f'assets/tconstruct/lang/{code}.json'
                    actual = json.loads((out / name).read_bytes())
                    expected = json.loads(archive.read(name))
                    key = 'item.tconstruct.piggy_backpack'
                    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
                    from piggy_labels import mark_unavailable
                    expected = mark_unavailable(expected, code)
                    self.assertEqual(actual, expected)
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
            generic = (Path(__file__).resolve().parents[1] / 'assets/tconstruct/gui/generic.png').read_bytes()
            self.assertEqual(hashlib.sha256(generic).hexdigest(), '3f0f7b1e4df1111d40589d29258a740973bc8351a79a3ba2e62a937e1f41c3aa')
            self.assertEqual((out / 'assets/minecraft/textures/gui/tconstruct_generic.png').read_bytes(), generic)
            panel = (Path(__file__).resolve().parents[1] / 'assets/tconstruct/gui/panel.png').read_bytes()
            self.assertEqual(hashlib.sha256(panel).hexdigest(), '6dce4d43b21133c75d774dfb3043bec34f9b326b9b3dd9475ff6973a8b329916')
            self.assertEqual((out / 'assets/minecraft/textures/gui/tconstruct_panel.png').read_bytes(), panel)
            with zipfile.ZipFile(Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-smeltery-assets.zip') as smeltery_assets:
                self.assertEqual((out / 'assets/minecraft/textures/gui/tconstruct_heating_structure.png').read_bytes(), smeltery_assets.read('assets/tconstruct/textures/gui/heating_structure.png'))
            with zipfile.ZipFile(Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-melter-assets.zip') as melter_assets:
                for suffix in ('.png', '.png.mcmeta'):
                    self.assertEqual(
                        (out / 'assets/minecraft/textures/block' / ('tconstruct_fluid_milk' + suffix)).read_bytes(),
                        (Path(__file__).resolve().parents[1] / 'assets/forge/textures/block' / ('milk_still' + suffix)).read_bytes())
                self.assertEqual((out / 'assets/minecraft/textures/gui/tconstruct_melter.png').read_bytes(), melter_assets.read('assets/tconstruct/textures/gui/melter.png'))
                fluids = json.loads(melter_assets.read('assets/tconstruct/melter-fluid-textures.json'))
                self.assertIn('tconstruct:molten_copper', fluids)
                animations = json.loads((Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-melter-fluid-animation.json').read_text())
                self.assertEqual(animations['tconstruct:molten_copper'], {'animation': {'frametime': 2}})
                self.assertEqual(animations['tconstruct:ichor'], {'animation': {'frametime': 3, 'interpolate': True}})
                self.assertEqual(set(fluids) - set(animations), {'tconstruct:powdered_snow'})
                self.assertEqual(json.loads((out / 'assets/minecraft/textures/block/tconstruct_fluid_fallback.png.mcmeta').read_text()), {'animation': {'frametime': 2}})
                for fluid, texture in fluids.items():
                    self.assertEqual((out / 'assets/minecraft/textures/block' / ('tconstruct_fluid_' + fluid.removeprefix('tconstruct:') + '.png')).read_bytes(), melter_assets.read('assets/tconstruct/' + texture))
                    meta = out / 'assets/minecraft/textures/block' / ('tconstruct_fluid_' + fluid.removeprefix('tconstruct:') + '.png.mcmeta')
                    self.assertEqual(meta.exists(), fluid in animations)
                    if fluid in animations:
                        self.assertEqual(json.loads(meta.read_text()), animations[fluid])
            pattern_root = Path(__file__).resolve().parents[1] / 'assets/tconstruct/gui/patterns'
            with zipfile.ZipFile(Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-station-layouts.zip') as layouts:
                for name in layouts.namelist():
                    if not name.endswith('.json'):
                        continue
                    layout = json.loads(layouts.read(name))
                    for slot in [layout.get('tool_slot', {})] + layout.get('input_slots', []):
                        icon = slot.get('icon', '')
                        if icon.startswith('tconstruct:'):
                            self.assertTrue((pattern_root / (icon.split(':', 1)[1] + '.png')).is_file(), (name, icon))
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
