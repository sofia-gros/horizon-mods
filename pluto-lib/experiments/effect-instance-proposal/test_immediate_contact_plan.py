"""Source/plan checks only. Never applies a patch or executes a native gate."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[3]
PLAN = ROOT / 'pluto-lib/docs/immediate-contact-validation-plan.json'


class ImmediateContactPlan(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.plan = json.loads(PLAN.read_text())
        cls.official = Path(os.environ['PLUTO_OFFICIAL_SOURCE']).resolve()
        cls.sources = {}
        rev = subprocess.check_output(['git', '-C', str(cls.official), 'rev-parse', 'HEAD'], text=True).strip()
        if rev != cls.plan['official_revision']:
            raise ValueError('wrong approved official revision')
        for path, expected in cls.plan['official_sha256'].items():
            raw = (cls.official / path).read_bytes()
            if hashlib.sha256(raw).hexdigest() != expected:
                raise ValueError('official source drift: ' + path)
            cls.sources[path] = raw.decode()
        for path, expected in cls.plan['g4_followup']['official_source_sha256'].items():
            raw = (cls.official / path).read_bytes()
            if hashlib.sha256(raw).hexdigest() != expected:
                raise ValueError('G4 official source drift: ' + path)
            cls.sources[path] = raw.decode()
        raw = Path(os.environ['PLUTO_ORDINARY_PLAN']).read_bytes()
        expected = cls.plan['g4_followup']['ordinary_plan']
        if len(raw) != expected['bytes'] or hashlib.sha256(raw).hexdigest() != expected['sha256']:
            raise ValueError('wrong frozen ordinary PlutoLib plan')
        cls.ordinary = json.loads(raw)

    def src(self, suffix):
        return self.sources['crates/horizon-client/src/' + suffix]

    def order(self, body, *needles):
        cursor = 0
        for needle in needles:
            index = body.find(needle, cursor)
            self.assertNotEqual(index, -1, needle)
            cursor = index + len(needle)

    def test_existing_inputs_remain_exact(self):
        self.assertEqual(len(self.plan['existing_artifact_sha256']), 11)
        for path, expected in self.plan['existing_artifact_sha256'].items():
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), expected, path)

    def test_real_crate_boundary_is_not_private_visibility(self):
        self.assertIn('path = "../horizon-client/src/client_lib.rs"', self.sources['crates/horizon-client-lib/Cargo.toml'])
        self.assertIn('horizon-client-lib = { path = "../horizon-client-lib"', self.sources['crates/horizon-client/Cargo.toml'])
        self.assertIn('pub mod motion;', self.src('client_lib.rs'))
        self.assertIn('pub use horizon_client_lib::*;', self.src('lib.rs'))
        self.assertIn('pub mod game;', self.src('lib.rs'))
        self.assertNotIn('pub mod game;', self.src('client_lib.rs'))

    def test_snapshot_and_later_live_reads_are_distinct(self):
        body = self.src('motion.rs').split('    fn check_inside_blocks(', 1)[1].split('    fn honey_block_inside(', 1)[0]
        self.order(body, 'Nothing below changes a block', 'let state = scene.block(',
                   'hits.push((iteration, p, state))', 'for (step, p, state) in hits',
                   'let context = self.context()', 'collision_shape_with_context(',
                   'visited.insert(p.pack())', 'crate::block_contact::Event::Inside(',
                   'match name {', 'if damage_only {', 'continue;', 'MotionEvent::EnterBlock { state }')
        self.assertIn('Ok(iterations + 1)', body)

    def test_native_control_flow_and_later_game_handoff(self):
        motion = self.src('motion.rs')
        body = motion.split('    pub fn server_block_damage(', 1)[1].split('    fn effects_from_blocks(', 1)[0]
        self.order(body, 'self.effects_from_blocks(true)?', 'self.player.ticks_frozen =', 'self.freeze_damage()?')
        turn = self.src('game/lan_actions.rs').split('    fn guest_server_tick(', 1)[1]
        turn = turn.split('let events = self.as_guest(host, conn, |game| {', 1)[1]
        self.order(turn, 'let _ = w.server_block_damage(start);', 'w.tick_environment_damage()',
                   'w.server_hazards_tick(in_wall)', 'std::mem::take(&mut w.player.events)',
                   'game.wither_rose_contact()', 'game.tick_mob_effects()', 'player.carry_client_movement(tick_movement)')

    def test_existing_native_resource_test_can_skip(self):
        body = self.src('motion.rs').split('fn standing_in_a_wither_rose_asks_for_its_effect()', 1)[1]
        self.order(body, 'if !root.join("movement-properties.nbt").exists()', 'return;', 'Assets::load(&root)', '.server_block_damage(from)')
        self.assertIn('never return/skip counted as pass', self.plan['result_protocol']['missing_resources'])
        self.assertIn('never truncate', self.plan['result_protocol']['trace_overflow'])

    def test_full_native_effect_operation_not_only_list_mutation(self):
        body = self.src('game/potions.rs').split('pub(super) fn add_player_effect(', 1)[1].split('fn mod_effect_tick(', 1)[0]
        self.order(body, 'self.potions.effects.add(effect)', 'self.apply_effect_status()',
                   'id == "absorption"', 'self.mod_effect_added(id, None, amp, new)',
                   'self.remote.is_none() && brewing::is_mod_effect(effect)')
        self.assertNotIn('if changed', body)
        effects = self.sources['crates/horizon-client-render/src/effects.rs']
        constructor = effects.split('pub fn new(', 1)[1].split('    fn infinite(', 1)[0]
        self.assertIn('Self {\n            id,', constructor)
        self.assertNotIn('strip_prefix', constructor)

    def test_all_future_gates_unproved_and_dependencies_closed(self):
        seen = set()
        gates = self.plan['gates']
        self.assertEqual(len(gates), 6)
        for gate in gates:
            self.assertNotIn(gate['id'], seen)
            self.assertTrue(set(gate['depends_on']) <= seen)
            self.assertIn(gate['status'], ['prepared_unexecuted', 'blocked_design', 'blocked_missing_fixture'])
            self.assertTrue(gate['pass_claim'])
            self.assertTrue(gate['does_not_prove'])
            seen.add(gate['id'])
        self.assertFalse(self.plan['runtime_changed'])
        self.assertFalse(self.plan['official_files_changed'])
        self.assertFalse(self.plan['production_activation'])
        self.assertTrue(self.plan['future_execution_requires_separate_grant'])
        self.assertEqual(self.plan['execution_evidence']['native_compile'], 'not_run')
        self.assertEqual(self.plan['execution_evidence']['native_runtime'], 'not_run')
        self.assertEqual(self.plan['execution_evidence']['frame_overlay'], 'not_written')

    def test_case_accounting_and_separate_continuation_obligations(self):
        cases = self.plan['cases']
        self.assertEqual(len(cases), 21)
        self.assertEqual(len({c['id'] for c in cases}), 21)
        gates = {g['id'] for g in self.plan['gates']}
        self.assertEqual({c['gate'] for c in cases}, gates)
        for case in cases:
            self.assertEqual(case['status'], 'not_run')
            self.assertTrue(case['setup'] and case['oracle'] and case['failure_or_limit'])
        self.assertEqual({o['id'] for o in self.plan['continuation_obligations']},
                         {'actor_owner', 'sampled_world', 'resource_owner', 'pending_tail', 'outer_tick', 'instance_lifetime'})
        self.assertEqual(self.plan['result_protocol']['disk_floor_bytes'], 10 * 1024**3 + 128 * 1024**2)
        self.assertTrue(self.plan['result_protocol']['no_implicit_launch'])

    def test_g4_complete_list_and_ignored_error_order(self):
        status = self.src('game/potions.rs').split('pub(super) fn apply_effect_status(', 1)[1].split('    fn thrown_player_targets(', 1)[0]
        self.order(status, 'self.potions.effects.modifiers()', 'modifiers != self.potions.applied',
                   'let _ = p.set_modifier_unclamped(attr, id, 0.0, op, false)',
                   'let _ = p.set_modifier_unclamped(attr, id, *amount, op, true)',
                   'let _ = p.clamp_health_to_max()', 'self.potions.applied = modifiers',
                   'let armor: Vec<Option<ItemStack>> = self.armor.to_vec()',
                   'self.potions.protection_for = Some', 'p.status = status')
        effects = self.sources['crates/horizon-client-render/src/effects.rs']
        modifiers = effects.split('    pub fn modifiers(', 1)[1].split('    pub fn mining_speed_factor(', 1)[0]
        self.assertIn('for e in &self.list', modifiers)
        self.assertFalse(self.plan['g4_followup']['full_installed_closure_verified'])

    def test_g4_concrete_allocator_is_not_every_trait_implementation(self):
        live = self.sources['crates/horizon-world/src/attributes/live.rs']
        self.assertIn('pub trait Identities: FnMut() -> i32', live)
        self.assertIn('hash: (self.next_identity)()', live)
        self.order(self.src('motion.rs'), 'pub fn new(resources: &MotionResources, position: Vec3)',
                   'let mut identity = 0i32', 'identity = identity.wrapping_add(1)',
                   'Self::with_attribute_identities(resources, position, &hashes, next)',
                   'pub fn with_attribute_identities(', 'next: Box<dyn horizon_world::attributes::live::Identities>')

    def test_g4_concrete_health_observer_and_native_callback_boundary(self):
        health = self.src('health.rs')
        updates = health.split("impl Observer for Updates<'_>", 1)[1].split('#[derive(Clone)]', 1)[0]
        self.assertIn('self.0.push(MotionEvent::Health(HealthEvent::Updated', updates)
        self.assertNotIn('Game', updates)
        setter = health.split('pub fn set_health(', 1)[1].split('pub(crate) struct Environment', 1)[0]
        self.order(setter, 'Updates(&mut self.events)', 'PlayerAttributeHealth::new(',
                   '&mut self.attributes', '&mut h.data', 'binding.set_absorption(value)?',
                   'binding.set_health(value)?')
        hooks = self.src('game/native_hooks.rs')
        for method, dispatcher in [('native_on_effect_added', 'dispatch_on_effect_added'),
                                   ('native_mob_effect_added', 'dispatch_mob_effect_added')]:
            section = hooks.split('pub(super) fn ' + method + '(', 1)[1]
            self.order(section, '&mut self', 'mod_api::' + dispatcher + '(self,')

    def test_g4_finite_ordinary_plan_overlap_not_installed_closure(self):
        evidence = self.plan['g4_followup']['ordinary_plan']
        self.assertIn('NOT all TConstruct/other installed mods', evidence['scope'])
        self.assertEqual({k: len(self.ordinary[k]) for k in evidence['counts']}, evidence['counts'])
        self.assertEqual(len(evidence['direct_endpoint_inventory']), 13)
        for endpoint in evidence['direct_endpoint_inventory']:
            actual = [i['injection_id'] for i in self.ordinary['injections']
                      if i['file'] == endpoint['file'] and i['target'].endswith('::' + endpoint['method'])
                      and ('::' + endpoint['owner']) in i['target']]
            self.assertEqual(actual, endpoint['matching_injection_ids'])
            self.assertEqual(actual, [])
        for saved in evidence['related_file_injections']:
            actual = next(i for i in self.ordinary['injections'] if i['injection_id'] == saved['injection_id'])
            self.assertEqual({k: actual[k] for k in saved}, saved)
        saved = evidence['same_potions_file_module']
        actual = next(m for m in self.ordinary['modules'] if m['module_id'] == saved['id'])
        self.assertEqual(actual['name'], saved['name'])
        self.assertEqual(hashlib.sha256(actual['source'].encode()).hexdigest(), saved['source_sha256'])
        self.assertEqual([s['target'] for s in self.ordinary['states']], evidence['state_targets'])
        self.assertEqual(next(g['status'] for g in self.plan['gates'] if g['id'] == 'G4_operation_continuation'), 'blocked_design')

    def test_official_inputs_unchanged_after_source_checks(self):
        self.assertEqual(len(self.plan['official_sha256']), 19)
        sources = {**self.plan['official_sha256'], **self.plan['g4_followup']['official_source_sha256']}
        for path, expected in sources.items():
            self.assertEqual(hashlib.sha256((self.official / path).read_bytes()).hexdigest(), expected, path)


if __name__ == '__main__':
    unittest.main()
