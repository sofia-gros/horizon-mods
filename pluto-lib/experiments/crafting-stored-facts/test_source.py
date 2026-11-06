import copy
import importlib.util
import json
from pathlib import Path
import re
import unittest

ROOT=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('stored_render',ROOT/'render.py');R=importlib.util.module_from_spec(spec);spec.loader.exec_module(R)
I=json.loads((ROOT/'inputs.json').read_text());O=json.loads((ROOT/'oracles.json').read_text());C={n:(ROOT/n).read_text() for n in I['candidate_sha256']}

def block(s,name):
 a=s.index(name); b=s.index('{',a); depth=0
 for pos in range(b,len(s)):
  if s[pos]=='{':depth+=1
  elif s[pos]=='}':
   depth-=1
   if depth==0:return s[a:pos+1]
 raise ValueError('unbalanced')

class SourceTests(unittest.TestCase):
 def test_exact_source_composes_all_sixteen_tests(self):
  f,r=R.compose(I,O,C);self.assertEqual(len(f),9);self.assertEqual(sum(map(len,r['expected_tests'].values())),16);self.assertFalse(r['rust_executed'])
 def test_old_public_structs_and_legacy_test_body_remain_exact(self):
  old=O[R.N+'inventory-api/src/crafting/matching.rs']
  for n in ['TemplateFacts','RecipeFacts','InputFacts','Request','Registration']:
   self.assertEqual(block(old,'pub struct '+n),block(C['matching.rs'],'pub struct '+n))
  self.assertIn(old.split('#[cfg(test)]\n',1)[1],C['matching.rs'])
 def test_single_registry_and_shared_duplicate_namespace(self):
  s=C['matching.rs'];self.assertEqual(s.count('static POLICIES:'),1);self.assertEqual(s.count('RwLock<BTreeMap'),1)
  self.assertIn('self.insert(entry.id, entry.interest, Entry::Legacy',s);self.assertIn('self.insert(entry.id, entry.interest, Entry::Stored',s)
 def test_missing_capability_is_denial_not_absence_or_skip(self):
  self.assertIn('stored.map_or(Decision::Deny',C['matching.rs']);self.assertIn('self.evaluate_inner(request, None)',C['matching.rs'])
 def test_native_reader_avoids_clone_codec_unsafe_or_foreign_impl(self):
  s=C['native-adapter.rs'].split('// Exact borrowed saved-field access;',1)[1]
  for x in ['.clone(', 'encode_', 'decode_', 'unsafe', 'impl matching::StoredValue for']:
   self.assertNotIn(x,s)
  self.assertIn('stack.data()?.other(id)?',s);self.assertIn('value = value.get(key)?',s)
 def test_interest_still_precedes_capture_and_result_creation(self):
  s=C['native-adapter.rs'];self.assertLess(s.index('matching::prepare'),s.index('ItemFacts::capture'));self.assertLess(s.index('matching::prepare'),s.index('template.create()'))
 def test_policy_not_installed_and_old_hooks_not_removed(self):
  self.assertFalse(I['runtime_installed']);self.assertFalse(I['old_hooks_removed']);self.assertNotIn('horizon-mixin.toml',C)
  self.assertNotIn('tconstruct:',C['matching.rs']);self.assertNotIn('tconstruct:',C['native-adapter.rs'])
 def test_source_and_oracle_drift_fail_closed(self):
  for which,key in [('candidate','consumer.rs'),('oracle',R.T+'anvil-material-recipe.rs')]:
   c=copy.deepcopy(C);o=copy.deepcopy(O)
   (c if which=='candidate' else o)[key]+='\n'
   with self.assertRaises(ValueError):R.compose(I,o,c)
 def test_extra_or_missing_input_is_rejected(self):
  for change in ['add','drop']:
   c=copy.deepcopy(C)
   if change=='add':c['unknown.rs']=''
   else:c.pop('consumer.rs')
   with self.assertRaises(ValueError):R.compose(I,O,c)
 def test_wrong_cutoff_or_execution_claim_is_rejected(self):
  for key,value in [('base','0'*40),('horizon','0'*40),('native_executed',True),('runtime_installed',True),('old_hooks_removed',True)]:
   i=copy.deepcopy(I);i[key]=value
   with self.assertRaises(ValueError):R.compose(i,O,C)
 def test_native_whole_source_check_is_required_before_real_preparation(self):
  with self.assertRaises(ValueError):R.verify_native(I,O,'','')
 def test_no_new_scheduler_registry_or_hooks_in_policy(self):
  s=C['consumer.rs']
  for x in ['static ', 'thread_local!', 'menu_session', 'set_other', 'ControlFlow', '#[test]']:
   self.assertNotIn(x,s)
  self.assertEqual(s.count('matching::register_stored('),1)
 def test_new_api_tests_are_distinct_and_unrun(self):
  s=C['matching.rs'].split('mod stored_tests {',1)[1]
  names=re.findall(r'#\[test\]\s*fn (\w+)',s);self.assertEqual(len(names),6);self.assertEqual(len(set(names)),6);self.assertFalse(I['native_executed'])

if __name__=='__main__':unittest.main()
