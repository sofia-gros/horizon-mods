"""Generic native facts only; consumer policy must be byte-preserved."""
from pathlib import Path
import hashlib
import importlib.util
import json
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


def normalize(source):
    for prefix in ('horizon_client_core::potions::', 'horizon_client::brewing::', 'horizon_client::pluto_potions::'):
        for method in ('potion', 'effect'):
            source = source.replace(prefix + method + '(', '<' + method + '>(')
        source = source.replace(prefix + 'is_mod_effect(', '<custom>(').replace(prefix + 'has_custom_namespace(', '<custom>(')
    source = source.replace('known.canonical_id', 'horizon_client_core::potions::namespaced(known.id)')
    return source.replace('.raw_id', '.id').replace('.color_rgb', '.color').replace('.name_suffix', '.name')


def normalize_composition(source, row):
    """Reverse only named, exact pre-reviewed changes before the catalog guard."""
    for delta in row.get('permitted_composition_deltas', []):
        before, after = delta['before'], delta['after']
        counts = (source.count(before), source.count(after))
        if counts == (0, 1):
            source = source.replace(after, before, 1)
        elif counts != (1, 0):
            raise ValueError('Changed or duplicated allowed composition delta: ' + delta['name'])
    return normalize(source)


class PotionFactsBoundary(unittest.TestCase):
    def test_all_six_consumers_preserve_every_non_catalog_policy_byte(self):
        evidence = json.loads((ROOT / 'pluto-lib/tests/potion-catalog-consumer-sources.json').read_text())
        self.assertEqual(len(evidence['sources']), 6)
        for row in evidence['sources']:
            with self.subTest(path=row['path']):
                source = (ROOT / row['path']).read_text()
                self.assertEqual(hashlib.sha256(normalize_composition(source, row).encode()).hexdigest(), row['policy_sha256'])
                self.assertNotIn('horizon_client_core::potions::', source)
                self.assertNotIn('horizon_client::brewing::', source)
                self.assertIn('horizon_client::pluto_potions::', source)

    def test_composed_codec_delta_is_exact_and_surrounding_policy_stays_guarded(self):
        evidence = json.loads((ROOT / 'pluto-lib/tests/potion-catalog-consumer-sources.json').read_text())
        row = next(row for row in evidence['sources'] if row['path'].endswith('/source-smeltery-fuel.rs'))
        delta, = row['permitted_composition_deltas']
        self.assertEqual(delta['name'], 'shared_recursive_lossless_item_codec')
        current = (ROOT / row['path']).read_text()
        # Test both branches without requiring an integrated source checkout.
        before = current.replace(delta['after'], delta['before'])
        after = before.replace(delta['before'], delta['after'])
        self.assertNotEqual(before, after)
        for source in (before, after):
            self.assertEqual(hashlib.sha256(normalize_composition(source, row).encode()).hexdigest(), row['policy_sha256'])
        for changed in (after.replace('encode_item_lossless(item).is_some()', 'encode_item_lossless(item).is_some() || true'),
                        after.replace('encode_item_lossless(item)', 'encode_item_lossless(&other)'),
                        after + '\n' + delta['after']):
            with self.assertRaises(ValueError):
                normalize_composition(changed, row)
        outside = after.replace('Some(4000)', 'Some(4001)', 1)
        self.assertNotEqual(hashlib.sha256(normalize_composition(outside, row).encode()).hexdigest(), row['policy_sha256'])

    def test_single_generic_owner_and_public_client_library_module(self):
        root = ROOT / 'pluto-lib/mixins/pluto_native_creative'
        source = (root / 'potion-facts.rs').read_text()
        production = source.split('#[cfg(test)]', 1)[0]
        for forbidden in ('tconstruct', 'register_mod_effects', 'EffectInstance', 'EffectAction', 'Game<', 'effects_color', 'Tag::', 'pluto_api::'):
            self.assertNotIn(forbidden, production)
        manifest = tomllib.loads((root / 'horizon-mixin.toml').read_text())
        module = [m for m in manifest['module'] if m['source'] == 'potion-facts.rs']
        self.assertEqual(module, [{'id': 'potion-facts', 'file': 'crates/horizon-client/src/client_lib.rs', 'name': 'pluto_potions', 'source': 'potion-facts.rs', 'visibility': 'pub'}])

    def test_caps_math_source_effect_rules_and_application_remain_local(self):
        root = ROOT / 'tconstruct/mixins/tconstruct'
        color = (root / 'source-melter-render.rs').read_text()
        for token in ('effects.len() <= 64', '0.0f32', 'amplifier.max(0) + 1', 'ShowParticles', 'color_tagged(fluid, tag)'):
            self.assertIn(token, color)
        self.assertIn('EffectInstance::new(id, duration, amplifier)', (root / 'source-slurping.rs').read_text())
        self.assertIn('mob.add_effect(id, duration, amplifier)', (root / 'source-spilling.rs').read_text())
        self.assertIn('horizon_client_core::potions::effects_color', (root / 'source-fluid-cloud.rs').read_text())

    def test_catalog_fixture_uses_only_its_exact_local_package_exception(self):
        path = ROOT / 'pluto-lib/tools/prepare_render_fixture.py'
        spec = importlib.util.spec_from_file_location('render_guards', path)
        guards = importlib.util.module_from_spec(spec); spec.loader.exec_module(guards)
        host = {'name': 'serde_json', 'version': '1.0.151', 'source': 'registry', 'checksum': 'abc'}
        fixture = {'name': 'pluto-potion-facts', 'version': '0.0.0'}
        identity = ('pluto-potion-facts', '0.0.0', None, None)
        guards.check_identities({'package': [host]}, {'package': [host, fixture]}, identity)
        for changed in (host | {'checksum': 'bad'}, {'name': 'pluto-render-equivalence', 'version': '0.0.0'}):
            with self.assertRaises(ValueError):
                guards.check_identities({'package': [host]}, {'package': [changed, fixture]}, identity)


if __name__ == '__main__':
    unittest.main()
