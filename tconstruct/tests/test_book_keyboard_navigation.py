"""Narrow guide-key adapter, exercised without a window or real LAN session."""
from pathlib import Path
import shutil
import subprocess
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]


class GuideKeyboardNavigation(unittest.TestCase):
    def test_input_uses_guide_policy_without_intercepting_native_turn(self):
        manifest = tomllib.loads((ROOT / 'mixins/tconstruct/horizon-mixin.toml').read_text())
        hooks = [hook for hook in manifest['injection'] if hook['id'] == 'materials-book-key']
        self.assertEqual(len(hooks), 1)
        hook = hooks[0]
        self.assertEqual(hook['target'], "horizon_client::game::books::Game<'_>::book_input")
        self.assertEqual((hook['kind'], hook['cancellable'], hook['helper']), ('HEAD', True, 'source-materials-book-key.rs'))
        self.assertEqual(hook['bindings'], [
            {'name': 'game', 'ty': "&mut Game<'_>", 'value': '&mut *self'},
            {'name': 'key', 'ty': 'Option<KeyCode>', 'value': 'key'},
        ])
        source = (ROOT / 'mixins/tconstruct/source-materials-book-key.rs').read_text()
        self.assertIn('BookScreen::View { pages, lectern: None, .. }', source)
        self.assertIn('Some("tconstruct:materials-book-root")', source)
        self.assertIn('tconstruct_materials_book_navigation::turn(game, forward)', source)
        self.assertFalse(any(h['target'].endswith("::turn_page") for h in manifest['injection']))
        for key in ('PageUp', 'PageDown'): self.assertIn(key, source)
        for excluded in ('Escape', 'save_book', 'close_book', 'game.turn_page('):
            self.assertNotIn(excluded, source)

    @unittest.skipUnless(shutil.which('cargo'), 'cargo unavailable for bounded adapter fixture')
    def test_production_keyboard_click_and_control_adapters(self):
        result = subprocess.run([
            'cargo', 'test', '--locked', '--offline', '--manifest-path',
            str(ROOT / 'tests/book-wire/Cargo.toml'), '--lib',
            'book_keyboard_navigation::', '--', '--test-threads=1',
            '--skip', 'book_keyboard_navigation::tconstruct_materials_book_spread::',
        ], check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        print(result.stdout)
        self.assertIn('8 passed; 0 failed', result.stdout)
        self.assertNotIn('ignored,', result.stdout.split('test result:')[0])


if __name__ == '__main__':
    unittest.main()
