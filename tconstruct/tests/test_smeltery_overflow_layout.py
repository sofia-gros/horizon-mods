"""Compile the production layout helper and verify shared UI call sites."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

class SmelteryOverflowLayout(unittest.TestCase):
    @unittest.skipUnless(shutil.which('rustc'), 'rustc not installed')
    def test_production_layout_against_source_algorithm(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / 'layout-tests'
            subprocess.run(['rustc', '--edition=2021', '--test', str(ROOT/'tests/rust/smeltery_overflow_layout.rs'), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)

    def test_draw_hover_tooltip_and_pointer_use_shared_geometry(self):
        modules = ROOT/'mixins/tconstruct'
        draw = (modules/'source-smeltery-draw.rs').read_text()
        self.assertIn('tconstruct_source_smeltery_layout::fluid_heights(&amounts, tank.capacity)', draw)
        for name in ['source-smeltery-draw.rs', 'source-smeltery-pointer.rs', 'station-result-tooltip.rs']:
            self.assertIn('tconstruct_source_smeltery_layout::fluid_at(&amounts, tank.capacity,', (modules/name).read_text())
