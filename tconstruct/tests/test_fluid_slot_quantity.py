"""Fluid temporary slots retain policy and delegate only movement quantities."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'tconstruct/mixins/tconstruct/source-fluid-container.rs'
ORACLE = ROOT / 'tconstruct/tests/rust/fluid_slot_quantity_legacy.rs'
META = json.loads(ORACLE.with_suffix('.json').read_text())

def span(text, name):
    start = text.index('pub(super) fn ' + name + '(')
    end = text.index('{', start) + 1
    depth = 1
    while depth:
        depth += (text[end] == '{') - (text[end] == '}')
        end += 1
    return start, end

def check_scope(case, source):
    oracle = ORACLE.read_text()
    case.assertEqual(hashlib.sha256(ORACLE.read_bytes()).hexdigest(), META['oracle_sha256'])
    for name in reversed(META['functions']):
        a, b = span(oracle, name); old = oracle[a:b]
        case.assertEqual(hashlib.sha256(old.encode()).hexdigest(), META['functions'][name]['old_sha256'])
        a, b = span(source, name); new = source[a:b]
        case.assertEqual(hashlib.sha256(new.encode()).hexdigest(), META['functions'][name]['adopted_sha256'])
        marker = '    match slots[index].as_mut() {' if name == 'click_with_color' else '    let amount = match slots[0].as_mut() {'
        case.assertEqual(old.split(marker, 1)[0], new.split('    use horizon_client::pluto_inventory::', 1)[0])
        case.assertIn('plan_between(', new)
        case.assertIn('receiving.item == offered.item && receiving.damage == offered.damage && receiving.extra == offered.extra', new)
        case.assertNotIn('pluto_stack_cells::pickup', new)
        source = source[:a] + old + source[b:]
    case.assertEqual(hashlib.sha256(source.encode()).hexdigest(), META['source_sha256'])

class FluidSlotQuantity(unittest.TestCase):
    def test_complete_old_functions_restore_the_entire_original_source(self):
        check_scope(self, SOURCE.read_text())

    def test_empty_cursor_extraction_prefix_is_byte_identical(self):
        source = SOURCE.read_text(); a, b = span(source, 'click_with_color')
        prefix = source[a:b].split('    if index != 0 ||')[0]
        self.assertEqual(hashlib.sha256(prefix.encode()).hexdigest(), META['preserved_extraction_prefix_sha256'])

    def test_all_successful_admission_families_require_positive_counts(self):
        source = SOURCE.read_text()
        for name, next_name, count_gate in (
            ('tool_capacity', 'transfer_tool', 'stack.count != 1'),
            ('can_contents', 'no_crafting_remainder', 'stack.count == 0'),
            ('potion_data', 'potion_tag', 'stack.count == 0'),
            ('lantern_tank', 'casting_portable_rewrite_safe', 'stack.count == 0'),
            ('accepts_with_color', 'effect_instance', 'stack.count > 0'),
        ):
            body = source[source.index('fn ' + name + '('):source.index('fn ' + next_name + '(')]
            self.assertIn(count_gate, body)
        check_scope(self, source) # Pins the complete dependency implementations.

    def test_local_and_guest_call_sites_are_unchanged(self):
        for path, expected in META['callers'].items():
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), expected)

    def test_policy_or_dependency_drift_is_rejected(self):
        source = SOURCE.read_text()
        for before, after in (
            ('if index == 1 && carried.is_some()', 'if index == 0 && carried.is_some()'),
            ('stack.count.div_ceil(2)', 'stack.count / 2'),
            ('receiving.extra == offered.extra', 'true'),
            ('stack.count != 1 || !stack.item.starts_with', 'stack.count == 1 || !stack.item.starts_with'),
            ('if index != 0 || !accepts_with_color', 'if index != 0 || accepts_with_color'),
        ):
            with self.subTest(before=before):
                self.assertIn(before, source)
                with self.assertRaises(AssertionError):
                    check_scope(self, source.replace(before, after, 1))

    def test_fixture_compiles_exact_real_module_and_one_locked_api(self):
        runner = (ROOT / 'tconstruct/tests/run_fluid_slot_quantity.py').read_text()
        for snippet in ('actual.write_bytes(production + extension)', "actual.read_bytes()[:len(production)] != production",
                        'add_inventory_api(manifest, root)', 'lock_fixture_to_host(manifest, host, root)',
                        'run_locked_fixture(manifest', 'verify_fixture_lock(manifest, host, root)',
                        'horizon-items', 'horizon-world', 'horizon-client-core', 'pluto_potion_fixture.rs'):
            self.assertIn(snippet, runner)
        self.assertNotIn('pub struct ItemStack', runner)
        tests = (ROOT / 'tconstruct/tests/rust/fluid_slot_quantity.rs').read_text()
        for count in ('19440', '3240', '540'):
            self.assertIn('assert_eq!(cases, ' + count + ')', tests)
        self.assertIn('new_trace.into_inner(), old_trace.into_inner()', tests)

if __name__ == '__main__':
    unittest.main()
