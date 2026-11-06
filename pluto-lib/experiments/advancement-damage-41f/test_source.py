"""Pure source/order checks. No Cargo, rustfmt, planner, apply or native execution."""
import copy, hashlib, importlib.util, json, os, unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
META=HERE.parents[1]/'tools'
BASE=Path(os.environ['PLUTO_BASE_ROOT'])/'pluto-lib'
OFFICIAL=Path(os.environ['PLUTO_41F_ROOT'])
GAME=Path(os.environ['PLUTO_41F_GAME'])
OLD_ADV=Path(os.environ['PLUTO_624C_ADVANCEMENTS'])
E=json.loads((HERE/'evidence.json').read_bytes())
sha=lambda b:hashlib.sha256(b).hexdigest()
def load(name,path):
 s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
A=load('source_advancement_guard',BASE/'tools/check_advancement_plan.py')
D=load('source_damage_guard',BASE/'tools/check_damage_equipment_plan.py')
def entries():return {r['path'].split('/')[-1]:json.loads((META/r['path'].split('/')[-1]).read_bytes()) for r in E['metadata']}
def adv_plan(rows):return {'schema':6,'injections':[dict(r,kind='HEAD',cancellable=True,mod_id='pluto_native_creative') for r in rows]}
def damage_plan(rows):return {'schema':6,'injections':[dict(r,kind='HEAD',priority=-2147483648,cancellable=True,mod_id='pluto_native_creative') for r in rows]}
def native(path):return GAME if path=='crates/horizon-client/src/game.rs' else OFFICIAL/path
class Tests(unittest.TestCase):
 def test_exact_metadata_bytes_and_inverse(self):
  for record in E['metadata']:
   name=record['path'].split('/')[-1];raw=(META/name).read_bytes();self.assertEqual(sha(raw),record['after_sha256'])
   old=(BASE/'tools'/name).read_bytes();self.assertEqual(sha(old),record['before_sha256'])
   original=json.loads(old);inverse=json.loads(raw);inverse['official_horizon']=original['official_horizon']
   if 'official_mixins' in original:inverse['official_mixins']=original['official_mixins']
   for a,b in zip(inverse.get('boundaries',[inverse]),original.get('boundaries',[original])):a['file_sha256']=b['file_sha256']
   self.assertEqual(inverse,original);self.assertEqual((json.dumps(inverse,indent=2)+'\n').encode(),old)
 def test_all_official_bytes_and_git_blobs(self):
  for path,record in E['native_sources'].items():
   b=native(path).read_bytes();self.assertEqual(len(b),record['bytes']);self.assertEqual(sha(b),record['sha256'])
   self.assertEqual(hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest(),record['git_blob'])
 def test_damage_original_body_oracles_unchanged(self):
  for name,row in entries().items():
   if name=='advancement-replacements.json':continue
   s=native(row['file']).read_text(encoding='utf-8');i=s.index(row['start']);j=s.index(row['end'],i)
   self.assertEqual(sha(s[i:j].rstrip().encode()),row['function_sha256'])
 def test_advancement_full_function_bytes_unchanged(self):
  old=OLD_ADV.read_bytes();self.assertEqual(sha(old),'a31910d9f940113fb32cf709345cb8dd2953bb77f8634b6d0a186e043c8d6bc4')
  new=native('crates/horizon-client/src/game/advancements.rs').read_text(encoding='utf-8')
  for prefix in ['    fn inventory_index(', '    pub(super) fn check_inventory_trigger(']:
   def extract(s):
    self.assertEqual(s.count(prefix),1);i=s.index(prefix);j=s.index('\n    }',i)+len('\n    }');return s[i:j]
   self.assertEqual(extract(old.decode()),extract(new))
 def test_helpers_unchanged(self):
  for path,record in E['unchanged_helpers'].items():self.assertEqual(sha((BASE/'mixins/pluto_native_creative'/path).read_bytes()),record['sha256'])
 def test_advancement_exclusive_and_binding_negatives(self):
  rows=entries()['advancement-replacements.json']['boundaries'];p=adv_plan(rows);A.check_order(p,rows)
  for mutation in ['duplicate','binding','fingerprint','priority']:
   q=copy.deepcopy(p)
   if mutation=='duplicate':q['injections'].append(copy.deepcopy(q['injections'][0]))
   elif mutation=='binding':q['injections'][0]['bindings']=[]
   elif mutation=='fingerprint':q['injections'][0]['function_fingerprint']='wrong'
   else:q['injections'][0]['priority']=0
   with self.assertRaises(ValueError):A.check_order(q,rows)
 def test_damage_exclusion_negatives(self):
  rows=[r for n,r in entries().items() if n!='advancement-replacements.json'];p=damage_plan(rows);D.check_order(p,rows)
  for kind,priority in [('MODIFY_VARIABLE',0),('HEAD',-2147483648)]:
   q=copy.deepcopy(p);extra=copy.deepcopy(q['injections'][0]);extra.update(mod_id='other',injection_id='other',kind=kind,priority=priority);q['injections'].append(extra)
   with self.assertRaises(ValueError):D.check_order(q,rows)
  q=copy.deepcopy(p);q['injections'][0]['function_fingerprint']='wrong'
  with self.assertRaises(ValueError):D.check_order(q,rows)
 def test_actual_evidence_scope_is_not_promoted(self):
  self.assertEqual(E['actual_run'],42231588813);self.assertFalse(E['full_guard_rerun_passed']);self.assertFalse(E['apply_or_native_type_executed'])
  self.assertFalse(E['runtime_changed']);self.assertFalse(E['guard_algorithm_changed'])
if __name__=='__main__':unittest.main()
