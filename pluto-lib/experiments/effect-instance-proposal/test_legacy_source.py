"""Pinned source-only ownership/oracle checks, not actor/save execution."""
import importlib.util
import os
from pathlib import Path
import unittest
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('prepare_legacy', HERE / 'prepare_legacy.py')
p = importlib.util.module_from_spec(spec); spec.loader.exec_module(p)

class LegacySourceGuards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.sources = p.read_sources(Path(os.environ['PLUTO_APPROVED_OFFICIAL']))
    def source(self, relative): return self.sources['crates/horizon-client/src/' + relative].decode()
    def test_oracle_keeps_exact_native_expression(self):
        save = self.source('save.rs')
        expression = save.split(p.START,1)[1].split(p.END,1)[0][:-1]
        self.assertIn(expression, p.oracle(save))
        self.assertIn('Some(Tag::Byte(v)) => i32::from(*v as u8)', expression)
    def test_actual_nbt_and_number_modules_are_unchanged(self):
        files = p.generated(self.sources)
        self.assertEqual(files['official-nbt.rs'], self.sources['crates/horizon-world/src/nbt.rs'])
        self.assertEqual(files['official-nbt-value.rs'], self.sources['crates/horizon-client-core/src/nbt_value.rs'])
        self.assertNotIn(b'pub enum Tag', files['legacy_view.rs'])
    def test_local_and_lan_share_existing_tuple_boundaries(self):
        self.assertIn('level.active_effects = self.potions.effects.saved();', self.source('game.rs'))
        lan = self.source('lan/playerdata.rs')
        self.assertIn('guest.effects = crate::effects::PlayerEffects::from_saved(&level.active_effects);', lan)
        self.assertIn('level.active_effects = guest.effects.saved();', lan)
    def test_guest_mutation_lives_inside_real_swap(self):
        lan = self.source('game/lan_actions.rs')
        body = lan.split('pub(super) fn as_guest_with_host',1)[1].split('pub(super) fn as_guest_messaged',1)[0]
        self.assertLess(body.index('self.enter_guest('), body.index('f(self, host)'))
        self.assertLess(body.index('f(self, host)'), body.index('self.leave_guest('))
        self.assertEqual(lan.count('self.potions.swap_player(&mut g.effects, &mut play.potions);'), 2)
    def test_autosave_snapshot_and_logout_barrier_are_explicit(self):
        lan = self.source('lan/playerdata.rs')
        enqueue = lan.split('pub fn enqueue(',1)[1].split('pub fn save_after(',1)[0]
        self.assertLess(enqueue.index('tag: player_tag(guest)'), enqueue.index('writer.send.send(snapshot)'))
        after = lan.split('pub fn save_after(',1)[1].split('pub fn completed(',1)[0]
        self.assertLess(after.index('self.completed(true)'), after.index('save(guest, world)'))
    def test_contacts_have_snapshot_and_visited_before_callback(self):
        motion = self.source('motion.rs')
        body = motion.split('    fn check_inside_blocks(',1)[1]
        self.assertLess(body.index('hits.push((iteration, p, state))'), body.index('for (step, p, state) in hits'))
        self.assertLess(body.index('inside && visited.insert(p.pack())'), body.index('Event::Inside('))
        self.assertLess(body.index('Event::Inside('), body.index('match name {', body.index('Event::Inside(')))
    def test_inspection_has_no_writer_registry_or_actor_escape(self):
        source = (HERE / 'legacy_view.rs').read_text()
        for token in ['write_dat(', '&mut Game', 'unsafe', 'Box::leak', 'potions::effect(', 'record_nbt::encode(', 'pub fn commit']:
            self.assertNotIn(token, source)
        self.assertIn("pub original: Option<&'a Tag>", source)
        self.assertIn('pub native_tuple: Option<NativeTuple>', source)
    def test_bounds_precede_id_conversion(self):
        source = (HERE / 'legacy_view.rs').read_text()
        self.assertLess(source.index('if list.len() > limits.max_entries'), source.index('text(entry.get("id"))'))
        self.assertLess(source.index('total.checked_add'), source.index('let entries = list.iter().map'))

if __name__ == '__main__': unittest.main()
