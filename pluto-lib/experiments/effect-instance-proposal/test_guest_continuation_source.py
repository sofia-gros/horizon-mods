"""Source-pinned continuation fixture preparation, not execution of Game."""
import hashlib
import importlib.util
import os
from pathlib import Path
import unittest
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('prepare_game_borrow',HERE/'prepare_game_borrow.py')
p=importlib.util.module_from_spec(spec);spec.loader.exec_module(p)
EXTRA={
 'crates/horizon-client/src/game/native_lifecycle.rs':'ca881b8411b33c91a466dff3bc29736da401f540e162e7e7370698b585adac86',
 'crates/horizon-client/src/game/server_travel.rs':'b6869465ea028b9a801b14e5e64e2f053d120883488fa9a60abe77296b665dc8',
}
class ContinuationSource(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  root=Path(os.environ['PLUTO_APPROVED_OFFICIAL']);cls.sources=p.read(root)
  for name,expected in EXTRA.items():
   raw=(root/name).read_bytes()
   if hashlib.sha256(raw).hexdigest()!=expected:raise ValueError('changed official source: '+name)
   cls.sources[name]=raw.decode()
  cls.lan=cls.sources['crates/horizon-client/src/game/lan_actions.rs']
  cls.observer=(HERE/'guest_continuation_observer.rs').read_text()
  native=Path('/tmp/pluto-v15-host-proof-20261106/staged/crates/horizon-client/src/game/native_lifecycle.rs').read_bytes()
  if hashlib.sha256(native).hexdigest()!='62cac276dfacf04560da74e1d232f1ac56f8ec14d9df51d736abf46f83385ace':raise ValueError('applied readiness adapter changed')
  cls.applied_readiness=native.decode()
 def test_native_turn_returns_only_after_leave(self):
  body=self.lan.split('    pub(super) fn as_guest_with_host<R>(',1)[1].split('    /// `as_guest`, with',1)[0]
  self.assertLess(body.index('self.enter_guest('),body.index('let result ='))
  self.assertLess(body.index('let result ='),body.index('self.leave_guest(host, swap)'))
  self.assertLess(body.index('self.leave_guest(host, swap)'),body.index('Some(result)'))
  self.assertNotIn('catch_unwind',body)
  self.assertNotIn('impl Drop',body)
 def test_owner_restore_then_publish_then_reentrant_notification(self):
  body=self.lan.split('    pub(super) fn leave_guest(',1)[1].split('    /// `ServerLevel.broadcastDamageEvent`',1)[0]
  self.assertLess(body.index(') = acting;'),body.index('std::mem::swap(&mut self.player, &mut player)'))
  self.assertLess(body.index('std::mem::swap(&mut self.player, &mut player)'),body.index('host.hub.teleport(i, Some(at))'))
  self.assertLess(body.index('host.hub.teleport(i, Some(at))'),body.index('self.flush_teleport_notes(host)'))
  travel=self.sources['crates/horizon-client/src/game/server_travel.rs']
  flush=travel.split('    pub(super) fn flush_teleport_notes(',1)[1].split('    /// The players the command',1)[0]
  self.assertIn('std::mem::take(&mut self.teleport_notes)',flush)
  self.assertIn('self.as_guest(host, conn, |g|',flush)
  self.assertIn('g.native_player_teleported(',flush)
 def test_epoch_exists_while_opening_and_closing(self):
  lifecycle=self.sources['crates/horizon-client/src/game/native_lifecycle.rs']
  epoch=lifecycle.split('    pub(super) fn epoch(',1)[1].split('    /// The open starts',1)[0]
  self.assertIn('self.attempt.map(',epoch)
  self.assertNotIn('OpenState::Ready',epoch)
  self.assertIn('session: OpenState::Opening',lifecycle)
  self.assertIn('attempt.session = OpenState::Closing',lifecycle)
 def test_same_level_teleport_resets_more_than_position(self):
  command=self.sources['crates/horizon-client/src/game/commands.rs']
  player=command.split('    pub(super) fn teleport_to_in_level(',1)[1].split('            Target::Player => {',1)[1].split('            Target::Mob(id) => {',1)[0]
  for anchor in ['self.stop_riding();','self.server_trips.retain(', 'p.position = v;', 'p.velocity = Vec3::new(0.0, 0.0, 0.0);','p.fall_distance = 0.0;','p.ground.on_ground = false;','self.previous_position = v;','self.teleport_notes']:
   self.assertIn(anchor,player)
 def test_cross_level_can_replace_body_not_just_coordinates(self):
  travel=self.sources['crates/horizon-client/src/game/server_travel.rs']
  body=travel.split('    pub(super) fn run_level_teleports(',1)[1]
  self.assertLess(body.index('self.enter_level(Some(host), t.dim)'),body.index('g.new_body(true)'))
  self.assertIn('host.hub\n                .change_level(',body)
 def test_native_guest_tick_has_no_ready_made_inside_continuation_frame(self):
  body=self.lan.split('    fn guest_server_tick(',1)[1]
  self.assertLess(body.index('w.server_block_damage(start)'),body.index('game.wither_rose_contact()'))
  self.assertLess(body.index('game.tick_mob_effects()'),body.index('player.carry_client_movement(tick_movement)'))
  self.assertIn('let Some(player) = game.player.as_mut() else',body)
 def test_prepared_observer_uses_real_readonly_roster_and_ready_adapter(self):
  self.assertIn('game.as_guest_with_host(host, conn, |active, actual_host|',self.observer)
  self.assertIn('actual_host.hub.guests.iter()',self.observer)
  self.assertIn('pluto_lifecycle_readiness::ready_server_epoch(',self.observer)
  self.assertIn('game.lan_acting.is_some()',self.observer)
  self.assertIn('moved.owner.dimension == before.owner.dimension',self.observer)
  self.assertIn('moved.ready_epoch == before.ready_epoch',self.observer)
  self.assertIn('p.id == id && p.position == away',self.observer)
  for forbidden in ['EntityRef::new','LevelRef::new','struct Game','struct PlayerMotion','unsafe {','catch_unwind','continue_allowed','resume_contact']:
   self.assertNotIn(forbidden,self.observer)
 def test_applied_readiness_adapter_keeps_native_private_state_owner(self):
  body=self.applied_readiness.split('pub(crate) mod pluto_lifecycle_readiness {',1)[1].split('    #[cfg(test)]',1)[0]
  self.assertIn('pub(in crate::game) fn ready_server_epoch(',body)
  self.assertIn('attempt.session == OpenState::Ready',body)
  self.assertIn('NativeAuthority::IntegratedServer |',body)
  self.assertIn('NativeAuthority::DedicatedServer',body)
 def test_round_trip_is_two_native_operations_not_position_write(self):
  self.assertEqual(self.observer.count('active.teleport_to_in_level('),2)
  self.assertNotRegex(self.observer,r'\.position\s*=(?!=)')
  self.assertIn('NOT rollback',self.observer)
 def test_outer_observation_is_after_native_wrapper_return(self):
  self.assertLess(self.observer.index('let during = game.as_guest_with_host('),self.observer.index('outer_after: owner(game)'))
  self.assertIn('during: Option<During>',self.observer)
  self.assertIn('handle: Option<EntityRef>',self.observer)
  self.assertNotIn('.unwrap()',self.observer)
if __name__=='__main__':unittest.main()
