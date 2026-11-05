"""Pinned source contracts and wiring for conditional slimeball alternatives."""
import json
from pathlib import Path
import re
import sys
import tempfile
import tomllib
import unittest
from zipfile import ZipFile
from test_survival_reachability import tag_items

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'
sys.path.insert(0, str(ROOT / 'tools'))
from package_hmod import package


class SlimeCrafting(unittest.TestCase):
    def test_two_projected_recipes_match_pinned_source(self):
        helper = (MIXINS / 'source-slime-crafting.rs').read_text()
        native = re.findall(r'\("recipe/tconstruct:(.*?)", br##"(.*?)"##\)', helper)
        self.assertEqual(len(native), 2)
        with ZipFile(ROOT / 'content/reference/tconstruct-recipes.zip') as archive:
            for path, text in native:
                with self.subTest(recipe=path):
                    source = json.loads(archive.read(f'data/tconstruct/recipes/{path}.json'))
                    self.assertEqual(source['conditions'], [{'type': 'tconstruct:config', 'prop': 'slime_recipe_fix'}])
                    expected = {key: source[key] for key in ('type', 'category', 'pattern')}
                    expected['key'] = {key: '#' + value['tag'] if 'tag' in value else value['item'] for key, value in source['key'].items()}
                    expected['result'] = {'id': source['result']['item'], 'count': source['result'].get('count', 1)}
                    self.assertEqual(json.loads(text), expected)
                    self.assertFalse((ROOT / f'content/data/tconstruct/recipe/{path}.json').exists())
        self.assertIn('config.get("slime_recipe_fix") != Some(&true)', helper)

    def test_existing_slimeball_family_supplies_all_four_types(self):
        self.assertEqual(tag_items('forge:slimeballs'), {'minecraft:slime_ball',
            'tconstruct:sky_slime_ball', 'tconstruct:ichor_slime_ball', 'tconstruct:ender_slime_ball'})
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'tconstruct.hmod'
            package(output)
            with ZipFile(output) as archive:
                for name in ('sky_slime_ball', 'ichor_slime_ball', 'ender_slime_ball'):
                    self.assertIn(f'data/tconstruct/horizon/item/{name}.json', archive.namelist())

    def test_ci_runs_rust_and_overlay_regressions(self):
        workflow = (ROOT.parent / '.github/workflows/mixins.yml').read_text()
        self.assertIn('cargo test --locked --manifest-path tconstruct/tests/slime-crafting/Cargo.toml', workflow)
        self.assertIn('run: python3 tconstruct/tests/run_slime_crafting_overlay_rust.py', workflow)
        self.assertLess(workflow.index('name: Fetch upstream sources'),
                        workflow.index('name: Test slime crafting overlay precedence and rebuild'))
        runner = (ROOT / 'tests/run_slime_crafting_overlay_rust.py').read_text()
        self.assertIn("['cargo', 'test', '--locked', '--offline'", runner)

    def test_per_load_native_hook_and_server_root_selection(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        injection = next(value for value in manifest['injection'] if value['id'] == 'source-slime-crafting-files')
        self.assertEqual(injection['target'], "horizon_client::game::native_hooks::Game<'_>::native_data_pack_files")
        self.assertEqual(injection['kind'], 'RETURN')
        helper = (MIXINS / injection['helper']).read_text()
        for text in ('game.remote.is_some()', 'game.world_root.as_os_str().is_empty()',
                     'game.standalone_server()', 'root_for_world(&game.world_root, dedicated, &working_dir)',
                     'tconstruct_slime_crafting::contribute(&root, files)'):
            self.assertIn(text, helper)


if __name__ == '__main__':
    unittest.main()
