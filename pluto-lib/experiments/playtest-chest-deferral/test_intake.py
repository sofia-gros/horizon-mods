import importlib.util,json,unittest,tomllib
from pathlib import Path
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('intake',HERE/'intake.py');m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
BLOCK=m.inputs()['removed_block'].encode()
PREFIX=b'[mod]\nid="tconstruct"\nversion="0.1.0"\n\n'
TAIL=b'[[injection]]\nid="unrelated"\ntarget="keep"\nkind="HEAD"\nhelper="keep.rs"\n\n[[module]]\nid="guide"\nfile="game.rs"\nname="text"\nsource="text.rs"\n'
class IntakeTests(unittest.TestCase):
 def test_remove_one(self):self.assertEqual(m.transform(PREFIX+BLOCK+TAIL),PREFIX+TAIL)
 def test_other_bytes_preserved(self):
  before=PREFIX+b'# retained exact comment\n'+BLOCK+TAIL
  self.assertEqual(m.transform(before),before.replace(BLOCK,b''))
 def test_absent_rejected(self):
  with self.assertRaises(ValueError):m.transform(PREFIX+TAIL)
 def test_duplicate_block_rejected(self):
  with self.assertRaises(ValueError):m.transform(PREFIX+BLOCK+BLOCK+TAIL)
 def test_duplicate_id_rejected(self):
  other=BLOCK.replace(b'kind = "RETURN"',b'kind = "HEAD"')
  with self.assertRaises(ValueError):m.transform(PREFIX+BLOCK+other+TAIL)
 def test_modified_policy_block_rejected(self):
  with self.assertRaises(ValueError):m.transform(PREFIX+BLOCK.replace(b'pos',b'other_pos')+TAIL)
 def test_exact_input_guard(self):
  with self.assertRaises(ValueError):m.apply_exact(PREFIX+BLOCK+TAIL,b'helper')
 def test_restore_requires_exact_output(self):
  with self.assertRaises(ValueError):m.restore_exact(PREFIX+TAIL)
 def test_counts(self):
  r=m.inputs();self.assertEqual(r['before_injections']-r['after_injections'],1);self.assertEqual(r['unchanged_counts'],{'module':251,'state':108,'crate':1})
 def test_only_one_target_path(self):
  v=json.loads((HERE/'source-validation.json').read_text());self.assertEqual(v['production_changed_paths'],[m.inputs()['target']]);self.assertFalse(v['native_executed']);self.assertFalse(v['plan_executed'])
 def test_old_return_contract_retained_for_restore(self):
  d=tomllib.loads(BLOCK.decode())['injection'][0];self.assertEqual(d['kind'],'RETURN');self.assertEqual(d['helper'],'source-chest-pick.rs');self.assertEqual(d['bindings'][0]['ty'],'Option<ItemStack>')
 def test_no_source_or_schema_deletion(self):
  patch=(HERE/'adoption.patch').read_text();self.assertEqual(patch.count('--- '),1);self.assertIn('horizon-mixin.toml',patch);self.assertEqual(m.inputs()['preserved_helper']['path'],'tconstruct/mixins/tconstruct/source-chest-pick.rs')
if __name__=='__main__':unittest.main()
