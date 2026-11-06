"""The additive cell API owns mechanics, not host authority or consumer policy."""
from pathlib import Path
import tomllib
import hashlib
import json
import unittest

ROOT = Path(__file__).resolve().parents[2]

def function_span(text, name):
    start = text.index(f'pub(super) fn {name}(')
    end = text.index('{', start) + 1
    depth = 1
    while depth:
        depth += (text[end] == '{') - (text[end] == '}')
        end += 1
    return start, end

class PlainStackCells(unittest.TestCase):
    def test_additive_native_module_has_one_owner_and_no_registry(self):
        base = ROOT / 'pluto-lib/mixins/pluto_native_creative'
        manifest = tomllib.loads((base / 'horizon-mixin.toml').read_text())
        entries = [m for m in manifest['module'] if m['id'] == 'stack-cells']
        self.assertEqual(entries, [dict(id='stack-cells', file='crates/horizon-client/src/client_lib.rs',
            name='pluto_stack_cells', source='stack-cells.rs', visibility='pub')])
        source = (base / 'stack-cells.rs').read_text()
        self.assertNotRegex(source, r'(?m)^(?://!|/\*!|#!)')
        for name in ('ChestKind', 'tconstruct', 'Game<', 'static ', 'Registry', 'item_to_nbt', 'item_from_nbt', 'container_changed'):
            self.assertNotIn(name, source)
        for name in ('pub fn pickup(', 'pub fn take(', 'pub fn collect_ordered(', 'pub fn insert_ordered(', 'ZeroCount', 'ZeroLimit'):
            self.assertIn(name, source)

    def test_ordered_insertion_reuses_planner_and_leaves_helpers_unchanged(self):
        base = ROOT / 'pluto-lib/mixins/pluto_native_creative'
        source = (base / 'stack-cells.rs').read_text().split('pub fn insert_ordered(', 1)[1]
        self.assertIn('plan_between(', source)
        self.assertIn('same(receiving, offered)', source)
        self.assertNotIn('saturating_sub', source)
        self.assert_ordered_helpers((ROOT / 'tconstruct/mixins/tconstruct/transaction.rs').read_text())

    def assert_ordered_helpers(self, consumer):
        oracle_path = ROOT / 'tconstruct/tests/rust/ordered_insertion_legacy.rs'
        oracle = oracle_path.read_text()
        receipt = json.loads(oracle_path.with_suffix('.json').read_text())
        self.assertEqual(hashlib.sha256(oracle_path.read_bytes()).hexdigest(), receipt['oracle_sha256'])
        # The whole-file hash records the immutable proof source. Other services
        # may migrate unrelated functions; the two insertion bodies below must
        # still match their exact independent originals.
        for name, expected in receipt['functions'].items():
            for text in (oracle, consumer):
                start, end = function_span(text, name)
                self.assertEqual(hashlib.sha256(text[start:end].encode()).hexdigest(), expected)

    def test_unrelated_condition_fold_edit_is_allowed(self):
        code = (ROOT / 'tconstruct/mixins/tconstruct/transaction.rs').read_text()
        start = code.index('            Self::Or(values) =>')
        end = code.index('            Self::And(values) =>', start)
        replacement = ('            Self::Or(values) => horizon_client::pluto_inventory::predicate_fold::any(\n'
                       '                values.iter().map(|value| value.matches(registries, config))),\n')
        changed = code[:start] + replacement + code[end:]
        self.assertNotEqual(code, changed)
        self.assert_ordered_helpers(changed)

    def test_either_ordered_insertion_body_drift_is_rejected(self):
        code = (ROOT / 'tconstruct/mixins/tconstruct/transaction.rs').read_text()
        for name in ('source_give_inventory', 'source_quick_move_player'):
            with self.subTest(name=name):
                start, end = function_span(code, name)
                changed = code[:end - 1] + '/* changed helper */' + code[end - 1:]
                self.assertGreater(end, start)
                with self.assertRaises(AssertionError):
                    self.assert_ordered_helpers(changed)

    def test_four_callers_still_use_same_helpers_and_only_five_bodies_changed(self):
        source = ROOT / 'tconstruct/mixins/tconstruct'
        for name in ('source-tinker-chest-click.rs', 'source-side-chest-click.rs', 'source-chest-guest-click.rs', 'source-crafting-lan.rs'):
            code = (source / name).read_text()
            self.assertIn('take_for_drop(', code)
            self.assertNotIn('pluto_stack_cells', code)
        code = (source / 'source-tinker-chests.rs').read_text()
        self.assertIn('pluto_stack_cells::pickup(', code)
        self.assertIn('pluto_stack_cells::take(', code)
        self.assertIn('pluto_stack_cells::collect_ordered(', code)
        self.assert_chest_dependencies(code)

    def assert_chest_dependencies(self, code):
        slot_path = ROOT / 'tconstruct/tests/rust/insert_slot_legacy.rs'
        slot_oracle = slot_path.read_text()
        slot_receipt = json.loads(slot_path.with_suffix('.json').read_text())
        self.assertEqual(hashlib.sha256(slot_path.read_bytes()).hexdigest(), slot_receipt['oracle_sha256'])
        start, end = function_span(slot_oracle, 'insert_slot')
        original = slot_oracle[start:end]
        self.assertEqual(hashlib.sha256(original.encode()).hexdigest(), slot_receipt['function_sha256'])
        start, end = function_span(code, 'insert_slot')
        adopted = code[start:end]
        self.assertEqual(hashlib.sha256(adopted.encode()).hexdigest(), slot_receipt['adopted_function_sha256'])
        self.assertIn('plan_between(', adopted)
        self.assertNotIn('snapshot: Vec', adopted)
        self.assertIn('kind.accepts(slot, first_equal, tagged_part)', adopted)
        self.assertIn('plan.from != 0 || plan.to != 1', adopted)
        code = code[:start] + original + code[end:]
        oracle_path = ROOT / 'tconstruct/tests/rust/plain_stack_cells_legacy.rs'
        oracle = oracle_path.read_text()
        receipt = json.loads(oracle_path.with_suffix('.json').read_text())
        self.assertEqual(receipt['source_commit'], '921bb12dc23caea0c970c5bcc39b72921e98981c')
        self.assertEqual(hashlib.sha256(oracle_path.read_bytes()).hexdigest(), receipt['oracle_sha256'])
        for name in reversed(('pickup_slot', 'pickup_player', 'take_for_drop', 'pickup_all_ordered')):
            start, end = function_span(oracle, name)
            original = oracle[start:end]
            self.assertEqual(hashlib.sha256(original.encode()).hexdigest(), receipt['functions'][name])
            start, end = function_span(code, name)
            code = code[:start] + original + code[end:]
        self.assertEqual(hashlib.sha256(code.encode()).hexdigest(), receipt['source_sha256'])

    def test_chest_insertion_dependency_drift_is_rejected(self):
        code = (ROOT / 'tconstruct/mixins/tconstruct/source-tinker-chests.rs').read_text()
        start, end = function_span(code, 'insert_slot')
        changed = code[:end - 1] + '/* changed dependency */' + code[end - 1:]
        self.assertGreater(end, start)
        with self.assertRaises(AssertionError):
            self.assert_chest_dependencies(changed)

    def test_runner_uses_actual_wrappers_types_and_locked_dependency_graph(self):
        code = (ROOT / 'tconstruct/tests/run_plain_stack_cells.py').read_text()
        self.assert_runner_dependencies(code)

    def assert_runner_dependencies(self, code):
        for snippet in ('source-tinker-chests.rs', 'stack-cells.rs', 'horizon-items', 'horizon-world',
                        'add_inventory_api(manifest, root)', 'lock_fixture_to_host(manifest, host, root)',
                        'verify_fixture_lock(manifest, host, root)', 'run_locked_fixture(manifest', 'source_sha256', 'mod consumer_impl;',
                        'mod insert_slot_legacy;', 'use crate::insert_slot_legacy::insert_slot;'):
            self.assertIn(snippet, code)
        self.assertNotIn('pub struct ItemStack', code)

    def test_strict_runner_dependency_guard_drift_is_rejected(self):
        code = (ROOT / 'tconstruct/tests/run_plain_stack_cells.py').read_text()
        for snippet in ('add_inventory_api(manifest, root)', 'lock_fixture_to_host(manifest, host, root)',
                        'verify_fixture_lock(manifest, host, root)', 'run_locked_fixture(manifest',
                        'mod consumer_impl;', 'use crate::insert_slot_legacy::insert_slot;'):
            with self.subTest(snippet=snippet):
                changed = code.replace(snippet, 'REMOVED_DEPENDENCY_GUARD')
                self.assertNotEqual(code, changed)
                with self.assertRaises(AssertionError):
                    self.assert_runner_dependencies(changed)

if __name__ == '__main__':
    unittest.main()
