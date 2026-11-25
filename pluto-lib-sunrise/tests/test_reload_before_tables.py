"""Parse-order source boundary checks, not recipe-parse/runtime proof.

The host builds a data load's content tables before it applies the mods'
JSON reload listeners, so a mod ingredient kind's load-time item list was
answered from data that had not loaded yet and the recipes using one were
dropped. These checks pin the two heads that apply the listeners first.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / 'mod'


class ReloadBeforeTablesTests(unittest.TestCase):
    def manifest(self):
        return tomllib.loads((BUNDLE / 'sunrise-mod.toml').read_text(encoding='utf-8'))

    def injection(self, wanted):
        return next(row for row in self.manifest()['injection'] if row['id'] == wanted)

    def test_world_open_applies_the_listeners_before_the_tables(self):
        hook = self.injection('pluto-reload-before-tables-open')
        self.assertEqual(hook['target'], "sunrise_client::game::Game<'_>::finish_open_world")
        self.assertEqual(hook['kind'], 'HEAD')
        self.assertNotIn('cancellable', hook)
        self.assertEqual(hook['helper'], 'pluto-reload-before-tables.rs')
        self.assertEqual(hook['bindings'], [
            {'name': 'game', 'ty': "&mut Game<'_>", 'value': '&mut *self'},
            {'name': 'prepared', 'ty': '&crate::game::world_loading::PreparedWorld', 'value': '&prepared'},
        ])

    def test_reload_applies_the_listeners_before_the_tables(self):
        hook = self.injection('pluto-reload-before-tables-reload')
        self.assertEqual(hook['target'], "sunrise_client::game::world_entry::Game<'_>::reload_world_tables")
        self.assertEqual(hook['kind'], 'HEAD')
        self.assertNotIn('cancellable', hook)
        self.assertEqual(hook['helper'], 'pluto-reload-before-tables-reload.rs')  # one private function per helper file (E_INVALID_HELPER, build-sp weave plan 2026-11-23)
        self.assertEqual(hook['bindings'], [
            {'name': 'game', 'ty': "&mut Game<'_>", 'value': '&mut *self'},
        ])

    def test_the_open_head_sets_the_world_root_before_applying(self):
        source = (BUNDLE / 'pluto-reload-before-tables.rs').read_text(encoding='utf-8')
        open_fn = source.index('fn pluto_reload_before_tables_open')
        root = source.index('game.world_root = prepared.save_root.clone();')
        apply = source.index('game.native_reload_listeners();')
        self.assertLess(open_fn, root)
        self.assertLess(root, apply)

    def test_the_reload_head_keeps_the_remote_return(self):
        source = (BUNDLE / 'pluto-reload-before-tables-reload.rs').read_text(encoding='utf-8')
        reload_fn = source.index('fn pluto_reload_before_tables_reload')
        guard = source.index('game.remote.is_some()')
        apply = source.index('game.native_reload_listeners();', reload_fn)
        self.assertLess(reload_fn, guard)
        self.assertLess(guard, apply)


if __name__ == '__main__':
    unittest.main()
