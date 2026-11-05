"""Narrow guide-key adapter, exercised without a window or real LAN session."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]


class GuideKeyboardNavigation(unittest.TestCase):
    def test_only_book_input_is_intercepted_and_existing_turn_path_is_reused(self):
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
        self.assertIn('game.turn_page(forward)', source)
        for excluded in ('Escape', 'PageUp', 'PageDown', 'save_book', 'close_book'):
            self.assertNotIn(excluded, source)

    @unittest.skipUnless(shutil.which('rustc'), 'rustc unavailable for bounded adapter fixture')
    def test_production_keyboard_and_turn_adapters(self):
        with tempfile.TemporaryDirectory(prefix='book-keyboard-') as tmp:
            binary = Path(tmp) / 'book-keyboard-tests'
            subprocess.run(['rustc', '--edition=2021', '--test', str(ROOT / 'tests/rust/book_keyboard_navigation.rs'), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
