"""UNAPPLIED source/inverse assertions, not Rust execution or actor authority."""
from pathlib import Path
import hashlib,json,re,unittest
R=Path(__file__).resolve().parent
S=json.loads((R/'inputs.json').read_text(encoding='utf-8'))
def read(name):return (R/name).read_text(encoding='utf-8')
def sha(value):return hashlib.sha256(value.encode()).hexdigest()

class NativeItemSource(unittest.TestCase):
    def test_shared_original_resolver_and_insert_reverse_byte_exactly(self):
        value=read('cloud-insertion.rs');spec=S['shared']
        self.assertEqual(sha(value),spec['after_sha256'])
        for key in ['added_frame_block','added_error','added_tests']:
            self.assertEqual(value.count(spec[key]),1);value=value.replace(spec[key],'')
        self.assertEqual(sha(value),spec['before_sha256'])
    def test_caller_four_edits_reverse_to_exact_original(self):
        value=read('source-splashing.rs');spec=S['caller']
        self.assertEqual(sha(value),spec['after_sha256'])
        prefix,body=value.split(spec['scope'],1);body=spec['scope']+body
        self.assertEqual(len(spec['edits']),4)
        for old,new in reversed(spec['edits']):
            self.assertEqual(body.count(new),1);body=body.replace(new,old)
        self.assertEqual(sha(prefix+body),spec['before_sha256'])
    def test_guest_and_no_guest_are_both_player_attribution(self):
        v=S['shared']['added_frame_block']
        actor=v.split('impl NativeItemActor {',1)[1].split('#[derive',1)[0]
        self.assertIn('Attribution::Player',actor)
        self.assertIn('Self::Guest { connection, .. } => Some(connection)',actor)
        self.assertIn('Self::PlayerWithoutGuest { .. } => None',actor)
        self.assertNotIn('Attribution::Unowned',v)
        self.assertNotIn('EntityRef::new',v.split('fn insert_native_item_frame',1)[0])
    def test_classifier_rejects_partial_guest_and_other_phase(self):
        v=S['shared']['added_frame_block']
        for x in ['if phase > 2','if phase != 0','entity_id == body as u32',
                  '(None, None, None)','_ => Err(Error::ActorUnavailable)']:
            self.assertIn(x,v)
        self.assertIn('for mask in 1..7',S['shared']['added_tests'])
    def test_live_frame_checks_ready_level_body_actor_hand_and_slot(self):
        v=S['shared']['added_frame_block'].split('fn native_item_frame',1)[1].split('/// Same-invocation',1)[0]
        for x in ['game.remote.is_some() || !game.native_authoritative()', 'ready_server_epoch',
                  'game.native_level(game.dim.id())','level.epoch() != epoch','game.native_level_height(level)',
                  'game.sculk_actor()','Who::Player','game.player.as_ref()', 'game.lan_acting_uuid',
                  'game.lan_acting_entity','game.guest_use','phase != hand','game.held_slot()','game.creative']:
            self.assertIn(x,v)
        for x in ['command_guest(','.loaded','.dead','client_loaded','ensure_player_uuid',
                  'native_entity(', 'Owner::Unowned','unsafe ', 'as_guest(', 'native_use_as_player(']:
            self.assertNotIn(x,v)
    def test_observation_does_not_claim_a_capability_or_aba_lease(self):
        v=S['shared']['added_frame_block']
        self.assertIn('not an authority capability',v)
        self.assertIn('not an ABA-proof lease',v)
        fields=v.split('struct NativeItemFrame {',1)[1].split('}',1)[0]
        self.assertNotIn('pub ',fields)
        self.assertIn('native_item_frame(game)? == *expected',v)
    def test_publication_tail_matches_original_exactly(self):
        v=read('cloud-insertion.rs');anchor='    let [x, _, z] = request.descriptor().position.map(|n| n.floor() as i32);'
        tails=[]
        for block in v.split(anchor)[1:]:tails.append(anchor+block.split('\n}',1)[0])
        self.assertEqual(len(tails),2);self.assertEqual(tails[0],tails[1])
        tail=tails[0]
        positions=[tail.index(x) for x in ['game.world.loaded','require_entity_successor',
            'pluto_cloud_factory::construct','try_reserve(1)','take_entity_id()','native.id = id','breath_clouds.push(native)']]
        self.assertEqual(positions,sorted(positions))
        self.assertNotIn('?',tail.split('take_entity_id()',1)[1])
    def test_requested_attribution_checked_before_creation(self):
        v=S['shared']['added_frame_block'].split('fn insert_native_item_frame',1)[1]
        ordered=['require_native_item_frame(game, frame)?','require_attribution(frame.attribution(), request.descriptor().attribution)?','pluto_cloud_factory::construct']
        self.assertEqual([v.index(x) for x in ordered],sorted(v.index(x) for x in ordered))
    def test_consumer_has_no_direct_native_construction_or_owner_fallback(self):
        v=read('source-fluid-cloud.rs')
        self.assertEqual(sha(v),S['consumer']['after_sha256'])
        v=v.split('#[cfg(test)]')[0]
        for x in ['Breath {','PotionCloud {','take_entity_id(', 'breath_clouds.push(', 'game.lan_acting',
                  'Attribution::Unowned','Owner::Entity','EntityRef::new']:
            self.assertNotIn(x,v)
        self.assertIn('request(at, effects, frame.attribution())?',v)
        self.assertIn('insert_native_item_frame(game, frame, request)',v)
    def test_existing_policy_request_is_not_rewritten(self):
        v=read('source-fluid-cloud.rs')
        for x in ['duration.checked_mul(4)','let effect_budget = effects.len()','(0..=255)',
                  'radius: 1.0','radius_per_tick: -1.0 / 600.0','radius_on_use: -0.5',
                  'duration_ticks: 600','wait_ticks: 10','reapplication_ticks: 20',
                  'effect_duration_scale: 0.25','entity.minecraft.area_effect_cloud']:
            self.assertIn(x,v)
    def test_cloud_success_flag_follows_ok_publication_and_mixed_glow(self):
        v=read('source-splashing.rs').split('pub(super) fn block(',1)[1]
        start=v.index('::spawn(game, frame, at, scaled_cloud.clone())')
        end=v.index('// FluidEffects uses',start)
        block=v[start:end]
        self.assertLess(block.index('.map_err('),block.index('cloud_spawned = true'))
        self.assertLess(block.index('cloud_spawned = true'),block.index('published_cloud = true'))
        self.assertIn('if mixed_glow && place_used > 0 && game.may_build()',v[end:])
        self.assertIn('return Ok(cloud_spawned || placed)',v[end:])
        self.assertIn('if cloud_spawned { (cloud_used, cloud_stage) } else { (place_used, place_stage) }',v)
    def test_changed_frame_and_held_stack_reject_before_charge_even_if_creative(self):
        v=read('source-splashing.rs').split('pub(super) fn block(',1)[1]
        check=v.index('        if published_cloud {')
        charge=v.index('        if successes > 0 && !game.creative {',check)
        body=v[check:charge]
        self.assertIn('require_current(game, frame)',body)
        self.assertIn('held.count == stack.count && held.damage == stack.damage && held.same_item_same_components(stack)',body)
        self.assertIn('item commit',body)
    def test_unsupported_initial_context_rejects_whole_action_before_cooldown(self):
        v=read('source-splashing.rs').split('pub(super) fn block(',1)[1]
        capture=v.index('::capture(game)');cooldown=v.index('game.add_cooldown')
        self.assertLess(capture,cooldown)
        self.assertIn('Err(error) => return Some(Err(',v[capture:cooldown])
        self.assertNotIn('mixed_glow',v[capture:cooldown])
        self.assertNotIn('glow-only',v[capture:cooldown])
    def test_inventory_of_prepared_tests_and_no_runtime_manifest(self):
        names=re.findall(r'#\[test\]\s+fn (\w+)',read('cloud-insertion.rs'))
        self.assertEqual(len(names),6);self.assertEqual(len(set(names)),6)
        self.assertEqual(len(re.findall(r'#\[test\]\s+fn (\w+)',read('source-fluid-cloud.rs'))),3)
        self.assertFalse((R/'horizon-mixin.toml').exists())
        self.assertIsNone(S['actual_plan_identity']);self.assertIsNone(S['actual_type_result'])

    def test_default_budget_validation_body_reverses_to_original(self):
        v=read('cloud-descriptor.rs');spec=S['descriptor']
        self.assertEqual(sha(v),spec['after_sha256'])
        for old,new in reversed(spec['edits']):
            self.assertEqual(v.count(new),1);v=v.replace(new,old)
        self.assertEqual(sha(v),spec['before_sha256'])
        self.assertIn('pub const MAX_EFFECTS: usize = 256;',v)
        self.assertIn('validate_with_effect_budget(descriptor, MAX_EFFECTS)',read('cloud-descriptor.rs'))
        method=read('cloud-descriptor.rs').split('pub fn with_native_effect_color',1)[1].split('\n}',1)[0]
        self.assertIn('(mut self) -> Self',method)
        self.assertIn('.filter(|effect| effect.visible).map(|effect| (effect.raw_id, effect.amplifier))',method)
        self.assertEqual(method.count('pluto_potions::effects_color('),1)
        self.assertNotIn('color: u32',method)


    def test_explicit_budget_is_typed_and_overflow_checked_not_a_flag(self):
        v=read('cloud-descriptor.rs').split('pub fn validate_with_effect_budget',1)[1]
        self.assertIn('effect_budget: usize',v)
        self.assertIn('effect_budget.checked_mul(element_bytes)',v)
        self.assertIn('bytes > isize::MAX as usize',v)
        self.assertIn('std::mem::size_of::<Effect>()',v)
        self.assertIn('std::mem::size_of::<ResolvedEffect>()',v)
        self.assertLess(v.index('checked_mul'),v.index('pluto_potions::effect('))
        self.assertIn('descriptor.effects.len() > effect_budget',v)

    def test_color_adapter_reverses_exactly_and_delegates_native_once(self):
        v=read('potion-facts.rs');spec=S['color']
        self.assertEqual(sha(v),spec['after_sha256'])
        self.assertEqual(v.count(spec['addition']),1)
        self.assertEqual(sha(v.replace(spec['addition'],'')),spec['before_sha256'])
        self.assertEqual(spec['addition'].count('potions::effects_color(effects)'),1)
        self.assertNotIn('sort',spec['addition']);self.assertNotIn('dedup',spec['addition'])

    def test_large_owned_sequence_retains_order_and_checks_tail_flags(self):
        v=read('source-fluid-cloud.rs')
        body=v.split('#[cfg(test)]')[0]
        for x in ['let effect_budget = effects.len()', 'effects.iter().enumerate()',
                  'color_rgb: 0', '}, effect_budget).map(|value| value.with_native_effect_color()).map_err(Error::Invalid)']:
            self.assertIn(x,body)
        for x in ['MAX_EFFECTS','red +=','green +=','blue +=','.sort(','.dedup(']:self.assertNotIn(x,body)
        for x in ['for budget in [0,256,usize::MAX]','conflicting.effects[256].ambient=true',
                  'ConflictingFlags { index:256 }','invalid.effects[256].amplifier=256',
                  'validate(value.descriptor().clone())','take(512)','raw_color.color_rgb=0xffffefff','visible.effects[1].visible=false']:
            self.assertIn(x,v)

if __name__=='__main__':unittest.main()
