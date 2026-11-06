"""Structural guards, not an execution proof of native actor lookup or insertion."""
from pathlib import Path
import hashlib
import json
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
NATIVE = ROOT / 'pluto-lib/mixins/pluto_native_creative'


class CloudInsertionBoundary(unittest.TestCase):
    def test_module_placement_has_no_new_hook_and_only_reviewed_consumer(self):
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
        for path in (ROOT / 'tconstruct/mixins/tconstruct').glob('*.rs'):
            if path.name != 'source-fluid-cloud.rs':
                self.assertNotIn('pluto_cloud_insertion', path.read_text(), path.name)
        consumer = (ROOT / 'tconstruct/mixins/tconstruct/source-fluid-cloud.rs').read_text()
        self.assertIn('pluto_cloud_insertion::insert_attached_player_frame', consumer)
        for forbidden in ('take_entity_id(', 'breath_clouds.push(', 'PotionCloud {', 'Breath {'):
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

    def test_fallible_work_precedes_native_id_and_push(self):
        source = (NATIVE / 'cloud-insertion.rs').read_text().split('#[cfg(test)]')[0]
        body = source.split('pub(in crate::game) fn insert(game:', 1)[1]
        ordered = ['require_entity_successor(game.next_entity)?',
            'pluto_cloud_factory::construct(request).into_native()',
            'game.breath_clouds.try_reserve(1).map_err(|_| Error::Capacity)?',
            'let id = game.take_entity_id();', 'native.id = id;',
            'game.breath_clouds.push(native);', 'Ok(EntityRef::new(epoch, id))']
        positions = [body.index(part) for part in ordered]
        self.assertEqual(positions, sorted(positions))
        after = body.split('let id = game.take_entity_id();', 1)[1]
        self.assertNotIn('?', after)
        self.assertNotIn('Err(', after)
        self.assertEqual(source.count('take_entity_id();'), 1)
        self.assertIn('last.checked_add(1)', source)
        self.assertIn('i32::try_from(next).is_ok()', source)

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
