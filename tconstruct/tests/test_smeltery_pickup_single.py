"""Static routing checks; runtime, multiplayer and drag need separate verification."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]/'mixins/tconstruct'
class PickupWiring(unittest.TestCase):
 def test_local_guard_is_bounded_and_commits_before_cursor(self):
  s=(ROOT/'source-smeltery-click.rs').read_text()
  f=s.split('// Bounded ordinary left-click:',1)[1].split('if input == horizon_client::menu::Input::QuickMove',1)[0]
  for x in ['game.lan.is_none()', 'button == 0', 'data.wire.patch().is_none()', 'horizon_items::bundle::Contents::of', 'no_retained_wire(live)', '*live == expected', 'prepare_single_insert']:
   self.assertIn(x,f)
  self.assertLess(f.index('tconstruct_source_melter_store::write'),f.index('game.carried = Some(rest)'))
  self.assertIn('!matches!(input, horizon_client::menu::Input::Pickup | horizon_client::menu::Input::QuickMove)', s)
 def test_guest_checks_packet_and_access_and_commits_before_cursor(self):
  s=(ROOT/'source-smeltery-guest-guard.rs').read_text()
  self.assertLess(s.index('packet.state != menu_state'),s.index('// Keep vanilla\'s authority checks'))
  f=s.split('// Keep vanilla\'s authority checks',1)[1].split('if packet.input == 1',1)[0]
  for x in ['packet.input == 0','packet.button == 0','Slots::Block(at) if at == pos','spectator()', 'still_valid(', 'count() == 1', 'smeltery_ui_pages().contains_key(&pos)', 'sent_page_matches(&menu.sent, &expected)', 'data.wire.patch().is_none()', 'horizon_items::bundle::Contents::of', 'prepare_single_insert']:
   self.assertIn(x,f)
  self.assertLess(f.index('sent_page_matches(&menu.sent, &expected)'),f.index('tconstruct_source_melter_store::write'))
  self.assertLess(f.index('tconstruct_source_melter_store::write'),f.index('hub.guests[gi].carried = Some(rest)'))
  self.assertIn('if packet.input != 0 {', s)
if __name__=='__main__':unittest.main()
