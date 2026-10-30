"""Validate the source snapshot used to implement connected glass."""
import json
import unittest
import zipfile
from pathlib import Path

SNAPSHOT = Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-connected-glass.zip'


class ConnectedGlassSourceTests(unittest.TestCase):
    def test_models_and_connected_sprites_are_present(self):
        with zipfile.ZipFile(SNAPSHOT) as archive:
            files = {path.split('/assets/tconstruct/', 1)[1]: path for path in archive.namelist()}
            states = [name for name in files if name.startswith('blockstates/')]
            self.assertEqual(len(states), 42)
            models = [json.loads(archive.read(path)) for name, path in files.items()
                      if name.startswith('models/') and name.endswith('.json')]
            textures = {name[len('textures/'): -4] for name in files
                        if name.startswith('textures/') and name.endswith('.png')}
            connected = set()
            for model in models:
                for value in model.get('textures', {}).values():
                    if isinstance(value, str) and value.startswith('tconstruct:'):
                        self.assertIn(value.split(':', 1)[1], textures)
                if model.get('loader') == 'mantle:connected':
                    keys = model.get('connection', {}).get('textures', {})
                    for key, kind in keys.items():
                        self.assertEqual(kind, 'cornerless_full')
                        connected.add(model['textures'][key].split(':', 1)[1])
            self.assertTrue(connected)
            for base in connected:
                for mask in range(1, 16):
                    suffix = ''.join(letter for bit, letter in enumerate('udlr') if mask & (1 << bit))
                    self.assertIn(f'{base}/{suffix}', textures)


if __name__ == '__main__':
    unittest.main()
