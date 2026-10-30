"""Verify the 26.3 to source-pack layout adapter."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

class HorizonSourceData(unittest.TestCase):
    def test_adapt_layout_and_preserve_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp) / 'resources'; base.mkdir()
            (base / 'horizon-resources.json').write_text('{}')
            cases = [('recipe/a.json', 'recipes/a.json'), ('tags/item/nested/a.json', 'tags/items/nested/a.json'), ('tags/fluid/a.json', 'tags/fluids/a.json')]
            for source, _ in cases:
                path = base / 'data/minecraft' / source; path.parent.mkdir(parents=True, exist_ok=True); path.write_text('{"test":true}')
            out = Path(tmp) / 'pack'
            cmd = [sys.executable, str(ROOT / 'tools/prepare_horizon_source_data.py'), str(base), str(out)]
            subprocess.run(cmd, check=True, capture_output=True)
            for source, dest in cases:
                self.assertEqual((base / 'data/minecraft' / source).read_bytes(), (out / 'data/minecraft' / dest).read_bytes())
            self.assertNotEqual(subprocess.run(cmd, capture_output=True).returncode, 0)
