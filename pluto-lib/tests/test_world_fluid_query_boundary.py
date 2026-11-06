"""Exact extraction/source guards; not native view execution proof."""
from pathlib import Path
import hashlib
import json
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
NATIVE = ROOT / 'pluto-lib/mixins/pluto_native_creative'
ORACLE = json.loads((ROOT / 'pluto-lib/tests/native/world-fluid-query-oracle.json').read_text())


def production():
    return (NATIVE / 'world-fluid-query.rs').read_text().split('#[cfg(test)]', 1)[0]


class WorldFluidQueryBoundary(unittest.TestCase):
    def test_decoder_is_exact_existing_logic_with_only_registry_parameter_changed(self):
        source = production()
        decoder = source[source.index('pub(super) fn cell('):source.index('/// All three references')]
        decoder = decoder.replace('pub(super) fn cell(registry: &BlockRegistry, state: BlockStateId) -> Option<(Arc<Definition>, Cell)> {',
                                  "fn cell(level: &GameLevel<'_>, state: BlockStateId) -> Option<(std::sync::Arc<Definition>, Cell)> {")
        decoder = decoder.replace('registry.state(state)?', 'level.registry.state(state)?')
        self.assertEqual(decoder, ORACLE['decoder'])

    def test_sample_preserves_exact_height_loaded_air_and_decoder_order(self):
        source = production()
        sample = source[source.index('    pub(super) fn sample('):source.index('    pub(super) fn query_point(')]
        sample = sample.replace('pub(super) fn sample', 'fn sample')
        sample = sample.replace('        if p[1]', '        let at = pos(p);\n        if p[1]', 1)
        for new, old in (
            ('self.world.shape.min_y', 'self.level.min_y()'),
            ('self.world.shape.max_y()', 'self.level.max_y()'),
            ('self.world.loaded(p[0], p[2])', 'self.level.is_loaded(at)'),
            ('self.world.block(p[0], p[1], p[2])', 'self.level.block_state(at)'),
            ('self.behaviours.is_air(state)', 'self.level.behaviours.is_air(state)'),
            ('cell(self.registry, state)', 'cell(self.level, state)'),
        ):
            sample = sample.replace(new, old)
        self.assertEqual(sample, ORACLE['adapter_sample'])

    def test_all_existing_mutation_and_bucket_policy_bytes_are_unchanged(self):
        source = (NATIVE / 'world-fluid.rs').read_text()
        self.assertEqual(source.count(ORACLE['new_wrappers']), 1)
        source = source.replace(ORACLE['new_wrappers'], ORACLE['decoder'])
        source = source.replace('use super::pluto_world_fluid_query::{cell, View};\n', '')
        source = source.replace('use pluto_api::fluid::{self, contact, Cell, Position, Sample, World};',
                                'use pluto_api::fluid::{self, Cell, Definition, Position, Sample, World};')
        self.assertEqual(source.count(ORACLE['new_adapter_sample']), 1)
        source = source.replace(ORACLE['new_adapter_sample'], ORACLE['adapter_sample'])
        source = source.replace('cell(self.level.registry, self.level.block_state(at))', 'cell(self.level, self.level.block_state(at))')
        source = source.replace('cell(level.registry, state)', 'cell(level, state)')
        source = source.replace('cell(level.registry, level.block_state(pos(hit.pos)))', 'cell(level, level.block_state(pos(hit.pos)))')
        self.assertEqual(hashlib.sha256(source.encode()).hexdigest(), ORACLE['baseline_sha256'])

    def test_read_view_and_entrypoints_have_no_write_or_actor_capability(self):
        source = production() + ORACLE['new_wrappers']
        for forbidden in ('&mut', 'Ticks', 'with_level(', 'flush_', 'set_block(', '.set(', '.schedule(',
                          'take_entity_id', 'lan_acting', 'FluidKind', 'tconstruct:', 'player.', 'mob.'):
            self.assertNotIn(forbidden, source)
        for required in ('contact::query_point_with(point, |p| self.sample(p))',
                         'contact::query_box_with(bounds, limits, |p| self.sample(p))',
                         'world: &\'a NativeWorld', 'registry: &\'a BlockRegistry', 'behaviours: &\'a Behaviours'):
            self.assertIn(required, source)

    def test_one_game_module_and_no_new_contact_hooks_or_consumers(self):
        manifest = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text())
        self.assertEqual([m for m in manifest['module'] if m['id'] == 'world-fluid-query'], [{
            'id':'world-fluid-query', 'file':'crates/horizon-client/src/game.rs',
            'name':'pluto_world_fluid_query', 'source':'world-fluid-query.rs',
        }])
        for section in ('injection', 'state', 'crate'):
            self.assertFalse(any(r['id'] == 'world-fluid-query' for r in manifest.get(section, [])))
        for path in (ROOT / 'tconstruct/mixins/tconstruct').glob('*.rs'):
            self.assertNotIn('pluto_world_fluid::query_', path.read_text(), path.name)


if __name__ == '__main__':
    unittest.main()
