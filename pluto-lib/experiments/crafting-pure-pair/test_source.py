import copy,importlib.util,json,unittest
from pathlib import Path
H=Path(__file__).resolve().parent
sp=importlib.util.spec_from_file_location('pair_render',H/'render.py');R=importlib.util.module_from_spec(sp);sp.loader.exec_module(R)
I=json.loads((H/'inputs.json').read_text());O=json.loads((H/'oracles.json').read_text());C={n:(H/n).read_text() for n in I['candidate_sha256']}
class Checks(unittest.TestCase):
 def test_exact_source_and_sixteen_unique_finite_cases(self):
  files,r=R.compose(I,O,C);self.assertEqual(len(files),7);self.assertEqual(len(r['tests']),16);self.assertFalse(r['rust_executed'])
 def test_old_pair_body_and_native_fragments_are_byte_exact(self):
  files,_=R.compose(I,O,C);self.assertEqual(files['overslime-crafting.rs'],O['overslime-crafting.rs'])
  for k,v in O.items():
   if k.startswith('official/'):self.assertIn(v,files['crafting.rs'])
  for k in ['old-assembly.rs','old-remainders.rs']:self.assertIn(O[k],files['crafting.rs'])
 def test_only_amount_and_refill_native_policy_slices_are_used(self):
  files,_=R.compose(I,O,C);s=files['overslime-arithmetic.rs'];self.assertEqual(s.count('pub(super) fn '),2)
  self.assertIn(R.function(O['overslime.rs'],'amount'),s);self.assertIn(R.function(O['overslime.rs'],'refill'),s)
 def test_policy_call_graph_has_no_external_state(self):
  s=O['overslime-crafting.rs']+R.function(O['overslime.rs'],'amount')+R.function(O['overslime.rs'],'refill')
  for x in ['static ', 'thread_local!', 'SystemTime','Instant','rand','Game','world.', 'std::fs', 'Mutex', 'RwLock']:
   self.assertNotIn(x,s)
  self.assertIn('let mut output = tool.clone();',s);self.assertIn('original.count = 1',s)
 def test_veto_comes_before_selection_and_output(self):
  s=C['native-derivation.rs'];start=s.index('fn after_matching_from(');end=s.index('pub fn after_matching(',start);s=s[start:end]
  self.assertLess(s.index('Outcome::Denied'),s.index('assembly_from('))
  self.assertIn('crate::crafting::pluto_matching::evaluate(input, recipe)',C['matching-head.rs'])
 def test_registry_has_one_pair_owner_and_unlocked_callbacks(self):
  s=C['native-derivation.rs'];self.assertEqual(s.count('static RECIPES:'),1);self.assertEqual(s.count('RwLock<BTreeMap'),1)
  body=s[s.index('fn insert('):s.index('fn snapshot(')];self.assertNotIn('.derive)',body)
  self.assertIn('select(registry.snapshot(),',s);self.assertIn('Selection::Conflict => return ControlFlow::Break(None)',s)
 def test_legacy_grid_shapes_and_failure_values_remain_explicit(self):
  s=C['native-derivation.rs'];self.assertIn('input.stacks().into_iter().map(|stack| Some(stack.clone()))',s)
  self.assertIn('derive(grid)',s);self.assertIn('vec![None; size * size]',s);self.assertIn('pair.remainders.len() == grid.len()',s)
 def test_new_library_layer_has_no_tconstruct_policy_or_storage_authority(self):
  s=C['native-derivation.rs'];self.assertNotIn('tconstruct:',s)
  for x in ['set_other','menu_session','encode_','decode_','wire_patch','thread_local!','unsafe']:self.assertNotIn(x,s)
  self.assertIn('pluto_inventory_api::crafting::products::Pair',s)
 def test_old_module_or_candidate_drift_and_extras_rejected(self):
  for values,key in [('candidate','consumer.rs'),('oracle','overslime-crafting.rs')]:
   c=copy.deepcopy(C);o=copy.deepcopy(O);(c if values=='candidate' else o)[key]+='\n'
   with self.assertRaises(ValueError):R.compose(I,o,c)
  c=copy.deepcopy(C);c['extra']=''
  with self.assertRaises(ValueError):R.compose(I,O,c)
 def test_activation_and_wrong_cutoff_fail(self):
  for key,value in [('base','0'*40),('native_executed',True),('old_hook_removed',True)]:
   i=copy.deepcopy(I);i[key]=value
   with self.assertRaises(ValueError):R.compose(i,O,C)
 def test_actual_native_whole_source_must_be_supplied(self):
  with self.assertRaises(ValueError):R.verify_native(I,O,'')
 def test_fixture_does_not_include_or_mock_game(self):
  files,_=R.compose(I,O,C);lib=files['lib.rs']
  self.assertIn('pub use horizon_items::{items,item_data};',lib);self.assertNotIn('struct Game',str(files))
  self.assertFalse(I['actual_ordinary_activation']);self.assertFalse(I['old_hook_removed'])
if __name__=='__main__':unittest.main()
