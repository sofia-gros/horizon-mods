"""Source/package-boundary assertions; native equivalence is a separate fixture."""
from pathlib import Path
import hashlib
import json
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
NATIVE = ROOT / "pluto-lib/mixins/pluto_native_creative"
CONSUMER = ROOT / "tconstruct/mixins/tconstruct"


class RenderingBoundary(unittest.TestCase):
    def test_one_public_module_without_new_hooks(self):
        m = tomllib.loads((NATIVE / "horizon-mixin.toml").read_text())
        self.assertEqual([r for r in m['module'] if r['id'] == 'rendering'], [{
            'id': 'rendering', 'file': 'crates/horizon-client/src/client_lib.rs',
            'name': 'pluto_render', 'source': 'rendering.rs', 'visibility': 'pub',
        }])
        for key in ('injection', 'state', 'crate'):
            self.assertFalse(any(r['id'] == 'rendering' for r in m.get(key, [])))

    def test_shared_source_has_no_content_or_reverse_dependency(self):
        s = (NATIVE / 'rendering.rs').read_text()
        for forbidden in ('tconstruct', 'crate::game', 'pluto_api', 'minecraft:', 'set_block'):
            self.assertNotIn(forbidden, s)
        self.assertIn('block_entity_render::vertex_light(packed)', s)
        self.assertIn('quad_budget.min(MAX_TILE_QUADS)', s)
        self.assertIn('out.extend(prepared)', s)

    def test_five_consumers_delegate_mechanics_only(self):
        for name in ('part-draw.rs','source-smeltery-draw.rs','render-copper-can.rs',
                     'source-machine-geometry.rs','source-melter-render.rs'):
            self.assertIn('horizon_client::pluto_render::', (CONSUMER / name).read_text())
        mesh = (CONSUMER / 'source-machine-geometry.rs').read_text()
        self.assertNotIn('out.push(Vertex', mesh)
        self.assertIn('(at - camera).as_vec3()', mesh)
        self.assertIn('tank.amount as f32 / tank.capacity as f32', mesh)
        self.assertIn('color_with_complete', mesh)
        self.assertIn('64.0 * 64.0', mesh)
        for name in ('part-draw.rs', 'source-smeltery-draw.rs'):
            s = (CONSUMER / name).read_text()
            self.assertEqual(s.count('pluto_render::tiled_bottom_up'), 2)
            self.assertIn('block/tconstruct_fluid_fallback', s)
        mask = (CONSUMER / 'render-copper-can.rs').read_text()
        self.assertIn('[(6., 3., 4.), (5., 4., 6.), (5., 5., 6.), (6., 6., 4.)]', mask)
        self.assertIn('[left, top, width, 1.0], [16.0, 16.0]', mask)

    def test_frozen_oracle_hash_and_actual_native_types(self):
        native = ROOT / 'pluto-lib/tests/native/rendering'
        e = json.loads((native / 'oracle-sources.json').read_text())
        self.assertEqual(hashlib.sha256((native / 'oracle.rs').read_bytes()).hexdigest(), e['oracle_sha256'])
        self.assertEqual(e['reference_commit'], '1e870214749b65b3f2c226761e2947a13b29d35f')
        fixture = (ROOT / 'pluto-lib/tools/prepare_render_fixture.py').read_text()
        self.assertIn('package = "horizon-client-render"', fixture)
        for name in ('tests.rs', 'oracle.rs'):
            text = (native / name).read_text()
            for mocked in ('struct Vertex', 'struct HudBatch', 'struct Rect'):
                self.assertNotIn(mocked, text)


if __name__ == '__main__':
    unittest.main()
