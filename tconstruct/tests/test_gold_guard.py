"""Gold Guard source definitions and authoritative attribute/reload ownership."""
import json
from pathlib import Path
import re
import tomllib
import unittest
from unittest.mock import patch
from tempfile import TemporaryDirectory
import run_gold_guard
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'


class GoldGuard(unittest.TestCase):
    def test_runner_rejects_wrong_revision_and_dirty_source(self):
        with patch.object(run_gold_guard.subprocess, 'check_output', return_value='wrong\n'), \
             patch.object(run_gold_guard.subprocess, 'run') as run:
            with self.assertRaises(RuntimeError):
                run_gold_guard.verify_source(Path('/host'), 'pinned')
            run.assert_not_called()
        with patch.object(run_gold_guard.subprocess, 'check_output', return_value='pinned\n'), \
             patch.object(run_gold_guard.subprocess, 'run', side_effect=run_gold_guard.subprocess.CalledProcessError(1, 'git')):
            with self.assertRaises(run_gold_guard.subprocess.CalledProcessError):
                run_gold_guard.verify_source(Path('/host'), 'pinned')

    def test_runner_rejects_dependency_drift(self):
        package = '[[package]]\nname="serde_json"\nversion="1.0.0"\nsource="registry+https://example.test"\nchecksum="abc"\n'
        with TemporaryDirectory() as directory:
            original, generated = (Path(directory) / n for n in ('original.lock', 'fixture.lock'))
            original.write_text(package)
            generated.write_text(package + '[[package]]\nname="local-fixture"\nversion="0.1.0"\n')
            run_gold_guard.verify_lock(original, generated)
            for changed in (package.replace('1.0.0', '1.0.1'), package.replace('abc', 'changed')):
                generated.write_text(changed)
                with self.assertRaises(RuntimeError):
                    run_gold_guard.verify_lock(original, generated)

    def test_exact_source_definitions(self):
        helper = (MIXINS / 'source-gold-guard.rs').read_text()
        with ZipFile(ROOT / 'content/reference/tconstruct-modifiers.zip') as archive:
            for name, variable in [('gold_guard', 'guard'), ('golden', 'golden')]:
                source = json.loads(archive.read(f'data/tconstruct/tinkering/modifiers/{name}.json'))
                fixture = re.search(rf'let {variable}=br#"(.*?)"#;', helper).group(1)
                self.assertEqual(json.loads(fixture), source)

    def test_host_target_and_reload_publication_boundary(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        hook = next(v for v in manifest['injection'] if v['id'] == 'source-gold-guard-attributes')
        self.assertEqual(hook['kind'], 'RETURN')
        self.assertEqual(hook['target'], "horizon_client::game::enchant_effects::Game<'_>::apply_enchantment_attributes")
        reload = (MIXINS / 'recipe-reload.rs').read_text()
        self.assertLess(reload.index('__horizon_mixins_tconstruct_modifier_resource_snapshot_mut() = Some(modifiers)'),
                        reload.index('tconstruct_gold_guard_world::refresh(game)'))
        helper = (MIXINS / 'gold-guard-world.rs').read_text()
        self.assertIn('game.remote.is_some()', helper)
        self.assertIn('has_modifier(key.clone(),source::MODIFIER)', helper)
        self.assertIn('player.set_health(maximum,false)', helper)
        self.assertNotIn('applied_modifiers.push', helper)


if __name__ == '__main__':
    unittest.main()
