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

    def test_official_inputs_unchanged_after_source_checks(self):
        self.assertEqual(len(self.plan['official_sha256']), 19)
        for path, expected in self.plan['official_sha256'].items():
            self.assertEqual(hashlib.sha256((self.official / path).read_bytes()).hexdigest(), expected, path)


if __name__ == '__main__':
    unittest.main()
