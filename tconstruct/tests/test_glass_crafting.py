"""Source-exact vanilla glass crafts and authoritative per-world hook wiring."""
import json
from pathlib import Path
import re
import tomllib
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'
NAMES = ('glass_bottle', 'beacon', 'daylight_detector', 'end_crystal')


class GlassCrafting(unittest.TestCase):
    def test_native_projections_preserve_source_patterns_inputs_and_counts(self):
        helper = (MIXINS / 'source-glass-crafting.rs').read_text()
        recipes = dict(re.findall(r'\("recipe/tconstruct:common/glass/vanilla/([^"]+)", br##"(.*?)"##\)', helper))
        self.assertEqual(set(recipes), set(NAMES))
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for name in NAMES:
                with self.subTest(recipe=name):
                    original = json.loads(archive.read(f'data/tconstruct/recipes/common/glass/vanilla/{name}.json'))
                    self.assertEqual(original['conditions'], [{'type': 'tconstruct:config', 'prop': 'glass_recipe_fix'}])
                    expected = dict(original)
                    del expected['conditions']
                    expected['key'] = {key: '#' + v['tag'] if 'tag' in v else v['item'] for key, v in original['key'].items()}
                    expected['result'] = {'id': original['result']['item'], 'count': original['result'].get('count', 1)}
                    self.assertEqual(json.loads(recipes[name]), expected)
        self.assertIn('config.get("glass_recipe_fix") != Some(&true)', helper)

    def test_native_tag_dependency_has_both_clear_glass_inputs(self):
        # This feature depends on the shared native tag added for machine crafting.
        tag = json.loads((ROOT / 'content/data/forge/tags/item/glass/colorless.json').read_text())
        self.assertEqual(set(tag['values']), {'minecraft:glass', 'tconstruct:clear_glass'})
        self.assertFalse(tag.get('replace', False))
        self.assertTrue((ROOT / 'content/data/tconstruct/horizon/block/clear_glass.json').is_file())

    def test_hook_is_per_world_and_no_static_recipe_bypasses_config(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        module = next(v for v in manifest['module'] if v['id'] == 'source-glass-crafting')
        self.assertEqual(module['source'], 'source-glass-crafting.rs')
        hook = next(v for v in manifest['injection'] if v['id'] == 'source-glass-crafting-files')
        self.assertEqual(hook['target'], "horizon_client::game::native_hooks::Game<'_>::native_data_pack_files")
        self.assertEqual(hook['kind'], 'RETURN')
        helper = (MIXINS / hook['helper']).read_text()
        for fragment in ('game.remote.is_some()', 'game.world_root.as_os_str().is_empty()',
                         'game.standalone_server()', 'root_for_world(&game.world_root, dedicated, &working_dir)',
                         'tconstruct_glass_crafting::contribute(&root, files)'):
            self.assertIn(fragment, helper)
        # Ordinary datapack JSON has no portable support for this Forge condition.
        for path in (ROOT / 'content/data').glob('*/recipe/**/*.json'):
            recipe = json.loads(path.read_text())
            result = recipe.get('result', {})
            result = result.get('id') if isinstance(result, dict) else result
            self.assertNotIn(result, {f'minecraft:{name}' for name in NAMES}, str(path))


if __name__ == '__main__':
    unittest.main()
