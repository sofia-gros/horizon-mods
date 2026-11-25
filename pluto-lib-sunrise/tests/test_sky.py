"""Dynamic sky surface checks, not native rendering proof.

The bright end of a mod's sky cannot be a custom-sky texture: a texture
cannot twinkle, cannot darken what another layer drew, and cannot cover
the vanilla stars (every custom-sky layer draws before them). So the
surface is a pure registry of layers (stars, darkenings, a say over the
vanilla stars) whose frame quads the game draws with the sun and moon.
These checks pin that wiring so it cannot silently fall away: the pure
module and its exports, the support-matrix row, the access module in the
entry point, the celestial cancellable head reach (with no `require`:
cancellable head reaches carry none, E_SELECTOR), the helper's white-
texel vertices and star-frame drop, and the transport's sink.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / 'mod'
API = ROOT / 'mod' / 'api' / 'src'


class SkySurfaceTests(unittest.TestCase):
    def manifest(self):
        return tomllib.loads((BUNDLE / 'sunrise-mod.toml').read_text(encoding='utf-8'))

    def injection(self, wanted):
        return next(row for row in self.manifest()['injection'] if row['id'] == wanted)

    def module(self, wanted):
        return next(row for row in self.manifest()['module'] if row['id'] == wanted)

    def source(self, name):
        return (BUNDLE / name).read_text(encoding='utf-8')

    def test_the_pure_module_is_declared_exported_and_supported(self):
        pure = (API / 'pure' / 'sky_api_v1.rs').read_text(encoding='utf-8')
        self.assertIn('pub mod sky_api_v1;', (API / 'pure' / 'mod.rs').read_text(encoding='utf-8'))
        exports = (API / 'exports.rs').read_text(encoding='utf-8')
        self.assertIn('pub use crate::pure::sky_api_v1::{self, SkyDarkening, SkyFrame, SkyLayer, SkyQuad, SkyRegistry, SkyStar, Twinkle, VanillaStars, celestial_direction, frame_quads, twinkle_factor};', exports)
        support = (API / 'pure' / 'base' / 'support.rs').read_text(encoding='utf-8')
        self.assertIn('sky_api_v1::SkyRegistry::{register,layer,registered_worlds} / frame_quads', support)
        for name in ('pub struct Twinkle', 'pub struct SkyStar', 'pub struct SkyDarkening',
                     'pub enum VanillaStars', 'pub struct SkyLayer', 'pub struct SkyFrame',
                     'pub struct SkyQuad', 'pub struct SkyRegistry',
                     'pub fn celestial_direction', 'pub fn twinkle_factor', 'pub fn frame_quads'):
            self.assertIn(name, pure)

    def test_the_access_module_is_attached_to_the_entry_point(self):
        access = self.module('pluto-sky-access')
        self.assertEqual(access['file'], 'crates/horizon-client/src/entry.rs')
        self.assertEqual(access['name'], 'pluto_sky_access')
        self.assertEqual(access['source'], 'pluto-sky-access.rs')
        self.assertEqual(access['visibility'], 'pub')
        source = self.source('pluto-sky-access.rs')
        self.assertIn('pub fn set_sink', source)
        self.assertIn('pub struct SkyQuad', source)
        self.assertIn('pub struct Sink', source)

    def test_the_celestial_reach_is_a_cancellable_head_without_require(self):
        reach = self.injection('pluto-sky-celestial')
        self.assertEqual(reach['target'], "sunrise_client::game::frame_geometry::Game<'_>::celestial_geometry")
        self.assertEqual(reach['kind'], 'HEAD')
        self.assertTrue(reach['cancellable'])
        self.assertNotIn('require', reach)  # cancellable HEAD reaches carry no require (E_SELECTOR)
        self.assertEqual(reach['helper'], 'pluto-sky-celestial.rs')
        source = self.source('pluto-sky-celestial.rs')
        self.assertIn('pluto_sky_access::sink()', source)
        self.assertIn('white_uv', source)
        self.assertIn('sky.star_angle', source)
        # No layer for the frame: the game's own body runs untouched. A layer
        # answers: the helper returns the bodies, its quads joined and the
        # star frame dropped when the layer hides the vanilla stars.
        self.assertIn('ControlFlow::Continue(())', source)
        self.assertIn('ControlFlow::Break((out, stars))', source)
        self.assertIn('if hide {', source)
        self.assertIn('view_day_time', source)

    def test_the_transport_installs_the_sink_and_serves_the_overworld_layer(self):
        hooks = (API / 'hooks.rs').read_text(encoding='utf-8')
        self.assertIn('pluto_sky_access::set_sink', hooks)
        self.assertIn('fn sky_frame(', hooks)
        self.assertIn('SkyRegistry::layer(0)', hooks)
        self.assertIn('frame_quads(&layer, &frame)', hooks)
        self.assertIn('VanillaStars::Hide', hooks)

    def test_the_docs_page_is_listed_and_compiled(self):
        page = ROOT / 'docs' / 'api' / 'sky.md'
        self.assertTrue(page.exists())
        self.assertIn('[sky](sky.md)', (ROOT / 'docs' / 'api' / 'index.md').read_text(encoding='utf-8'))
        pure_lib = (ROOT / 'tests' / 'pure' / 'src' / 'lib.rs').read_text(encoding='utf-8')
        self.assertIn('page!(Sky, "sky.md");', pure_lib)
        self.assertIn('mod sky_access_tests;', pure_lib)


if __name__ == '__main__':
    unittest.main()
