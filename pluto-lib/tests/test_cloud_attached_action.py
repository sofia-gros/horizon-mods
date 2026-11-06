"""Source/inverse checks only. No Game execution or Rust typing is implied."""
import hashlib
import json
from pathlib import Path
import tomllib
import unittest
ROOT=Path(__file__).resolve().parents[2]
NATIVE=ROOT/'pluto-lib/mixins/pluto_native_creative'
TC=ROOT/'tconstruct/mixins/tconstruct'
DOC=ROOT/'pluto-lib/docs'
def sha(v):return hashlib.sha256(v).hexdigest()
def text(p):return p.read_text()

class AttachedAction(unittest.TestCase):
    def test_exact_four_caller_changes_reverse_to_original_published_source(self):
        spec=json.loads(text(DOC/'cloud-attached-action-inverse.json'))
        value=text(TC/'source-splashing.rs')
        self.assertEqual(sha(value.encode()),spec['after_sha256'])
        prefix,body=value.split(spec['scope'],1);body=spec['scope']+body
        self.assertEqual(len(spec['edits']),4)
        for old,new in reversed(spec['edits']):
            self.assertEqual(body.count(new),1);body=body.replace(new,old)
        self.assertEqual(sha((prefix+body).encode()),spec['before_sha256'])

    def test_exact_module_relocation_only(self):
        m=json.loads(text(DOC/'cloud-attached-action-inverse.json'))['manifest']
        current=text(TC/'horizon-mixin.toml')
        self.assertEqual(sha(current.encode()),m['after_sha256'])
        self.assertEqual(current.count(m['new']),1)
        self.assertEqual(sha(current.replace(m['new'],m['old']).encode()),m['before_sha256'])
        row=next(m for m in tomllib.loads(current)['module'] if m['id']=='source-fluid-cloud')
        self.assertEqual(row,{'id':'source-fluid-cloud','file':'crates/horizon-client/src/game.rs','name':'tconstruct_source_fluid_cloud','source':'source-fluid-cloud.rs','visibility':'pub'})

    def test_shared_original_lookup_and_commit_reverse_byte_exactly(self):
        spec=json.loads(text(DOC/'cloud-attached-action-inverse.json'))['shared']
        value=text(NATIVE/'cloud-insertion.rs')
        self.assertEqual(sha(value.encode()),spec['after_sha256'])
        for key in ['added_frame_block','added_error','added_tests']:
            self.assertEqual(value.count(spec[key]),1);value=value.replace(spec[key],'')
        self.assertEqual(sha(value.encode()),spec['before_sha256'])

    def test_frame_has_private_fields_and_live_native_lookups(self):
        value=text(NATIVE/'cloud-insertion.rs').split('/// Resolve only through')[0]
        fields=value.split('struct AttachedPlayerFrame {',1)[1].split('}',1)[0]
        self.assertNotIn('pub ',fields)
        for required in ['ready_server_epoch','game.native_level(game.dim.id())','game.native_level_height(level)',
            'game.command_guest(connection)','!guest.placed || !guest.loaded || guest.dead',
            'guest.dimension != game.dim.id()','guest.entity_id != motion.entity_id',
            'game.lan_acting_entity != Some(id)','game.lan_acting_uuid != Some(guest.uuid)',
            'game.native_entity(id)','game.native_entity_target(entity)','game.native_entity_level(entity)',
            'resolve_owner(game, level, Owner::Entity(entity))','attached_player_frame(game)? == *expected']:
            self.assertIn(required,value)
        for forbidden in ['unwrap_or(', 'unsafe ', 'as_guest(', 'enter_guest(', 'with_level(', 'take_entity_id(', 'Owner::Unowned, request']:
            self.assertNotIn(forbidden,value)

    def test_existing_insert_is_the_only_publication_path(self):
        value=text(NATIVE/'cloud-insertion.rs').split('#[cfg(test)]')[0]
        self.assertEqual(value.count('take_entity_id();'),1)
        self.assertEqual(value.count('breath_clouds.push(native);'),1)
        part=value.split('fn insert_attached_player_frame',1)[1].split('/// Resolve',1)[0]
        self.assertLess(part.index('require_attached_player_frame(game, frame)?'),part.index('insert(game, frame.level, Owner::Entity(frame.entity), request)'))

    def test_consumer_cannot_construct_native_cloud_or_infer_an_owner(self):
        value=text(TC/'source-fluid-cloud.rs').split('#[cfg(test)]')[0]
        for forbidden in ['PotionCloud {','Breath {','take_entity_id(', 'breath_clouds.push(', 'Attribution::Unowned', 'guest_connection: None', 'game.player', 'game.lan_acting', 'game.next_entity']:
            self.assertNotIn(forbidden,value)
        self.assertIn('request(at, effects, frame.attribution())?',value)
        self.assertIn('insert_attached_player_frame(game, frame, request)',value)

    def test_content_policy_is_explicit_and_bounded(self):
        value=text(TC/'source-fluid-cloud.rs').split('#[cfg(test)]')[0]
        for required in ['effects.is_empty() || effects.len() > pluto_cloud::MAX_EFFECTS','duration < 0','!(0..=255).contains(&amplifier)',
            'duration.checked_mul(4)','pluto_potions::effect(id)','radius: 1.0','radius_per_tick: -1.0 / 600.0',
            'radius_on_use: -0.5','duration_ticks: 600','wait_ticks: 10','reapplication_ticks: 20',
            'duration_on_use: 0','effect_duration_scale: 0.25','visible: true, ambient: false','entity.minecraft.area_effect_cloud']:
            self.assertIn(required,value)
        self.assertLess(value.index('duration.checked_mul(4)'),value.index('pluto_cloud::validate'))

    def test_pure_cloud_unsupported_returns_before_cooldown_or_world_effects(self):
        value=text(TC/'source-splashing.rs').split('pub(super) fn block(',1)[1]
        at=value.index('::capture(game)');bad=value.index('if !mixed_glow { return Some(Err(')
        cooldown=value.index('game.add_cooldown(')
        self.assertLess(at,bad);self.assertLess(bad,cooldown)
        self.assertIn('retaining glow-only attempt',value[bad:cooldown])
        self.assertIn('if mixed_glow && place_used > 0 && game.may_build()',value)

    def test_fluid_staging_and_success_accounting_follow_publication(self):
        value=text(TC/'source-splashing.rs').split('pub(super) fn block(',1)[1]
        stage=value.index('let cloud_stage =');spawn=value.index('::spawn(game, frame, at, scaled_cloud.clone())')
        success=value.index('cloud_spawned = true;');published=value.index('published_cloud = true;')
        self.assertLess(stage,spawn);self.assertLess(spawn,success);self.assertLess(success,published)
        self.assertIn('.map_err(|error| format!("TConstruct cloud publication failed: {error:?}"))?;',value[spawn:success])
        self.assertIn('let (used, updated) = if cloud_spawned { (cloud_used, cloud_stage) } else { (place_used, place_stage) };',value)
        self.assertIn('last_updated = if cloud_spawned { cloud_stage } else { place_stage };',value)

    def test_published_cloud_rechecks_actor_and_item_before_charge(self):
        value=text(TC/'source-splashing.rs').split('pub(super) fn block(',1)[1]
        final=value.split('if successes > 0 && !game.creative {',1)[1]
        self.assertLess(final.index('::require_current(game, frame)'),final.index('*game.held_mut() = last_updated'))
        self.assertLess(final.index('held.count == stack.count && held.damage == stack.damage && held.same_item_same_components(stack)'),final.index('*game.held_mut() = last_updated'))
        self.assertIn('already published; changed actor prevents item commit',final)
        self.assertIn('already published; changed held item prevents item commit',final)

    def test_metadata_keeps_old_guard_policy_and_adds_only_type_inputs(self):
        proof=json.loads(text(DOC/'cloud-attached-action-proof.json'))
        meta=json.loads(text(ROOT/'pluto-lib/tools/cloud-insertion-boundaries.json'))
        self.assertEqual(meta['horizon_revision'],proof['horizon_revision'])
        self.assertEqual(len(meta['official_source_sha256']),19)
        for path in ['crates/horizon-client/src/lan/guest.rs','crates/horizon-client/src/motion.rs']:
            self.assertEqual(meta['official_source_sha256'][path],proof['additional_native_inputs'][path]['sha256'])
        for key in ['exclusive_files','protected_target_prefixes','protected_state_files']:
            self.assertEqual(meta[key],proof['unchanged_guard_policy'][key])
        self.assertEqual(sha((ROOT/'pluto-lib/tools/check_cloud_insertion_plan.py').read_bytes()),proof['unchanged_guard_algorithm_sha256'])
        row=next(m for m in meta['modules'] if m['source_file']=='cloud-insertion.rs')
        self.assertEqual(row['source_sha256'],sha((NATIVE/'cloud-insertion.rs').read_bytes()))

    def test_scope_keeps_runtime_and_full_composition_unproved(self):
        proof=json.loads(text(DOC/'cloud-attached-action-proof.json'))
        self.assertIsNone(proof['actual_plan']);self.assertIsNone(proof['host_type_check'])
        self.assertFalse(proof['native_tests_executed']);self.assertFalse(proof['actual_game_action_executed'])
        self.assertEqual(proof['new_rust_tests_prepared'],{'shared_frame_values':2,'consumer_descriptor_policy':3})
        self.assertEqual(len(proof['remaining_original_guard_conflicts']),19)
        self.assertNotIn('source-fluid-cloud',[r['id'] for r in proof['remaining_original_guard_conflicts']])

if __name__=='__main__':unittest.main()
