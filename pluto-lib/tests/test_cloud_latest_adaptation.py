"""Source and synthetic-plan checks only. No native planner/compiler/Game runs."""
from pathlib import Path
import copy
import hashlib
import importlib.util
import json
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parent
META_PATH = ROOT / 'tools/cloud-insertion-boundaries.json'
REPORT = json.loads((ROOT / 'docs/cloud-playtest-adaptation-41f.json').read_text())
SPEC = importlib.util.spec_from_file_location('cloud_plan', ROOT / 'tools/check_cloud_insertion_plan.py')
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)

def sha(data):
    return hashlib.sha256(data).hexdigest()

class LatestCloudSource(unittest.TestCase):
    def setUp(self):
        self.meta = json.loads(META_PATH.read_text())

    def synthetic(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        workspace = Path(temp.name)
        meta = copy.deepcopy(self.meta)
        for name in meta['official_source_sha256']:
            p = workspace / name
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text('opaque guard fixture: ' + name)
            meta['official_source_sha256'][name] = sha(p.read_bytes())
        plan = {'schema':6, 'inputs':dict(meta['official_source_sha256']),
                'modules':[], 'injections':[], 'states':[]}
        for module in meta['modules']:
            row = {k:module[k] for k in ('module_id','file','name','visibility')}
            row.update(mod_id='pluto_native_creative', source=(ROOT/'mixins/pluto_native_creative'/module['source_file']).read_text())
            plan['modules'].append(row)
        return workspace, meta, plan

    def test_only_revision_and_ten_full_file_pins_change(self):
        self.assertEqual(sha(META_PATH.read_bytes()), REPORT['candidate_metadata_sha256'])
        self.assertEqual(self.meta['horizon_revision'], '41f822b659789780f58d32c63ddfb6887ea1b8bf')
        self.assertEqual(len(self.meta['official_source_sha256']), 17)
        self.assertEqual(sum(self.meta['official_source_sha256'][p] != h
                             for p,h in REPORT['old_official_source_sha256'].items()), 10)
        old = copy.deepcopy(self.meta)
        old['horizon_revision'] = REPORT['old_revision']
        old['official_source_sha256'] = REPORT['old_official_source_sha256']
        self.assertEqual(sha((json.dumps(old,indent=2)+'\n').encode()), REPORT['original_metadata_sha256'])

    def test_guard_and_five_runtime_modules_are_unchanged(self):
        self.assertEqual(sha((ROOT/'tools/check_cloud_insertion_plan.py').read_bytes()), REPORT['unchanged_guard_sha256'])
        self.assertEqual(len(self.meta['modules']), 5)
        for m in self.meta['modules']:
            self.assertEqual(sha((ROOT/'mixins/pluto_native_creative'/m['source_file']).read_bytes()), m['source_sha256'])

    def test_seventeen_exact_source_review_records_cover_candidate(self):
        rows = REPORT['official_inputs']
        self.assertEqual({r['path']:r['latest_sha256'] for r in rows}, self.meta['official_source_sha256'])
        self.assertEqual(sum(not r['identical'] for r in rows),10)
        self.assertEqual({r['path']:r['guard_sha256'] for r in rows}, REPORT['old_official_source_sha256'])

    def test_twenty_conflicting_rows_and_payloads_remain_pinned(self):
        manifest = tomllib.loads((REPO/REPORT['integration_manifest']['path']).read_text())
        self.assertEqual(len(REPORT['conflicts']),20)
        self.assertEqual(sum(r['kind']=='module' for r in REPORT['conflicts']),4)
        for row in REPORT['conflicts']:
            found = [x for x in manifest[row['entry_section']] if x['id']==row['id']]
            self.assertEqual(len(found),1)
            expected = {k:row[k] for k in found[0]}
            self.assertEqual(found[0],expected)
            data=(REPO/row['payload_path']).read_bytes()
            self.assertEqual(sha(data),row['payload_sha256'])
            self.assertEqual(hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest(),row['payload_git_blob'])
            self.assertTrue(row['manifest_same_as_main08afa'] and row['payload_same_as_main08afa'])
            self.assertFalse(row['allowed_by_candidate'])

    def test_synthetic_cloud_only_shape_passes_not_an_actual_plan(self):
        workspace,meta,plan=self.synthetic()
        GUARD.check(plan,workspace,meta=meta)

    def test_all_twenty_conflicts_still_reject_without_allowlist(self):
        workspace,meta,base=self.synthetic()
        for row in REPORT['conflicts']:
            with self.subTest(identity=row['id']):
                plan=copy.deepcopy(base)
                if row['kind']=='module':
                    plan['modules'].append({'module_id':'tconstruct-'+row['id'],
                        'file':row['file'],'name':row['name'],'mod_id':'tconstruct'})
                else:
                    plan['injections'].append({'injection_id':'tconstruct-'+row['id'],
                        'file':'unresolved-static-fixture.rs','target':row['target'],'mod_id':'tconstruct'})
                with self.assertRaises(ValueError): GUARD.check(plan,workspace,meta=meta)

    def test_source_and_payload_drift_remain_rejected(self):
        workspace,meta,plan=self.synthetic()
        for field in ('source','owner','input'):
            p=copy.deepcopy(plan)
            if field=='source':p['modules'][0]['source']+='\n'
            elif field=='owner':p['modules'][0]['mod_id']='foreign'
            else:p['inputs'][next(iter(p['inputs']))]='unreviewed'
            with self.assertRaises(ValueError):GUARD.check(p,workspace,meta=meta)

    def test_no_actual_plan_type_or_permission_claim_is_added(self):
        for key in ('actual_plan','actual_module_identities','host_type_result'):
            self.assertIsNone(REPORT[key])
        for key in ('runtime_changed','guard_algorithm_changed','guard_restrictions_relaxed'):
            self.assertFalse(REPORT[key])
        self.assertTrue(all(not r['transitive_runtime_verified'] for r in REPORT['conflicts']))

if __name__=='__main__':unittest.main()
