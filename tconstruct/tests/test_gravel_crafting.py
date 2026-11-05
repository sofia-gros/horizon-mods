"""Source contract and native per-load wiring for optional gravel crafting."""
import hashlib
import json
from pathlib import Path
import re
import tomllib
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'


class GravelCrafting(unittest.TestCase):
    def test_projection_retains_source_inputs_output_and_setting(self):
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            source = json.loads(archive.read('data/tconstruct/recipes/common/flint.json'))
        self.assertEqual(source['conditions'], [{'type': 'tconstruct:config', 'prop': 'gravel_to_flint'}])
        helper = (MIXINS / 'source-gravel-crafting.rs').read_text()
        native = json.loads(re.search(r'const RECIPE:.*?br#"(.*?)"#;', helper).group(1))
        self.assertEqual(native, {'type': source['type'], 'category': source['category'],
            'ingredients': [value['item'] for value in source['ingredients']],
            'result': {'id': source['result']['item'], 'count': source['result'].get('count', 1)}})
        self.assertEqual(native['ingredients'], ['minecraft:gravel'] * 3)
        self.assertEqual(native['result'], {'id': 'minecraft:flint', 'count': 1})
        self.assertIn('config.get("gravel_to_flint") == Some(&true)', helper)
        # Nothing is statically published that could bypass the per-world gate.
        for path in (ROOT / 'content/data').glob('*/recipe/**/*.json'):
            value = json.loads(path.read_text())
            self.assertNotEqual(value.get('result', {}).get('id') if isinstance(value.get('result'), dict) else None,
                                'minecraft:flint', str(path))

    def test_source_discovery_maps_event_key_without_unconditional_install(self):
        fixture = ROOT / 'tests/fixtures/flint_discovery'
        raw = (fixture / 'flint.json').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), json.loads((fixture / 'sha256.json').read_text())['flint'])
        expected = json.loads(raw)
        expected['criteria']['has_the_recipe']['conditions'] = {'recipes': 'tconstruct:common/flint'}
        helper = (MIXINS / 'source-gravel-crafting.rs').read_text()
        native = json.loads(re.search(r'const ADVANCEMENT:.*?br#"(.*?)"#;', helper).group(1))
        self.assertEqual(native, expected)
        self.assertEqual(native['criteria']['has_item']['conditions'], {'items': [{'items': ['minecraft:gravel']}]})
        self.assertEqual(native['requirements'], [['has_item', 'has_the_recipe']])
        self.assertEqual(native['rewards']['recipes'], ['tconstruct:common/flint'])
        self.assertNotIn('display', native)
        self.assertIn('"advancement/tconstruct:recipes/misc/common/flint"', helper)
        for path in (ROOT / 'content/data').glob('*/advancement/**/*.json'):
            self.assertNotIn('tconstruct:common/flint', json.loads(path.read_text()).get('rewards', {}).get('recipes', []), str(path))

    def test_native_contribution_hook_uses_authoritative_world_config(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        injection = next(value for value in manifest['injection'] if value['id'] == 'source-gravel-crafting-files')
        self.assertEqual(injection['target'], "horizon_client::game::native_hooks::Game<'_>::native_data_pack_files")
        self.assertEqual(injection['kind'], 'RETURN')
        helper = (MIXINS / injection['helper']).read_text()
        self.assertIn('game.remote.is_some()', helper)
        self.assertIn('game.world_root.as_os_str().is_empty()', helper)
        self.assertIn('game.standalone_server()', helper)
        self.assertIn('root_for_world(&game.world_root, dedicated, &working_dir)', helper)
        self.assertIn('tconstruct_gravel_crafting::contribute(&root, files)', helper)


if __name__ == '__main__':
    unittest.main()
