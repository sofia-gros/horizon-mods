"""Remaining machine panel UV arithmetic is shared, not its layout policy."""
from pathlib import Path
import hashlib
import json
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONSUMER = ROOT / 'tconstruct/mixins/tconstruct'


class RenderPanelConsumers(unittest.TestCase):
    def test_panel_keeps_input_policy_and_has_no_raw_uv_access(self):
        source = (CONSUMER / 'source-smeltery-draw.rs').read_text()
        self.assertNotIn('.u0', source)
        self.assertNotIn('.v0', source)
        self.assertNotIn('.u1', source)
        self.assertNotIn('.v1', source)
        self.assertNotIn('atlas.sprites.get', source)
        for choice in ('game.container_kind().is_some_and(|entity| entity.slots().first().is_none_or(Option::is_none))',
                       '(ox + 124.0..ox + 142.0).contains(&game.cursor.0) && (oy + 69.0..oy + 87.0).contains(&game.cursor.1)',
                       'if cols > 0', 'if max > 0', 'offset.min(max)', 'if first+i < count { 0.0 } else { 22.0 }'):
            self.assertIn(choice, source)
        self.assertIn('[224.0, 186.0, 16.0, 16.0]', source)
        self.assertIn('[176.0, 202.0, 18.0, 18.0]', source)
        self.assertIn('[176.0 + f32::from(mode) * 16.0, 186.0, 16.0, 16.0]', source)
        self.assertIn('[sx, sy, sw, sh], sprite.logical_size()', source)
        self.assertIn('238.0, 22.0, 18.0]', source)

    def test_frozen_panel_oracle_and_unchanged_layout(self):
        native = ROOT / 'pluto-lib/tests/native/rendering'
        evidence = json.loads((native / 'panel-oracle-sources.json').read_text())
        self.assertEqual(evidence['reference_commit'], '8443c8c857131877e62d458948235a113d38e756')
        self.assertEqual(evidence['oracle_sha256'], hashlib.sha256((native / 'panel-oracle.rs').read_bytes()).hexdigest())
        self.assertEqual(evidence['unchanged_layout_sha256'], hashlib.sha256((CONSUMER / 'source-smeltery-layout.rs').read_bytes()).hexdigest())
        self.assertNotIn('struct Rect', (native / 'panel-oracle.rs').read_text())
        self.assertNotIn('struct HudBatch', (native / 'panel-oracle.rs').read_text())


if __name__ == '__main__':
    unittest.main()
