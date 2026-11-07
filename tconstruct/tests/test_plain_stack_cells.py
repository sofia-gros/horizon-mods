"""The additive cell API owns mechanics, not host authority or consumer policy."""
from pathlib import Path
import tomllib
import hashlib
import json
import unittest

ROOT = Path(__file__).resolve().parents[2]

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
        for name in ('pub fn pickup(', 'pub fn take(', 'pub fn collect_ordered(', 'ZeroCount', 'ZeroLimit'):
            self.assertIn(name, source)

    def test_four_callers_still_use_same_helpers_and_only_three_bodies_changed(self):
        source = ROOT / 'tconstruct/mixins/tconstruct'
        for name in ('source-tinker-chest-click.rs', 'source-side-chest-click.rs', 'source-chest-guest-click.rs', 'source-crafting-lan.rs'):
            code = (source / name).read_text()
            self.assertIn('take_for_drop(', code)
            self.assertNotIn('pluto_stack_cells', code)
        code = (source / 'source-tinker-chests.rs').read_text()
        self.assertIn('pluto_stack_cells::pickup(', code)
        self.assertIn('pluto_stack_cells::take(', code)
        self.assertIn('pluto_stack_cells::collect_ordered(', code)
        oracle_path = ROOT / 'tconstruct/tests/rust/plain_stack_cells_legacy.rs'
        oracle = oracle_path.read_text()
        receipt = json.loads(oracle_path.with_suffix('.json').read_text())
        self.assertEqual(receipt['source_commit'], '921bb12dc23caea0c970c5bcc39b72921e98981c')
        self.assertEqual(hashlib.sha256(oracle_path.read_bytes()).hexdigest(), receipt['oracle_sha256'])
        def span(text, name):
            start = text.index(f'pub(super) fn {name}(')
            end = text.index('{', start) + 1
            depth = 1
            while depth:
                depth += (text[end] == '{') - (text[end] == '}')
                end += 1
            return start, end
        for name in reversed(('pickup_player', 'take_for_drop', 'pickup_all_ordered')):
            start, end = span(oracle, name)
            original = oracle[start:end]
            self.assertEqual(hashlib.sha256(original.encode()).hexdigest(), receipt['functions'][name])
            start, end = span(code, name)
            code = code[:start] + original + code[end:]
        self.assertEqual(hashlib.sha256(code.encode()).hexdigest(), receipt['source_sha256'])

    def test_runner_uses_actual_wrappers_types_and_locked_dependency_graph(self):
        code = (ROOT / 'tconstruct/tests/run_plain_stack_cells.py').read_text()
        for snippet in ('source-tinker-chests.rs', 'stack-cells.rs', 'horizon-items', 'horizon-world',
                        'add_inventory_api(manifest, root)', 'lock_fixture_to_host(manifest, host, root)',
                        'verify_fixture_lock(manifest, host, root)', 'run_locked_fixture(manifest', 'source_sha256'):
            self.assertIn(snippet, code)
        self.assertNotIn('pub struct ItemStack', code)

if __name__ == '__main__':
    unittest.main()
