"""Source/inverse checks only; the five Rust regression tests require a native gate."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
GROUP = ROOT / 'integration/storage-intake-20261107'
PATH = 'tconstruct/mixins/tconstruct/source-melter-state.rs'
PLUTO = 'pluto-lib/mixins/pluto_native_creative/stack-storage.rs'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def restore(raw, row):
    if sha(raw) != row['after_sha256']:
        raise ValueError('Postimage differs')
    text = raw.decode()
    addition = row['test_append']
    if not text.endswith(addition) or text.count(addition) != 1:
        raise ValueError('Rust test append differs')
    text = text[:-len(addition)]
    for before, after in reversed(row['edits']):
        if text.count(after) != 1:
            raise ValueError('Production inverse is not unique')
        text = text.replace(after, before, 1)
    raw = text.encode()
    if sha(raw) != row['before_sha256']:
        raise ValueError('Before image differs')
    return raw


class PortableStorageMigration(unittest.TestCase):
    def setUp(self):
        self.inverse = json.loads((GROUP/'inverse.json').read_bytes())
        self.row = self.inverse['production'][PATH]
        self.raw = (ROOT/PATH).read_bytes()
        self.source = self.raw.decode()

    def test_exact_two_production_edits_and_full_inverse(self):
        self.assertEqual(set(self.inverse['production']), {PATH})
        self.assertEqual(len(self.row['edits']), 2)
        self.assertEqual(sha(restore(self.raw, self.row)), self.row['before_sha256'])

    def test_unexpected_source_change_rejected(self):
        with self.assertRaises(ValueError): restore(self.raw+b' ', self.row)

    def test_input_gate_uses_existing_shared_storage(self):
        body = self.source.split('pub(super) fn portable_tank(',1)[1].split('/// Stage either recipe',1)[0]
        self.assertIn('if !casting_item_save_safe(stack) { return None; }', body)
        self.assertNotIn('item_from_nbt', body)
        self.assertLess(body.index('casting_item_save_safe'), body.index('let raw ='))

    def test_result_gate_uses_existing_shared_storage(self):
        body = self.source.split('pub(super) fn fill_portable(',1)[1].split('pub(super) fn fill_portable_tagged(',1)[0]
        self.assertIn('casting_item_save_safe(&result).then_some(result)', body)
        self.assertNotIn('item_from_nbt', body)
        self.assertIn('portable_tank(stack, tank.capacity).as_ref() != Some(&tank)', body)
        self.assertIn('tank.to_nbt_preserving(raw)?', body)

    def test_existing_shared_gate_is_not_reimplemented(self):
        body = self.source.split('pub(super) fn casting_item_save_safe(',1)[1].split('\n}',1)[0]
        self.assertIn('horizon_client::pluto_stack_storage::encode_item_lossless(item).is_some()', body)
        self.assertEqual(self.source.count('pub(super) fn casting_item_save_safe('), 1)
        self.assertNotIn('fn contains_retained_wire(', self.source)

    def test_original_wire_and_schema_policy_remains(self):
        body = self.source.split('pub(super) fn portable_tank(',1)[1].split('/// Stage either recipe',1)[0]
        for literal in ['stack.count != 1', 'stack.damage != 0', 'stack.wire_patch().is_some()',
                        'data.wire.patch().is_some()', '["FluidName", "Amount", "Tag", "Tank", "Fluid"]',
                        '(tank.to_nbt_preserving(raw).as_ref() == Some(raw)).then_some(tank)']:
            self.assertIn(literal, body)

    def test_five_native_cases_cover_recursive_and_positive_paths(self):
        addition = self.row['test_append']
        self.assertEqual(addition.count('#[test]'), 5)
        for text in ['container.push', 'charged.push', 'pot_decorations', 'bundle_contents.push',
                     'same_item_snapshot', 'wire_patch().is_none()', 'JavaString(vec![0xd800])',
                     'Tag::Float(0x7fc01234)', 'portable_storage_keeps_existing_tag_and_capacity_policy']:
            self.assertIn(text, addition)

    def test_partition_is_exact_and_does_not_claim_all_complete(self):
        tracker = json.loads((GROUP/'tracker.json').read_bytes())
        rows = tracker['rows']
        self.assertEqual(len(rows), 70)
        self.assertEqual(len({r['number'] for r in rows}), 70)
        self.assertFalse(any(r['feature_completion_verified'] for r in rows))
        self.assertEqual({r['number'] for r in rows if r['status']=='changed-pending'}, {922})
        self.assertEqual({r['number'] for r in rows if r['status']=='included'}, {918})


if __name__ == '__main__':
    unittest.main()
