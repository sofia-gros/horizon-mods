"""Dispense delivery source boundary checks, not dispenser runtime proof.

The host already asks its mods before a dispenser ejects (its native
hook 67, `Hooks::dispense_item`); PlutoLib's half was missing, so no
consumer ever heard about a dispense. These checks pin the delivery:
the pure registry, the hook implementation that answers from it, and
the absence of any new weaving (the hook is a plain trait method, so
the manifest must not grow an injection for it).
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
API = ROOT / 'mod' / 'api' / 'src'


class DispenseReachTests(unittest.TestCase):
    def test_the_pure_registry_and_shot_math_exist(self):
        source = (API / 'pure' / 'dispense_api_v1.rs').read_text(encoding='utf-8')
        self.assertIn('pub enum DispenseBehavior', source)
        self.assertIn('Shoot', source)
        self.assertIn('pub struct DispenseRegistry', source)
        self.assertIn('pub fn dispense_shot', source)
        module = (API / 'pure' / 'mod.rs').read_text(encoding='utf-8')
        self.assertIn('pub mod dispense_api_v1;', module)
        exports = (API / 'exports.rs').read_text(encoding='utf-8')
        self.assertIn('DispenseRegistry', exports)

    def test_the_hook_answers_from_the_registry(self):
        hooks = (API / 'hooks.rs').read_text(encoding='utf-8')
        self.assertIn('fn dispense_item', hooks)
        self.assertIn('DispenseRegistry::get', hooks)
        # the projectile launch level event, and one item consumed per shot
        self.assertIn('1002', hooks)
        self.assertIn('host.summon', hooks)

    def test_no_injection_was_added_for_the_dispense_hook(self):
        # The only dispense weaving in the manifest is the armor stand's game-side head; the mod-facing
        # delivery is a plain Hooks trait method and needs no entry of its own.
        manifest = (ROOT / 'mod' / 'sunrise-mod.toml').read_text(encoding='utf-8')
        self.assertEqual(manifest.count('dispense'), 4)
        self.assertIn('pluto-hanging-stand-dispense', manifest)


if __name__ == '__main__':
    unittest.main()
