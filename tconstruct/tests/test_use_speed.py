"""Source contracts for the high-priority Unburdened use-speed bridge."""
import json
from pathlib import Path
import re
import tomllib
import unittest
from tempfile import TemporaryDirectory
import run_use_speed
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'


class UseSpeed(unittest.TestCase):
    def test_native_body_guard_rejects_unrelated_edits(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'crates/horizon-client/src/game.rs'
            source.parent.mkdir(parents=True)
            source.write_text('    pub(crate) fn sync_player_body(&mut self) {\n        let item_use_speed = item_in_use.map(|i| i.use_speed);\n        self.auto_jump = true;\n        return;\n    }\n    fn held_slot(&self) {}')
            helper = root / 'helper.rs'
            body = '// BEGIN PINNED NATIVE BODY\nlet item_use_speed = Some(speed);\ngame.auto_jump = true;\nreturn std::ops::ControlFlow::Break(());\n// END PINNED NATIVE BODY'
            helper.write_text(body)
            run_use_speed.verify_native_body(root, helper)
            helper.write_text(body.replace('auto_jump = true', 'auto_jump = false'))
            with self.assertRaises(RuntimeError):
                run_use_speed.verify_native_body(root, helper)

    def test_four_exact_source_definitions(self):
        text = (MIXINS / 'source-use-speed.rs').read_text()
        literals = re.findall(r'r#"(.*?)"#', text.split('fn known', 1)[0])
        self.assertEqual(len(literals), 4)
        with ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as archive:
            for name, literal in zip(('unburdened', 'cobalamin', 'featherweight', 'melee_protection'), literals):
                self.assertEqual(json.loads(literal), json.loads(archive.read(f'data/tconstruct/tinkering/modifiers/{name}.json')))

    def test_source_tool_defaults_and_override_examples(self):
        expected = {'dagger': 1.0, 'travelers_shield': 0.8, 'sky_staff': 0.4, 'earth_staff': 0.4,
                    'ichor_staff': 0.4, 'ender_staff': 0.4}
        with ZipFile(ROOT / 'content/reference/tconstruct-tool-definitions.zip') as archive:
            found = {}
            for name in archive.namelist():
                value = json.loads(archive.read(name))
                for module in value.get('modules', []):
                    if 'tconstruct:use_item_speed' in module.get('stats', {}):
                        found[Path(name).stem] = module['stats']['tconstruct:use_item_speed']
        self.assertEqual(found, expected)

    def test_real_input_boundary_and_named_ci_assertion(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        hook = next(v for v in manifest['injection'] if v['id'] == 'source-use-speed-body')
        self.assertEqual(hook['target'], "horizon_client::game::Game<'_>::sync_player_body")
        self.assertEqual(hook['kind'], 'HEAD')
        self.assertTrue(hook['cancellable'])
        self.assertEqual(sum(v['target'] == hook['target'] for v in manifest['injection']), 1)
        helper = (MIXINS / 'source-use-speed.rs').read_text()
        self.assertIn('game.remote.is_some() || game.riding.is_some()', helper)
        self.assertRegex(helper, r'if total != trait_level \|\| trait_level < 0')
        workflow = (ROOT.parent / '.github/workflows/mixins.yml').read_text()
        self.assertIn('python3 tconstruct/tests/run_use_speed.py --horizon ../horizon-official', workflow)
        self.assertIn('test game::tconstruct_use_speed::tests::held_stat_uses_defaults_additions_multipliers_and_clamp ... ok', workflow)


if __name__ == '__main__':
    unittest.main()
