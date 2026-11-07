"""Side inventory/info crop extraction must preserve all layout and text policy."""
from pathlib import Path
import hashlib
import json
import unittest

ROOT = Path(__file__).resolve().parents[2]


def normalize_policy(source):
    for key in ('gui/tconstruct_generic', 'gui/tconstruct_panel'):
        source = source.replace('atlas.sprites.get("' + key + '").copied()', 'lookup("' + key + '")')
        source = source.replace('horizon_client::pluto_render::sprite(atlas, "' + key + '")', 'lookup("' + key + '")')
    start = source.index('let region = |')
    opening = source.index('{', start)
    end = opening + 1
    level = 1
    while level:
        if source[end] == '{': level += 1
        if source[end] == '}': level -= 1
        end += 1
    return source[:start] + '<shared-crop>' + source[end:]


class RenderSideConsumers(unittest.TestCase):
    def test_non_crop_text_and_layout_are_byte_preserved(self):
        native = ROOT / 'pluto-lib/tests/native/rendering'
        evidence = json.loads((native / 'side-oracle-sources.json').read_text())
        for row in evidence['sources']:
            with self.subTest(path=row['path']):
                source = (ROOT / row['path']).read_text()
                self.assertEqual(hashlib.sha256(normalize_policy(source).encode()).hexdigest(), row['non_crop_policy_sha256'])
                self.assertIn('horizon_client::pluto_render::crop(batch, sprite,', source)
                for member in ('.u0', '.u1', '.v0', '.v1', 'atlas.sprites.get'):
                    self.assertNotIn(member, source)
        self.assertEqual(evidence['oracle_sha256'], hashlib.sha256((native / 'side-oracle.rs').read_bytes()).hexdigest())

    def test_layout_bounds_and_actual_callers_are_unchanged(self):
        native = ROOT / 'pluto-lib/tests/native/rendering'
        evidence = json.loads((native / 'side-oracle-sources.json').read_text())
        layout = ROOT / 'tconstruct/mixins/tconstruct/source-side-layout.rs'
        self.assertEqual(evidence['layout_sha256'], hashlib.sha256(layout.read_bytes()).hexdigest())
        builder = (ROOT / 'tconstruct/mixins/tconstruct/builder-info-ui.rs').read_text()
        station = (ROOT / 'tconstruct/mixins/tconstruct/station-info-ui.rs').read_text()
        self.assertIn('panel(game, batch, ox + 180.0, oy, width, 184.0)', builder)
        self.assertIn('panel(game, batch, ox + 182.0, oy + offset, width, 83.0)', station)
        self.assertIn('.clamp(80.0, 126.0)', layout.read_text())


if __name__ == '__main__':
    unittest.main()
