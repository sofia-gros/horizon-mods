"""Keep the content package within Horizon's loader limit and retain renderer aliases."""
import sys
import json
import tempfile
import unittest
import zipfile
from pathlib import Path

MOD = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class ContentPackageTest(unittest.TestCase):
    def test_loader_limit_and_source_sprite_aliases(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with zipfile.ZipFile(output) as packed:
                names = set(packed.namelist())
                guide = 'assets/tconstruct/book/materials_and_you/en_us/intro/tinkerschest_craft.json'
                self.assertEqual(packed.read(guide), (MOD / 'content' / guide).read_bytes())
                for path in ('assets/tconstruct/items/blazing_bone.json', 'assets/tconstruct/models/item/blazing_bone.json', 'assets/tconstruct/textures/item/materials/blazing_bone.png', 'data/tconstruct/horizon/item/blazing_bone.json'):
                    actual, expected = packed.read(path), (MOD / 'content' / path).read_bytes()
                    self.assertEqual(json.loads(actual), json.loads(expected)) if path.endswith('.json') else self.assertEqual(actual, expected)
                for path in ('assets/tconstruct/items/silky_cloth.json', 'assets/tconstruct/models/item/silky_cloth.json', 'assets/tconstruct/textures/item/materials/silky_cloth.png', 'data/tconstruct/horizon/item/silky_cloth.json'):
                    actual, expected = packed.read(path), (MOD / 'content' / path).read_bytes()
                    self.assertEqual(json.loads(actual), json.loads(expected)) if path.endswith('.json') else self.assertEqual(actual, expected)
                page = 'assets/tconstruct/book/mighty_smelting/en_us/smeltery/tanks.json'
                self.assertEqual(json.loads(packed.read(page)), json.loads((MOD / 'content' / page).read_bytes()))
                for block in ('seared_tinted_glass', 'scorched_tinted_glass'):
                    path = f'data/tconstruct/horizon/block/{block}.json'
                    self.assertEqual(json.loads(packed.read(path)), json.loads((MOD / 'content' / path).read_text()))
                    self.assertEqual(json.loads(packed.read(path))['template'], 'minecraft:tinted_glass')
                for block in ('blood_slime_fern', 'ichor_slime_fern'):
                    path = f'data/tconstruct/loot_table/blocks/{block}.json'
                    self.assertEqual(json.loads(packed.read(path)), json.loads((MOD / 'content' / path).read_text()))
                    self.assertEqual(json.loads(packed.read(path))['pools'][0]['condition'], {'type':'minecraft:survives_explosion'})
                # The MIT text travels with the content containing upstream pixels.
                self.assertEqual(packed.read('assets/tconstruct/licenses/tconstruct.txt'),
                                 (MOD / 'third_party/tconstruct/LICENSE').read_bytes())
                path = 'assets/tconstruct/textures/block/clear_tinted_glass.png'
                self.assertEqual(packed.read(path), (MOD / 'content' / path).read_bytes())
                self.assertFalse(any(name.startswith('assets/tconstruct/textures/block/clear_tinted_glass/') for name in names))
                for path in ('assets/tconstruct/items/magma_bottle.json',
                             'assets/tconstruct/models/item/magma_bottle.json',
                             'assets/tconstruct/textures/item/bottle/magma_contents.png',
                             'data/tconstruct/horizon/item/magma_bottle.json'):
                    self.assertIn(path, names)
                animations = json.loads((MOD / 'content/reference/tconstruct-melter-fluid-animation.json').read_text())
                for fluid, animation in animations.items():
                    path = 'assets/tconstruct/runtime/block/tconstruct_fluid_' + fluid.removeprefix('tconstruct:') + '.png.mcmeta'
                    self.assertEqual(json.loads(packed.read(path)), animation)
                for path in ('assets/tconstruct/source-packs/tconstruct-original-pack/data/tconstruct/tinkering/tags/modifiers/abilities/general.json',
                             'assets/tconstruct/source-packs/tconstruct-original-pack/data/tconstruct/tinkering/modifiers/expanded.json'):
                    self.assertIn(path, names)
                self.assertNotIn('assets/tconstruct/runtime/block/tconstruct_fluid_powdered_snow.png.mcmeta', names)
                self.assertIn('assets/tconstruct/runtime/block/tconstruct_fluid_fallback.png.mcmeta', names)
                for suffix in ('.png', '.png.mcmeta'):
                    self.assertEqual(
                        packed.read('assets/tconstruct/runtime/block/tconstruct_fluid_milk' + suffix),
                        (MOD / 'assets/forge/textures/block' / ('milk_still' + suffix)).read_bytes())
                with zipfile.ZipFile(MOD / 'content/reference/tconstruct-part-visuals.zip') as source:
                    languages = [name for name in source.namelist() if name.startswith('assets/tconstruct/lang/') and name.endswith('.json')]
                    self.assertEqual(len(languages), 20)
                    for name in languages:
                        self.assertIn(name, names)
                        expected = json.loads((MOD / 'content/assets/tconstruct/lang/en_us.json').read_bytes()
                                              if name.endswith('/en_us.json') else source.read(name))
                        if not name.endswith('/en_us.json'):
                            sys.path.insert(0, str(MOD / 'tools'))
                            from piggy_labels import mark_unavailable
                            expected = mark_unavailable(expected, Path(name).stem)
                        self.assertEqual(json.loads(packed.read(name)), expected, name)
                    self.assertEqual(json.loads(packed.read('assets/tconstruct/lang/ja_jp.json'))['item.tconstruct.copper_can.contents'], '%s: 1インゴット分')
                    self.assertEqual(json.loads(packed.read('assets/tconstruct/lang/en_us.json'))['item.tconstruct.copper_can.contents'], '%s: 1 ingot')
            self.assertLessEqual(len(names - {'horizon-mod.toml'}), 65_536)
            for kind in ('part', 'tool'):
                with zipfile.ZipFile(MOD / 'content/reference' / f'tconstruct-{kind}-visuals.zip') as source:
                    sprites = [n.removeprefix('assets/tconstruct/textures/').removesuffix('.png')
                               for n in source.namelist()
                               if n.startswith('assets/tconstruct/textures/') and n.endswith('.png')]
                for sprite in sprites:
                    alias = f'tconstruct_{kind}/{sprite}.json'
                    self.assertIn(f'assets/tconstruct/items/{alias}', names)
                    self.assertIn(f'assets/tconstruct/models/item/{alias}', names)


if __name__ == '__main__':
    unittest.main()
