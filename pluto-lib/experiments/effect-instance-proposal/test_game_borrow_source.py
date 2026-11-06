"""Source guards for unexecuted actual-Game compilation probes."""
import importlib.util
import os
from pathlib import Path
import unittest
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('prepare_game_borrow',HERE/'prepare_game_borrow.py')
p=importlib.util.module_from_spec(spec);spec.loader.exec_module(p)

class GameBorrowSource(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.sources=p.read(Path(os.environ['PLUTO_APPROVED_OFFICIAL']))
        cls.original=cls.sources[p.SOURCE]
        cls.positive=(HERE/'game_borrow_positive.rs').read_text()
        cls.negative=(HERE/'game_borrow_negative.rs').read_text()
    def test_additions_are_exact_and_original_game_is_unchanged(self):
        for kind in ['positive','negative']:
            self.assertTrue(p.proposed(self.original,kind).startswith(self.original+'\n'))
            self.assertEqual((HERE/f'UNAPPLIED-game-borrow-{kind}.patch').read_text(),p.patch(self.original,kind))
            self.assertEqual(p.proposed(self.original,kind).count("struct Game<'a> {"),1)
    def test_actual_private_game_requires_its_real_crate(self):
        self.assertIn("struct Game<'a> {",self.original)
        self.assertNotIn("pub struct Game<'a> {",self.original)
        self.assertIn('pub mod game;',self.sources['crates/horizon-client/src/lib.rs'])
        native=self.sources['crates/horizon-client/src/game/native_entity_refs.rs']
        self.assertIn('#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]',native)
        self.assertIn('let mut game = Game::new(',native)
    def test_probes_reference_real_types_and_all_motion_fields(self):
        for source in [self.positive,self.negative]:
            self.assertIn('Game<\'_>',source)
            self.assertIn('motion::MotionWorld',source)
            for field in ['entities:', 'player,', 'scene:', 'assets:', 'collision_binding:', 'resources:', 'health:']:
                self.assertIn(field,source)
            for forbidden in ['struct Game', 'struct MotionWorld', 'unsafe {', 'Box::leak', 'EntityRef::new', 'LevelRef::new']:
                self.assertNotIn(forbidden,source)
    def test_positive_ends_motion_call_before_game_mutation(self):
        body=self.positive.split('    fn sequential_borrows(',1)[1].split('    // A future',1)[0]
        self.assertLess(body.index('let before = observe_motion(game)?'),body.index('game.add_player_effect(effect)'))
        self.assertLess(body.index('game.add_player_effect(effect)'),body.index('let after = observe_motion(game)?'))
    def test_negative_keeps_real_borrow_live_after_game_call(self):
        self.assertLess(self.negative.index('let player = game.player.as_mut()?'),self.negative.index('game.add_player_effect(effect)'))
        self.assertLess(self.negative.index('game.add_player_effect(effect)'),self.negative.index('Some(world.player.position)'))
        self.assertIn('EXPECTED E0499',self.negative)
    def test_teleport_observation_uses_real_guest_owner_and_keeps_missing_handles(self):
        self.assertIn('game.as_guest(host, conn, |active| {',self.positive)
        self.assertIn('active.teleport_to_in_level(Target::Player, to, None, false)',self.positive)
        self.assertIn('let handle = active.native_entity(id);',self.positive)
        self.assertIn('let after_handle = active.native_entity(id);',self.positive)
        self.assertNotIn('.unwrap()',self.positive)
    def test_native_entity_handle_is_not_a_position_generation(self):
        api=self.sources['crates/horizon-client/src/mod_api.rs']
        body=api.split('pub struct EntityRef {',1)[1].split('}',1)[0]
        self.assertEqual(body.strip(),'epoch: NativeWorldEpoch,\n    id: u32,')
        command=self.sources['crates/horizon-client/src/game/commands.rs']
        body=command.split('    pub(super) fn teleport_to_in_level(',1)[1].split('            Target::Mob(id) =>',1)[0]
        self.assertIn('p.position = v;',body)
        self.assertIn('self.teleport_notes',body)
    def test_guest_handle_lookup_needs_roster_not_just_acting_fields(self):
        native=self.sources['crates/horizon-client/src/game/native_entity_refs.rs']
        target=native.split('    pub(super) fn native_entity_target(',1)[1].split('    fn native_client_tracked(',1)[0]
        self.assertLess(target.index('.lan'),target.index('self.lan_acting == Some(g.conn)'))
        lan=self.sources['crates/horizon-client/src/game/lan.rs']
        tick=lan.split('    pub(super) fn tick_lan(',1)[1]
        self.assertLess(tick.index('self.lan.take()'),tick.index('self.tick_guests(&mut host)'))

if __name__=='__main__':unittest.main()
