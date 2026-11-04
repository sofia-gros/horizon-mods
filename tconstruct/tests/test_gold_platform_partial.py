"""Bounded Gold Platform item, recipe, casting tag and packaging checks."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
sys.path.insert(0, str(ROOT / 'tools'))
from piggy_labels import WARNINGS, PLATFORM_WARNINGS, mark_unavailable
from package_hmod import package

class GoldPlatformPartial(unittest.TestCase):
    def test_source_recipes_and_basin_cast(self):
        with ZipFile(CONTENT / 'reference/tconstruct-recipes.zip') as source:
            gold = json.loads(source.read('data/tconstruct/recipes/common/gold_platform.json'))
            basin = json.loads(source.read('data/tconstruct/recipes/smeltery/scorched/basin.json'))
        for original, name in ((gold, 'gold_platform'), (basin, 'scorched_basin')):
            actual = json.loads((CONTENT / f'data/tconstruct/recipe/{name}.json').read_text())
            self.assertEqual(actual['pattern'], original['pattern'])
            expected = {key: ('#' + val['tag'] if 'tag' in val else val['item']) for key, val in original['key'].items()}
            self.assertEqual(actual['key'], expected)
            self.assertEqual(actual['result']['id'], original['result']['item'])
            self.assertEqual(actual['result']['count'], original['result'].get('count', 1))
        tag = json.loads((CONTENT / 'reference/tconstruct-tags/data/tconstruct/tags/items/casts/empty/basin.json').read_text())
        self.assertIn('tconstruct:gold_platform', tag['values'])
        with ZipFile(CONTENT / 'reference/forge-47.4.0-data.zip') as forge:
            for kind, item in [('ingots', 'minecraft:gold_ingot'), ('nuggets', 'minecraft:gold_nugget')]:
                self.assertIn(item, json.loads(forge.read(f'data/forge/tags/items/{kind}/gold.json'))['values'])

    def test_item_only_and_localized_warning(self):
        self.assertEqual(json.loads((CONTENT / 'data/tconstruct/horizon/item/gold_platform.json').read_text()), {'template': 'minecraft:gold_ingot'})
        self.assertFalse((CONTENT / 'data/tconstruct/horizon/block/gold_platform.json').exists())
        self.assertEqual(set(PLATFORM_WARNINGS), set(WARNINGS))
        for locale in WARNINGS:
            language = mark_unavailable({'block.tconstruct.gold_platform': 'Gold Platform'}, locale)
            self.assertIn(PLATFORM_WARNINGS[locale], language['item.tconstruct.gold_platform'])
        english = json.loads((CONTENT / 'assets/tconstruct/lang/en_us.json').read_text())
        self.assertIn('placement unavailable', english['item.tconstruct.gold_platform'])
        groups = json.loads((CONTENT / 'creative-tabs.json').read_text())
        self.assertEqual(sum('tconstruct:gold_platform' in item for group in groups for item in group['items']), 1)

    def test_native_package_and_windows_prepare_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp) / 'tconstruct.hmod'; package(target)
            with ZipFile(target) as archive:
                for path in ('data/tconstruct/horizon/item/gold_platform.json', 'data/tconstruct/recipe/gold_platform.json',
                             'data/tconstruct/recipe/scorched_basin.json', 'data/forge/tags/item/ingots/gold.json',
                             'data/forge/tags/item/nuggets/gold.json', 'assets/tconstruct/models/item/gold_platform.json',
                             'assets/tconstruct/textures/block/platform/gold_top.png'):
                    self.assertEqual(archive.read(path), (CONTENT / path).read_bytes())
                japanese = json.loads(archive.read('assets/tconstruct/lang/ja_jp.json'))
                self.assertIn(PLATFORM_WARNINGS['ja_jp'], japanese['item.tconstruct.gold_platform'])
        launcher = (ROOT / 'tools/play-windows.ps1').read_text()
        self.assertLess(launcher.index("'tools/resources.py'"), launcher.index("'--tconstruct-prepare'"))
        prepare = (ROOT / 'mixins/tconstruct/prepare.rs').read_text()
        self.assertIn('Content::read(&content_path, &manifest)', prepare)
        self.assertIn('mod_content::build(&base, &[content_mod], &out)', prepare)

if __name__ == '__main__':
    unittest.main()
