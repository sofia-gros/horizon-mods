"""Pure source/negative checks. No Rust, metadata, Git, plan or host execution."""
import ast
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('chest_policy_renderer',HERE/'render.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

def args():
    o=m.frozen_inputs()
    return [o['old_helper'],o['metadata_store'],(HERE.parent/'latest-chest-pick/source-chest-pick.rs').read_bytes(),o['native_pick'],(HERE/'regression_tests.rs').read_bytes()]

class Tests(unittest.TestCase):
    def test_01_full_source_inventory(self):
        generated,receipt=m.compose()
        self.assertEqual(set(generated),{'lib.rs','old_policy.rs','new_policy.rs','native_pick.rs','regression_tests.rs'})
        self.assertEqual([len(receipt['test_names'][n]) for n in ['original3','additional14']],[3,14])
        self.assertFalse(receipt['native_executed'])
        self.assertEqual(receipt['horizon'],m.HORIZON)

    def test_02_entire_native_pick_including_old_tests_unchanged(self):
        generated,_=m.compose()
        self.assertEqual(generated['native_pick.rs'],m.frozen_inputs()['native_pick'])
        self.assertIn(b's.same_item_same_components(stack)',generated['native_pick.rs'])
        self.assertIn(b'None if infinite_materials',generated['native_pick.rs'])

    def test_03_both_helper_substitutions_have_exact_inverse(self):
        a=args();generated,_=m.compose()
        for name,original in [('old_policy.rs',a[0]),('new_policy.rs',a[2])]:
            changed=generated[name].removeprefix(b'use horizon_items::items::ItemStack;\n')
            changed=changed.replace(m.FIXTURE_DECL,m.DECL).replace(m.FIXTURE_ARG,m.ARG).replace(m.FIXTURE_READ,m.READ)
            self.assertEqual(changed,original)

    def test_04_policy_guard_tail_keeps_exact_types_and_assignment(self):
        a=args();anchor=b'    if &*stack.item'
        self.assertEqual(a[0][a[0].index(anchor):],a[2][a[2].index(anchor):])
        self.assertIn(b'Tag::Int(color)',a[2]);self.assertIn(b'dyed_color=Some(color)',a[2])
        self.assertNotIn(b'creative',a[2]);self.assertNotIn(b'include_data',a[2])

    def test_05_wrong_body_for_each_input_rejected(self):
        for i in range(5):
            a=args();a[i]+=b'\n'
            with self.assertRaises(ValueError):m.render(*a)

    def test_06_reader_acquisition_change_rejected(self):
        store=args()[1]
        for changed in [store.replace(b'pos.0 >> 4',b'pos.0 >> 3'),store.replace(m.READER_START,m.READER_START*2),store.replace(b'    if let Some(value)=raw.get(key)',b'    if let Some(value)=raw.get("different")')]:
            with self.assertRaises(ValueError):m.reader_slice(changed)

    def test_07_reader_suffix_exact_and_world_acquisition_excluded(self):
        store=args()[1];reader,suffix=m.reader_slice(store)
        self.assertIn(suffix,store)
        self.assertNotIn(b'world.chunks',reader)
        self.assertLess(suffix.index(b'raw.get(key)'),suffix.index(b'raw.get("id")'))
        self.assertIn(b'raw.get("color")',suffix)

    def test_08_ambiguous_helper_substitution_rejected(self):
        old=args()[0]
        for before in [m.READ,m.ARG,m.DECL]:
            with self.assertRaises(ValueError):m.helper_slice(old.replace(before,before+before))

    def test_09_existing_fixture_names_cannot_leak_into_original(self):
        for after in [m.FIXTURE_READ,m.FIXTURE_ARG,m.FIXTURE_DECL]:
            with self.assertRaises(ValueError):m.helper_slice(args()[0]+after)

    def test_10_unexpected_policy_change_rejected_even_if_hash_updated(self):
        a=args();a[2]=a[2].replace(b'Tag::Int',b'Tag::Long');c=copy.deepcopy(m.config());c['new_helper_sha256']=m.sha(a[2])
        with mock.patch.object(m,'config',return_value=c),self.assertRaises(ValueError):m.render(*a)

    def test_11_expected_test_inventory_not_just_count(self):
        a=args();a[4]=a[4].replace(b'fn none_bypasses_',b'fn changed_none_bypasses_');c=copy.deepcopy(m.config());c['regression_sha256']=m.sha(a[4])
        with mock.patch.object(m,'config',return_value=c),self.assertRaises(ValueError):m.render(*a)

    def test_12_wrong_pin_or_execution_claim_rejected(self):
        original=m.HERE
        for key,value in [('horizon','f'*40),('candidate','0'*40),('native_executed',True),('game_type_verified',True)]:
            c=m.config();c[key]=value
            with tempfile.TemporaryDirectory() as td:
                p=Path(td);(p/'inputs.json').write_text(json.dumps(c))
                with mock.patch.object(m,'HERE',p),self.assertRaises(ValueError):m.config()
        self.assertEqual(m.HERE,original)

    def test_13_no_native_or_subprocess_execution_in_renderer(self):
        tree=ast.parse((HERE/'render.py').read_text())
        imported={n.names[0].name for n in ast.walk(tree) if isinstance(n,ast.Import)}
        self.assertFalse(imported & {'subprocess','os','resource'})
        self.assertNotIn('cargo',(HERE/'render.py').read_text().lower())

    def test_14_runtime_selection_and_wire_checks_are_explicit(self):
        tests=args()[4]
        for token in [b'bundle_selected.0',b'.wire.patch()',b'got.wire_patch().is_none()',b'None::<ItemStack>.map',b'old_meta.reads.get()',b'crate::native_pick::pick']:
            self.assertIn(token,tests)
        self.assertNotIn(b'#[ignore]',tests)
        generated,_=m.compose()
        self.assertNotIn(b'struct Game',b''.join(generated.values()))

if __name__=='__main__':unittest.main()
