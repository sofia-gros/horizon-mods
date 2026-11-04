"""Keep the LAN guard on the two fixed-host recipe reload entry points."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
MIXINS = ROOT / 'mixins/tconstruct'

class LanCraftReloadWiring(unittest.TestCase):
    def test_both_reload_entries_invalidate_before_capture(self):
        manifest = tomllib.loads((MIXINS / 'horizon-mixin.toml').read_text())
        injections = {entry['id']: entry for entry in manifest['injection']}
        self.assertNotIn('lan-craft-reload-generation', injections)
        for name, target, helper in (
            ('recipe-reload-command', "horizon_client::game::functions::Game<'_>::cmd_reload", 'recipe-command.rs'),
            ('recipe-pack-change', "horizon_client::game::functions::Game<'_>::apply_pack_change", 'recipe-pack.rs'),
        ):
            with self.subTest(name=name):
                entry = injections[name]
                self.assertEqual((entry['target'], entry['kind'], entry['helper']), (target, 'RETURN', helper))
                text = (MIXINS / helper).read_text()
                self.assertLess(text.index('invalidate_lan_crafting_results(game)'), text.index('capture(game)'))
        self.assertIn('if value.is_ok()', (MIXINS / 'recipe-command.rs').read_text())

if __name__ == '__main__':
    unittest.main()
