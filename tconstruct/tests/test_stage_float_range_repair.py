"""Synthetic, bounded filesystem/byte tests. No generated game or native execution."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('repair', Path(__file__).parents[1] / 'tools' / 'stage_float_range_repair.py')
r = importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(r)


def body():
    return r.SIGNATURE+b'\n'+r.CONTEXT+b'\n            _ => {}\n        };\n'+r.TAIL+b'\n'+r.NEXT+b'\n        self.origin\n    }\n}\n'


class Tests(unittest.TestCase):
    def test_exact_one_space(self):
        before=body();after,offset=r.candidate(before)
        self.assertEqual(len(after),len(before)+1)
        self.assertEqual(after[:offset]+after[offset+1:],before)
        self.assertIn(r.GOOD,after);self.assertNotIn(r.BAD,after)

    def test_crlf_is_preserved(self):
        before=body().replace(b'\n',b'\r\n');after,offset=r.candidate(before)
        self.assertEqual(after[:offset]+after[offset+1:],before)

    def test_already_correct_rejected(self):
        with self.assertRaises(ValueError):r.candidate(body().replace(r.BAD,r.GOOD))

    def test_duplicate_bad_rejected(self):
        with self.assertRaises(ValueError):r.candidate(body()+r.BAD)

    def test_mixed_good_bad_rejected(self):
        with self.assertRaises(ValueError):r.candidate(body()+r.GOOD)

    def test_different_context_rejected(self):
        with self.assertRaises(ValueError):r.candidate(body().replace(b'as_f64()',b'as_f32()'))

    def test_changed_method_rejected(self):
        with self.assertRaises(ValueError):r.candidate(body().replace(b'pub fn new(',b'pub fn other('))

    def test_duplicate_method_rejected(self):
        with self.assertRaises(ValueError):r.candidate(r.SIGNATURE+body())

    def test_intervening_method_rejected(self):
        with self.assertRaises(ValueError):r.candidate(body().replace(r.CONTEXT,b'    fn other() {}\n'+r.CONTEXT))

    def test_impl_close_rejected(self):
        with self.assertRaises(ValueError):r.candidate(body().replace(r.CONTEXT,b'}\n'+r.CONTEXT))

    def test_changed_tail_rejected(self):
        with self.assertRaises(ValueError):r.candidate(body().replace(b'origin: ChunkPos::new(0, 0)',b'origin: ChunkPos::new(1, 0)'))

    def test_missing_following_member_rejected(self):
        with self.assertRaises(ValueError):r.candidate(body().replace(r.NEXT,b''))

    def test_mixed_line_endings_rejected(self):
        with self.assertRaises(ValueError):r.candidate(body().replace(b'\n',b'\r\n',1))

    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);base=Path(self.temp.name)
        self.original=base/'original';self.modded=base/'modded';self.mixins=base/'mixins';self.plan=base/'plan.json'
        for p in (self.original,self.modded,self.mixins):p.mkdir()
        self.before=body();self.pristine=self.before.replace(r.BAD,r.GOOD)
        for root,data in ((self.original,self.pristine),(self.modded,self.before)):
            p=root/r.TARGET;p.parent.mkdir(parents=True);p.write_bytes(data);(root/'Cargo.lock').write_bytes(b'locked\n')
        (self.modded/'other.rs').write_bytes(b'untouched\n')
        (self.mixins/'Cargo.lock').write_bytes(b'prettyplease fixed synthetic lock\n')
        self.closure={'injections':[{'file':r.TARGET,'injection_id':'synthetic'}],
                      'modules':[{'file':r.TARGET,'module_id':str(i)} for i in range(9)],'states':[],'bridges':[]}
        self.value={'schema':6,'inputs':{r.TARGET:r.sha(self.pristine),'Cargo.lock':r.sha(b'locked\n')},**self.closure}
        self.plan.write_text(json.dumps(self.value))
        patches={'ORIGINAL_SHA':r.sha(self.pristine),'HORIZON_LOCK':r.sha(b'locked\n'),
                 'MIXINS_PINS':{'Cargo.lock':r.sha((self.mixins/'Cargo.lock').read_bytes())},'CLOSURE_SHA':r.sha(r.canonical(self.closure))}
        self.mock=patch.multiple(r,**patches);self.mock.start();self.addCleanup(self.mock.stop)

    def call(self, **kw):
        return r.repair(self.original,self.modded,self.mixins,self.plan,horizon_revision=kw.get('horizon',r.HORIZON),mixins_revision=r.MIXINS)

    def test_synthetic_repair_receipt_and_full_inverse(self):
        others=r.snapshot(self.modded);original=r.snapshot(self.original);mixins=r.snapshot(self.mixins)
        result=self.call();after=(self.modded/r.TARGET).read_bytes();self.assertEqual(after,self.pristine)
        self.assertTrue(result['inverse_verified']);self.assertFalse(result['ast_verified']);self.assertEqual(result['inserted_bytes'],1)
        self.assertEqual(r.snapshot(self.original),original);self.assertEqual(r.snapshot(self.mixins),mixins)
        others.pop(r.TARGET);now=r.snapshot(self.modded);now.pop(r.TARGET);self.assertEqual(now,others)
        self.assertEqual(json.loads((self.modded.parent/r.RECEIPT).read_bytes()),result)

    def test_revision_failure_leaves_stage(self):
        with self.assertRaises(ValueError):self.call(horizon='0'*40)
        self.assertEqual((self.modded/r.TARGET).read_bytes(),self.before)

    def test_original_pin_failure_leaves_stage(self):
        (self.original/r.TARGET).write_bytes(self.pristine+b' ')
        with self.assertRaises(ValueError):self.call()
        self.assertEqual((self.modded/r.TARGET).read_bytes(),self.before)

    def test_lock_pin_failure(self):
        (self.mixins/'Cargo.lock').write_bytes(b'drift')
        with self.assertRaises(ValueError):self.call()

    def test_plan_closure_failure(self):
        v=copy.deepcopy(self.value);v['modules'].pop();self.plan.write_text(json.dumps(v))
        with self.assertRaises(ValueError):self.call()

    def test_plan_official_input_failure(self):
        v=copy.deepcopy(self.value);v['inputs'][r.TARGET]='0'*64;self.plan.write_text(json.dumps(v))
        with self.assertRaises(ValueError):self.call()

    def test_preexisting_receipt_rejected(self):
        (self.modded.parent/r.RECEIPT).write_text('old')
        with self.assertRaises(ValueError):self.call()

    def test_root_alias_rejected(self):
        with self.assertRaises(ValueError):r.repair(self.original,self.original,self.mixins,self.plan,horizon_revision=r.HORIZON,mixins_revision=r.MIXINS)

    def test_link_in_stage_rejected(self):
        (self.modded/'linked').symlink_to(self.original/'Cargo.lock')
        with self.assertRaises(ValueError):self.call()

    def test_final_other_file_drift_rejected(self):
        replace=r.os.replace
        def drift(*args):
            replace(*args);(self.modded/'other.rs').write_bytes(b'drift')
        with patch.object(r.os,'replace',side_effect=drift):
            with self.assertRaisesRegex(ValueError,'Other stage'):self.call()
        self.assertFalse((self.modded.parent/r.RECEIPT).exists())

    def test_final_plan_drift_rejected(self):
        replace=r.os.replace
        def drift(*args):replace(*args);self.plan.write_bytes(b'drift')
        with patch.object(r.os,'replace',side_effect=drift):
            with self.assertRaisesRegex(ValueError,'Plan or pristine'):self.call()
        self.assertFalse((self.modded.parent/r.RECEIPT).exists())


if __name__=='__main__':unittest.main()
