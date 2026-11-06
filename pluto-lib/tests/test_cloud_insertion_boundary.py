"""Structural guards, not an execution proof of native actor lookup or insertion."""
from pathlib import Path
import hashlib
import json
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
NATIVE = ROOT / 'pluto-lib/mixins/pluto_native_creative'
CONTRACT = json.loads((ROOT / 'pluto-lib/tests/cloud-native-item-adoption.json').read_text())


class CloudInsertionBoundary(unittest.TestCase):
    def test_module_placement_has_no_new_hook_and_exact_consumer(self):
        manifest = tomllib.loads((NATIVE / 'horizon-mixin.toml').read_text())
        modules = {m['id']: m for m in manifest['module']}
        for identity, file, name in [
            ('lifecycle-readiness', 'game/native_lifecycle.rs', 'pluto_lifecycle_readiness'),
            ('cloud-insertion', 'game.rs', 'pluto_cloud_insertion'),
        ]:
            self.assertEqual(modules[identity], {'id': identity,
                'file': 'crates/horizon-client/src/' + file, 'name': name, 'source': identity + '.rs'})
        self.assertFalse(any('cloud' in h['id'] or 'lifecycle-readiness' in h['id']
                             for h in manifest.get('injection', [])))
        consumers = {p.name for p in (ROOT / 'tconstruct/mixins/tconstruct').glob('*.rs')
                     if 'pluto_cloud_insertion' in p.read_text()}
        self.assertEqual(consumers, {'source-fluid-cloud.rs'})
        # The sole new consumer reaches the private Game module, never the old
        # potions-child literal or a second allocation/owner implementation.
        consumer = (ROOT / 'tconstruct/mixins/tconstruct/source-fluid-cloud.rs').read_text()
        self.assertIn('pluto_cloud_insertion::insert_native_item_frame(game, frame, request)', consumer)
        for forbidden in ('PotionCloud {', 'Breath {', 'take_entity_id(', 'breath_clouds.push(', 'EntityRef::new', 'Attribution::Unowned'):
            self.assertNotIn(forbidden, consumer)

    def test_real_lifecycle_owner_is_observed_without_parallel_state(self):
        source = (NATIVE / 'lifecycle-readiness.rs').read_text().split('#[cfg(test)]')[0]
        for required in ('lifecycle.attempt.as_ref()?', 'attempt.session == OpenState::Ready',
                         'NativeAuthority::IntegratedServer | NativeAuthority::DedicatedServer',
                         'NativeWorldEpoch::new(attempt.epoch)'):
            self.assertIn(required, source)
        for forbidden in ('static ', 'Mutex', 'Atomic', '&mut', 'begin_open(', 'world_open_ready(', 'begin_close('):
            self.assertNotIn(forbidden, source)

    def test_official_lookups_and_explicit_owner_are_mandatory(self):
        source = (NATIVE / 'cloud-insertion.rs').read_text().split('#[cfg(test)]')[0]
        for required in ('game.native_entity_target(entity).ok_or(Error::ActorUnavailable)?',
            'game.native_entity_level(entity) != Some(level)',
            'game.command_guest(conn).ok_or(Error::ActorUnavailable)?',
            'game.lan_acting.ok_or(Error::ActorUnavailable)?',
            'motion.entity_id != guest.entity_id', 'game.lan_acting_uuid != Some(guest.uuid)',
            'game.lan_acting_entity != Some(entity.id())',
            'Owner::Entity(entity) = owner else { return Ok(Attribution::Unowned); }',
            '_ => Err(Error::UnsupportedActor)',
            'game.remote.is_some() || !game.native_authoritative()',
            'ready_server_epoch(&game.native_lifecycle)',
            'game.native_level(level.dimension()) != Some(level)',
            'game.native_level_height(level).is_none()',
            'level.dimension() != game.dim.id()', 'game.world.loaded(x, z)',
            'require_attribution(resolved, request.descriptor().attribution)?'):
            self.assertIn(required, source)
        for forbidden in ('unwrap_or(', 'source_entity(', 'implied_player(', 'as_guest(',
                          'with_level(', 'potion_owner(', 'pluto_equipment', 'unsafe '):
            self.assertNotIn(forbidden, source)

    def test_fallible_work_precedes_both_identical_native_publication_tails(self):
        source = (NATIVE / 'cloud-insertion.rs').read_text().split('#[cfg(test)]')[0]
        anchor = '    let [x, _, z] = request.descriptor().position.map(|n| n.floor() as i32);'
        tails = [anchor + part.split('\n}', 1)[0] for part in source.split(anchor)[1:]]
        self.assertEqual(len(tails), 2)
        self.assertEqual(tails[0], tails[1])
        body = tails[0]
        ordered = ['game.world.loaded(x, z)', 'require_entity_successor(game.next_entity)?',
            'pluto_cloud_factory::construct(request).into_native()',
            'game.breath_clouds.try_reserve(1).map_err(|_| Error::Capacity)?',
            'let id = game.take_entity_id();', 'native.id = id;',
            'game.breath_clouds.push(native);', 'Ok(EntityRef::new(epoch, id))']
        positions = [body.index(part) for part in ordered]
        self.assertEqual(positions, sorted(positions))
        after = body.split('let id = game.take_entity_id();', 1)[1]
        self.assertNotIn('?', after)
        self.assertNotIn('Err(', after)
        # The full inverse below fixes both allocation occurrences; a count
        # increase alone is not an admission for another publication path.
        new = CONTRACT['shared']['added_frame_block']
        before = new.split(anchor, 1)[0]
        self.assertIn('require_native_item_frame(game, frame)?;', before)
        self.assertIn('require_attribution(frame.attribution(), request.descriptor().attribution)?;', before)
        self.assertIn('last.checked_add(1)', source)
        self.assertIn('i32::try_from(next).is_ok()', source)

    def test_original_lookup_insert_and_manifest_reverse_exactly(self):
        source = (NATIVE / 'cloud-insertion.rs').read_text()
        spec = CONTRACT['shared']
        self.assertEqual(hashlib.sha256(source.encode()).hexdigest(), spec['after_sha256'])
        for key in ('added_frame_block', 'added_error', 'added_tests'):
            self.assertEqual(source.count(spec[key]), 1)
            source = source.replace(spec[key], '')
        self.assertEqual(hashlib.sha256(source.encode()).hexdigest(), spec['before_sha256'])
        source = (ROOT / 'tconstruct/mixins/tconstruct/horizon-mixin.toml').read_text()
        spec = CONTRACT['manifest']
        self.assertEqual(hashlib.sha256(source.encode()).hexdigest(), spec['after_sha256'])
        self.assertEqual(source.count(spec['new']), 1)
        self.assertEqual(hashlib.sha256(source.replace(spec['new'], spec['old']).encode()).hexdigest(), spec['before_sha256'])

    def test_budget_color_and_consumer_deltas_reverse_exactly(self):
        for filename, spec in [('cloud-descriptor.rs', CONTRACT['descriptor']),
                               ('potion-facts.rs', CONTRACT['color'])]:
            value = (NATIVE / filename).read_text()
            self.assertEqual(hashlib.sha256(value.encode()).hexdigest(), spec['after_sha256'])
            if 'edits' in spec:
                for old, new in reversed(spec['edits']):
                    self.assertEqual(value.count(new), 1)
                    value = value.replace(new, old)
            else:
                self.assertEqual(value.count(spec['addition']), 1)
                value = value.replace(spec['addition'], '')
            self.assertEqual(hashlib.sha256(value.encode()).hexdigest(), spec['before_sha256'])
        source = (ROOT / 'tconstruct/mixins/tconstruct/source-splashing.rs').read_text()
        spec = CONTRACT['caller']
        self.assertEqual(hashlib.sha256(source.encode()).hexdigest(), spec['after_sha256'])
        prefix, body = source.split(spec['scope'], 1)
        body = spec['scope'] + body
        for old, new in reversed(spec['edits']):
            self.assertEqual(body.count(new), 1)
            body = body.replace(new, old)
        self.assertEqual(hashlib.sha256((prefix + body).encode()).hexdigest(), spec['before_sha256'])

    def test_constructor_body_unchanged_and_accessor_only_consumes(self):
        source = (NATIVE / 'cloud-factory.rs').read_text()
        body = source.split('pub(in crate::game) fn construct', 1)[1].split('#[cfg(test)]', 1)[0]
        self.assertEqual(hashlib.sha256(body.encode()).hexdigest(),
            '343dcc0358eee3e4af2fa2325ac2a19ab4b699f0585e02a97a60825c09aef783')
        self.assertIn('pub(in crate::game) fn into_native(self) -> Breath { self.native }', source)
        self.assertNotIn('pub native:', source)

    def test_evidence_does_not_claim_unrun_native_rejections(self):
        evidence = json.loads((ROOT / 'pluto-lib/docs/cloud-insertion-proof.json').read_text())
        self.assertEqual(evidence['horizon_revision'],
                         '624c4f6265c02296fbe8a80f029dd0d987fb94c0')
        self.assertEqual(len(evidence['official_source_sha256']), 8)
        self.assertEqual(evidence['native_tests_prepared'],
                         {'lifecycle_real_state': 2, 'insertion_pure_rejections': 2})
        self.assertFalse(evidence['native_tests_executed'])


if __name__ == '__main__':
    unittest.main()
