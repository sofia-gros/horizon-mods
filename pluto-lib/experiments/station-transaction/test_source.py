import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('station_compose', ROOT / 'compose.py')
CMP = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(CMP)
I = json.loads((ROOT / 'inputs.json').read_text())
O = json.loads((ROOT / 'oracles.json').read_text())
C = {n: (ROOT / n).read_text() for n in I['candidate_sha256']}

class SourceTests(unittest.TestCase):
    def test_exact_composition_has_four_outputs_and_no_activation(self):
        files, receipt = CMP.compose(I, O, C)
        self.assertEqual(len(files), 4)
        self.assertFalse(receipt['ordinary_manifest_changed'])
        self.assertFalse(receipt['native_executed'])

    def test_transaction_signature_and_body_are_exact(self):
        files, _ = CMP.compose(I, O, C)
        self.assertEqual(files['station-transaction-consumer.rs'][len(C['consumer-prefix.rs']):], O['old-transaction.rs'])

    def test_open_registration_is_after_remote_lan_before_all_local_writes(self):
        files, _ = CMP.compose(I, O, C); s = files['source-crafting-station.rs']
        start = s.index('pub(super) fn open('); end = s.index('pub(super) fn side(')
        s = s[start:end]
        self.assertLess(s.index('if game.remote.is_some()'), s.index(CMP.INSERT))
        self.assertLess(s.index('if game.lan_acting.is_some()'), s.index(CMP.INSERT))
        self.assertLess(s.index('return true;'), s.index(CMP.INSERT))
        for marker in ['let key =', '.get_or_insert_with(', 'game.craft_grid.clone_from', '_crafting_sessions_mut().insert']:
            self.assertLess(s.index(CMP.INSERT), s.index(marker))
        self.assertEqual(files['source-crafting-station.rs'].replace(CMP.INSERT, ''), O['station.rs'])

    def test_observed_session_mutations_are_one_insert_one_remove(self):
        uses = I['session_accessor_occurrences']
        self.assertEqual(len(uses), 2)
        self.assertTrue(all(u['id'] == 'source-crafting-station' for u in uses))
        self.assertEqual(sum('.insert(' in u['text'] for u in uses), 1)
        self.assertEqual(sum('.remove(' in u['text'] for u in uses), 1)

    def test_stale_session_is_interested_without_new_block_filter(self):
        interest = C['consumer-prefix.rs'].split('fn interest(', 1)[1].split('pub(crate) fn ensure', 1)[0]
        self.assertIn('position(game).is_some()', interest)
        self.assertNotIn('world.block', interest)
        self.assertNotIn('block_entities', interest)

    def test_veto_runs_before_any_transaction_selection(self):
        s = C['native-transaction.rs'].split('pub fn attempt(', 1)[1]
        self.assertIn('match super::pluto_placement::attempt', s)
        self.assertIn('ControlFlow::Break(()) => ControlFlow::Break(())', s)
        self.assertIn('ControlFlow::Continue(()) => dispatch', s)

    def test_claim_never_reexecutes_native_or_calls_handler_twice(self):
        s = C['native-transaction.rs'].split('fn dispatch(', 1)[1].split('pub fn attempt(', 1)[0]
        self.assertEqual(s.count('handle(game, book, entry_index, use_max)'), 1)
        self.assertIn('let _ = handle(game, book, entry_index, use_max);\n    ControlFlow::Break(())', s)
        self.assertIn('let Some(handle) = selected else { return ControlFlow::Continue(()); };', s)
        self.assertNotIn('game.place_recipe(', s)

    def test_no_registry_lock_is_taken_inside_callback_loop(self):
        s = C['native-transaction.rs'].split('fn dispatch(', 1)[1].split('pub fn attempt(', 1)[0]
        self.assertLess(s.index('registry.snapshot()'), s.index('(entry.interest)(game)'))
        self.assertNotIn('.read()', s); self.assertNotIn('.write()', s)
        self.assertLess(s.index('if conflict'), s.index('handle(game'))

    def test_registration_is_one_static_entry_without_callback_reentry(self):
        s = C['consumer-prefix.rs']
        self.assertEqual(s.count('OnceLock<Result<(), RegistrationError>>'), 1)
        self.assertEqual(s.count('pluto_transactions::register('), 1)
        self.assertIn('if let Err(error) = result', s)
        self.assertIn('panic!(', s)
        initializer = s.split('get_or_init(', 1)[1].split('));', 1)[0]
        self.assertNotIn('ensure()', initializer); self.assertNotIn('position(', initializer)

    def test_old_policy_order_and_native_menu_transaction_are_unmodified(self):
        s = O['old-transaction.rs']
        markers = ['position(game)', '!= "tconstruct:crafting_station"', '.cloned()', '::refresh(game)', 'game.menu_session(', 'menu.slot_count() != 46', '.placement(&entry.id)', 'let before:', 'menu.place_recipe(', 'let after:', '!menu.dropped.is_empty()', 'grid = Some(', 'game.craft_grid = grid', '::sync(game)', 'game.fill_ghost(', 'game.recipe_book_changed()']
        positions = [s.index(x) for x in markers]
        self.assertEqual(positions, sorted(positions))
        self.assertNotIn('game.place_recipe(', s)

    def test_wrong_or_extra_candidate_and_oracle_fail_closed(self):
        for source, key in [('candidate', 'single-head.rs'), ('oracle', 'old-transaction.rs')]:
            c = copy.deepcopy(C); o = copy.deepcopy(O)
            (c if source == 'candidate' else o)[key] += '\n'
            with self.assertRaises(ValueError): CMP.compose(I, o, c)
        c = dict(C, unknown='')
        with self.assertRaises(ValueError): CMP.compose(I, O, c)

    def test_cutoff_and_execution_claim_drift_fail_closed(self):
        for key, value in [('base', '0'*40), ('official', '0'*40), ('runtime_installed', True), ('old_hook_removed', True), ('native_executed', True)]:
            i = copy.deepcopy(I); i[key] = value
            with self.assertRaises(ValueError): CMP.compose(i, O, C)

    def test_native_source_must_match_and_lan_is_not_composed(self):
        with self.assertRaises(ValueError): CMP.verify_native(I, O, '')
        files, _ = CMP.compose(I, O, C)
        self.assertNotIn('lan-handler.rs', files)
        self.assertNotIn('lan_place_recipe', C['single-head.rs'])

if __name__ == '__main__': unittest.main()
