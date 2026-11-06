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
        for path, expected in cls.plan['fixture_admission_followup']['official_source_sha256'].items():
            raw = (cls.official / path).read_bytes()
            if hashlib.sha256(raw).hexdigest() != expected:
                raise ValueError('fixture admission official source drift: ' + path)
            cls.sources[path] = raw.decode()
        evidence = cls.plan['fixture_admission_followup']['game_ci']
        ci_root = Path(os.environ['PLUTO_GAME_CI_SOURCE']).resolve()
        cls.game_ci = {}
        for path, expected in evidence['source_sha256'].items():
            raw = subprocess.check_output(['git', '-C', str(ci_root), 'show',
                                           evidence['source_commit'] + ':' + path])
            if hashlib.sha256(raw).hexdigest() != expected:
                raise ValueError('frozen Game CI source drift: ' + path)
            cls.game_ci[Path(path).name] = raw.decode()
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

    def test_fixture_constructor_and_remote_test_are_not_server_open(self):
        game = self.src('game.rs')
        body = game.split("    fn new(\n        res: &'a Arc<Resources>,", 1)[1].split('/// CPU-only startup data', 1)[0]
        self.order(body, 'streaming.paused = true', 'player: None',
                   'native_lifecycle: Default::default()', 'if game.native_window()',
                   'game.native_client_start()', 'Ok(game)')
        self.assertNotIn('native_lifecycle_world_ready', body)
        loader = game.split('fn load_resources(', 1)[1]
        self.order(loader, 'if headless {', 'UiResources::server(root)', 'UiResources::load(root, vanilla_root)')
        remote = self.src('game/native_entity_refs.rs').split('fn a_client_reads_the_entities_and_menu_the_server_shows_it()', 1)[1]
        self.order(remote, 'HORIZON_TEST_RESOURCES', 'load_resources(', 'Game::new(',
                   'Pipe::pair()', 'game.connect_local(pipe, "Steve")', 'game.remote.as_ref()')

    def test_fixture_manual_roster_is_not_native_ready_history(self):
        body = self.src('game/building_players.rs').split('fn for_player_keeps_the_outer_body_in_its_original_dimension()', 1)[1]
        self.order(body, 'ExecutionRole::IntegratedServer', 'Game::new(', 'game.player = Some(',
                   'game.lan_host(', 'Guest::new(', 'guest.placed = true', 'guest.loaded = true',
                   'hub.guests.push(guest)', 'game.for_player(')
        self.assertNotIn('native_lifecycle_', body)
        self.assertNotIn('integrated_open', body)
        host = self.src('game/lan.rs').split('fn lan_host(', 1)[1].split('pub(super) fn lan_close(', 1)[0]
        self.assertIn('self.lan = Some(LanHost {', host)
        self.assertNotIn('native_lifecycle_', host)
        lifecycle = self.src('game/native_lifecycle.rs')
        self.order(lifecycle, '#[derive(Default)]', 'pub(super) struct NativeLifecycle',
                   'attempt: Option<OpenAttempt>', 'pub(super) fn begin_open(',
                   'session: OpenState::Opening', 'pub(super) fn world_open_ready(',
                   'let attempt = self.attempt.as_mut()?', 'attempt.session != OpenState::Opening',
                   'attempt.session = OpenState::Ready')

    def test_fixture_real_integrated_open_precedes_ready_and_harness(self):
        integrated = self.src('game/integrated.rs')
        opening = integrated.split('    fn integrated_open(', 1)[1].split('    fn stop_integrated(', 1)[0]
        self.order(opening, 'self.native_server_about_to_start(false)', 'self.native_lifecycle_open_started()',
                   'self.join_world_rest()', 'self.finish_open_world(', 'self.start_mods()',
                   'self.lan_host(settings, net, config)', 'self.serving = Some(Serving {', 'self.settle_spawn()')
        running = integrated.split('    fn run_integrated_inner(', 1)[1].split('    fn integrated_open(', 1)[0]
        self.order(running, 'if let Err(e) = self.integrated_open(', 'self.native_lifecycle_world_failed(',
                   'return Err(e)', 'self.native_lifecycle_world_ready()', 'HORIZON_COMMAND_SCRIPT',
                   'self.run_command_harness(', 'self.stop_integrated()', 'return result.and(stopped)')

    def test_fixture_harness_entry_conflicts_with_frozen_observer(self):
        harness = self.src('game/command_harness.rs')
        run = harness.split('pub(super) fn run_command_harness(', 1)[1]
        self.order(run, 'guests.iter().any(|g| g.placed && g.loaded)', 'self.tick_server(true)?',
                   'Instant::now() > deadline', 'owner did not connect',
                   'self.harness_owner(|g| g.harness_settle(deadline, None))?')
        owner = harness.split('    fn harness_owner<T>(', 1)[1].split('    fn harness_owner_menu(', 1)[0]
        self.order(owner, '.serving', '.owner', 'self.lan.take()',
                   'find(|g| g.uuid == owner && g.placed)', 'self.enter_level(',
                   '.enter_guest(', 'self.lan = Some(host)', 'let result = f(self)',
                   'self.lan.take()', 'self.leave_guest(', 'host.hub.sync_all_effects()',
                   'self.run_level_teleports(', 'self.enter_level(', 'self.lan = Some(host)',
                   'restored?', 'result')
        observer = (ROOT / 'pluto-lib/experiments/effect-instance-proposal/guest_continuation_observer.rs').read_text()
        self.assertIn('game.remote.is_some() || game.lan.is_some() || game.lan_acting.is_some()', observer)
        self.assertIn('server is not Ready; an epoch alone is insufficient', observer)
        wrapper = self.src('game/lan_actions.rs').split('pub(super) fn as_guest_with_host<R>(', 1)[1].split('pub(super) fn as_guest_messaged', 1)[0]
        self.order(wrapper, 'self.enter_guest(',
                   '(self.serving.is_some() || self.options.dedicated.is_some()).then_some(self.dim.id())',
                   'block_contact::in_level(', 'self.leave_guest(')

    def test_fixture_native_loaded_facts_have_different_meanings(self):
        hub = self.src('lan/hub.rs')
        placement = hub.split('pub fn place_new_player(', 1)[1]
        self.order(placement, 'self.guests[i].placed = true', 'self.resend_teleport(')
        packet = hub.split('S::PlayerLoaded(_) => {', 1)[1].split('S::ChunkBatchReceived(', 1)[0]
        self.order(packet, 'g.loaded = true', 'g.loading = false', 'g.load_timeout = 0',
                   'events.push(HostEvent::PlayerLoaded(conn))')
        method = self.src('lan/guest.rs').split('pub fn client_loaded(&self) -> bool {', 1)[1].split('    }', 1)[0]
        self.assertEqual(method.strip(), '!self.dead && self.load_timeout == 0')
        self.assertIn('motion.client_loaded = g.client_loaded()', self.src('game/lan_actions.rs'))
        findings = self.plan['fixture_admission_followup']['findings']
        self.assertEqual(len(findings), 7)
        self.assertEqual(len({f['id'] for f in findings}), 7)
        self.assertTrue(all(f['contract'] and f['limit'] and f['source'] for f in findings))

    def test_fixture_existing_ci_is_compile_only_and_probes_unchanged(self):
        ci = self.game_ci['ci.py']
        self.order(ci, 'for kind in ("positive", "negative")', 'expected_overlay(',
                   'command = ["cargo", "check", "--locked"]', '"-p", "horizon-client", "--lib"',
                   '"--no-default-features", "--message-format=json", "-vv"',
                   'runner.check(kind, command, stage, 0 if kind == "positive" else 101)',
                   'checks.reject_diagnostic(messages, content)')
        self.assertNotIn('guest_continuation_observer', ci)
        for name in ['game_borrow_positive.rs', 'game_borrow_negative.rs']:
            self.assertEqual(self.game_ci[name],
                             (ROOT / 'pluto-lib/experiments/effect-instance-proposal' / name).read_text())
        self.assertEqual(self.plan['fixture_admission_followup']['game_ci']['execution'], 'not_run')

    def test_official_inputs_unchanged_after_source_checks(self):
        self.assertEqual(len(self.plan['official_sha256']), 19)
        sources = {**self.plan['official_sha256'], **self.plan['g4_followup']['official_source_sha256'],
                   **self.plan['fixture_admission_followup']['official_source_sha256']}
        for path, expected in sources.items():
            self.assertEqual(hashlib.sha256((self.official / path).read_bytes()).hexdigest(), expected, path)


if __name__ == '__main__':
    unittest.main()
