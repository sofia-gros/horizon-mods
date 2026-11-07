"""Five existing crop call sites reuse Pluto; source policy stays local."""
from pathlib import Path
import hashlib
import json
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONSUMER = ROOT / 'tconstruct/mixins/tconstruct'


def function(name):
    s = (CONSUMER / 'source-melter-render.rs').read_text()
    return s.split('pub(super) fn ' + name + '(', 1)[1].split('\n}', 1)[0]


class RenderCropConsumers(unittest.TestCase):
    def test_heat_fire_keep_selection_and_empty_behavior(self):
        heat = function('draw_heat_bar')
        fire = function('draw_fire')
        for body in (heat, fire):
            self.assertIn('pluto_render::sprite(atlas, texture)', body)
            self.assertIn('pluto_render::crop(batch, sprite,', body)
            self.assertIn('sprite.logical_size()', body)
            self.assertNotIn('sprite.u0', body)
        self.assertIn('if height <= 0.0 { return; }', heat)
        self.assertIn('[source_x, 150.0 + 16.0 - height, 3.0, height]', heat)
        self.assertIn('[x - 4.0, y + 16.0 - height, 3.0, height]', heat)
        self.assertIn('let height = fire_height(fuel, quality) as f32;', fire)
        self.assertIn('if height == 0.0 { return; }', fire)
        self.assertIn('[176.0, 136.0 + 14.0 - height, 14.0, height]', fire)
        self.assertIn('[x, y + 14.0 - height, 14.0, height]', fire)
        height = function('fire_height')
        self.assertIn('if quality == 0 { return 0; }', height)
        self.assertIn('((14 * u64::from(fuel)) / u64::from(quality)).min(14) as u32', height)

    def test_panel_gauge_coordinates_and_frozen_oracle(self):
        part = (CONSUMER / 'part-draw.rs').read_text()
        smeltery = (CONSUMER / 'source-smeltery-draw.rs').read_text()
        for text in ('if has_heater { (150.0, 18.0, 36.0, 176.0) } else { (152.0, 14.0, 38.0, 194.0) }',
                     '[sx, 52.0, width, height], sprite.logical_size()',
                     '[ox + x, oy + 31.0, width, height]',
                     '[176.0, 0.0, 52.0, 52.0], gauge.logical_size()',
                     '[ox + 90.0, oy + 16.0, 52.0, 52.0]'):
            self.assertIn(text, part)
        self.assertIn('[176.0, 0.0, 80.0, 106.0], gauge.logical_size()', smeltery)
        self.assertIn('[ox + 8.0, oy + 16.0, 80.0, 106.0]', smeltery)
        native = ROOT / 'pluto-lib/tests/native/rendering'
        evidence = json.loads((native / 'crop-oracle-sources.json').read_text())
        self.assertEqual(evidence['reference_commit'], 'a10de5e02377d8ccb5dd5d67611562e8e95886e1')
        self.assertEqual(evidence['oracle_sha256'], hashlib.sha256((native / 'crop-oracle.rs').read_bytes()).hexdigest())


if __name__ == '__main__':
    unittest.main()
