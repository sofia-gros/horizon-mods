"""Textured sky surface checks, not native rendering proof.

The faint end of a mod's sky is a texture, and the host draws textures of
that kind only from a resource pack (its custom-sky scanner reads one
namespace under one folder of a pack root). So the surface is a pure
registry of textured layers — a skybox PNG with the pack format's blend,
fade, turning and weather — whose textures and per-frame draws join the
pack's own custom sky in the game's one sky object. These checks pin that
wiring so it cannot silently fall away: the pure module and its exports,
the support-matrix row, the access module inside the renderer's custom-sky
module, the load reach (a cancellable head with no `require`: the load's
own empty answer is built in a closure a return reach cannot re-emit), the
composite's index discipline, and the transport's sink.
"""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / 'mod'
API = ROOT / 'mod' / 'api' / 'src'


class SkyTextureSurfaceTests(unittest.TestCase):
    def manifest(self):
        return tomllib.loads((BUNDLE / 'horizon-mixin.toml').read_text(encoding='utf-8'))

    def injection(self, wanted):
        return next(row for row in self.manifest()['injection'] if row['id'] == wanted)

    def module(self, wanted):
        return next(row for row in self.manifest()['module'] if row['id'] == wanted)

    def source(self, name):
        return (BUNDLE / name).read_text(encoding='utf-8')

    def test_the_pure_module_declares_the_textured_surface(self):
        pure = (API / 'pure' / 'sky_api_v1.rs').read_text(encoding='utf-8')
        exports = (API / 'exports.rs').read_text(encoding='utf-8')
        self.assertIn(
            'SkyBlend, SkyDarkening, SkyFade, SkyFrame, SkyLayer, SkyNow, SkyQuad, SkyRegistry, '
            'SkyStar, SkyTextureConditions, SkyTextureDraw, SkyTextureLayer, SkyTextureRegistry, '
            'SkyWeather, Twinkle, VanillaStars, celestial_direction, fade_ticks, frame_quads, '
            'png_size, texture_draws, twinkle_factor',
            exports,
        )
        support = (API / 'pure' / 'base' / 'support.rs').read_text(encoding='utf-8')
        self.assertIn('sky_api_v1::SkyTextureRegistry::{register,layers,registered_worlds} / texture_draws', support)
        for name in ('pub enum SkyBlend', 'pub enum SkyNow', 'pub struct SkyWeather',
                     'pub struct SkyFade', 'pub fn fade_ticks', 'pub struct SkyTextureLayer',
                     'pub struct SkyTextureRegistry', 'pub fn png_size',
                     'pub struct SkyTextureConditions', 'pub struct SkyTextureDraw',
                     'pub fn texture_draws'):
            self.assertIn(name, pure)
        # The blend declaration order is the transport's blend number.
        blend = pure.split('pub enum SkyBlend', 1)[1].split('\n}', 1)[0]
        names = ['Add', 'Subtract', 'Multiply', 'Dodge', 'Burn', 'Screen',
                 'Replace', 'Overlay', 'Alpha']
        positions = [blend.index(name) for name in names]
        self.assertEqual(positions, sorted(positions))

    def test_the_access_module_lives_in_the_renderers_custom_sky(self):
        access = self.module('pluto-sky-texture-access')
        self.assertEqual(access['file'], 'crates/horizon-client-render/src/custom_sky.rs')
        self.assertEqual(access['name'], 'pluto_sky_texture_access')
        self.assertEqual(access['source'], 'pluto-sky-texture-access.rs')
        self.assertEqual(access['visibility'], 'pub')
        source = self.source('pluto-sky-texture-access.rs')
        self.assertIn('pub fn set_sink', source)
        self.assertIn('pub struct TexImage', source)
        self.assertIn('pub struct TexDraw', source)
        self.assertIn('pub struct Sink', source)
        self.assertIn('pub fn wrap', source)
        # The composite: pack images first (their indices do not move), the
        # mods' draws appended with their indices shifted past the pack's,
        # rotation built from axis and turns as the pack format builds it.
        self.assertIn('pack_images as u16 + draw.image', source)
        self.assertIn('Mat4::from_axis_angle', source)
        self.assertIn('impl super::SkyLayers for Composite', source)

    def test_the_load_reach_is_a_cancellable_head_without_require(self):
        reach = self.injection('pluto-sky-texture-load')
        self.assertEqual(reach['target'], 'horizon_client_render::custom_sky::load')
        self.assertEqual(reach['kind'], 'HEAD')
        self.assertTrue(reach['cancellable'])
        self.assertNotIn('require', reach)  # cancellable HEAD reaches carry no require (E_SELECTOR)
        self.assertEqual(reach['helper'], 'pluto-sky-texture-load.rs')
        self.assertEqual(reach['bindings'], [{'name': 'root', 'ty': '&Path', 'value': 'root'}])
        source = self.source('pluto-sky-texture-load.rs')
        self.assertIn('pluto_sky_texture_access::sink()', source)
        # No sink or no layer: the game's own load runs untouched. A layer
        # answers: the pack formats' own layers, wrapped by the composite.
        self.assertIn('ControlFlow::Continue(())', source)
        self.assertIn('crate::pack_hooks::formats().custom_sky(root)', source)
        self.assertIn('ControlFlow::Break(crate::custom_sky::pluto_sky_texture_access::wrap(pack, *sink))', source)
        # A helper file holds exactly one function.
        self.assertEqual(source.count('\nfn '), 1)
        self.assertNotIn('///', source)

    def test_the_transport_installs_the_sink_and_prepares_the_layers(self):
        hooks = (API / 'hooks.rs').read_text(encoding='utf-8')
        self.assertIn('pluto_sky_texture_access::set_sink', hooks)
        self.assertIn('fn sky_texture_any(', hooks)
        self.assertIn('fn sky_texture_images(', hooks)
        self.assertIn('fn sky_texture_has(', hooks)
        self.assertIn('fn sky_texture_draws(', hooks)
        # Decoded by the host's own image decoder; a layer it refuses drops
        # out of the prepared set, so images and draws never disagree.
        self.assertIn('horizon_client::hud::decode_image_bytes(&layer.png)', hooks)
        self.assertIn('texture_generation()', hooks)
        self.assertIn('texture_draws(layers, first_image, &conditions, sun_turns, seconds, snap, eased)', hooks)


if __name__ == '__main__':
    unittest.main()
