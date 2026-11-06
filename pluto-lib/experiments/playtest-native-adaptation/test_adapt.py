import importlib.util, json, tomllib, unittest
from pathlib import Path
from unittest.mock import patch
HERE = Path(__file__).resolve().parent
s = importlib.util.spec_from_file_location('adapt', HERE/'adapt.py'); a = importlib.util.module_from_spec(s); s.loader.exec_module(a)

class Adaptation(unittest.TestCase):
    def test_exact_originals(self):
        before = a.originals(); self.assertEqual(len(before), 10)
        self.assertEqual({p:a.sha(b) for p,b in before.items()}, a.oracle()['source_sha256'])
    def test_base_inverse(self):
        before=a.originals(); after=a.transform_selected(before)
        self.assertEqual(a.restore_selected(after),before)
        self.assertEqual(len(after),12)
    def test_guide_inverse_and_preservation(self):
        before=a.originals(True); after=a.transform_selected(before,True)
        self.assertEqual(a.restore_selected(after,True),before)
        old=tomllib.loads(before[a.MANIFEST].decode()); new=tomllib.loads(after[a.MANIFEST].decode())
        self.assertEqual(old['module'],new['module'])
        self.assertEqual(len(old['module']),len(tomllib.loads(a.originals()[a.MANIFEST].decode())['module'])+2)
    def test_shared_and_store_remain_pr945(self):
        before=a.originals(); after=a.transform_selected(before)
        for f in ['source-mining-context-shared.rs','source-melter-store.rs']:
            self.assertEqual(before[a.TC+f],after[a.TC+f])
        self.assertIn(b'pluto_fluid_facts::player',after[a.TC+'source-mining-context-shared.rs'])
        self.assertIn(b'pluto_inventory::storage::stage_updates',after[a.TC+'source-melter-store.rs'])
    def test_items_return_preserved(self):
        before=a.originals(); after=a.transform_selected(before)
        self.assertEqual(before[a.TC+'source-mining-return.rs'],after[a.TC+'source-mining-return.rs'])
        old=tomllib.loads(before[a.MANIFEST].decode())['injection']; new=tomllib.loads(after[a.MANIFEST].decode())['injection']
        for h in old:
            if h.get('helper')=='source-mining-return.rs': self.assertIn(h,new)
    def test_single_new_head_without_old_locals(self):
        hs=tomllib.loads(a.transform_selected(a.originals())[a.MANIFEST].decode())['injection']
        self.assertEqual(sum(h['id']=='source-native-mining-context' for h in hs),1)
        self.assertFalse(any(h['id'] in ['source-host-mining-context','source-client-mining-context'] for h in hs))
        h=next(h for h in hs if h['id']=='source-native-mining-context')
        self.assertNotIn('original',str(h['bindings'])); self.assertEqual(h['kind'],'HEAD')
    def test_chest_entire_helper_and_hook_unchanged(self):
        before=a.originals(); after=a.transform_selected(before)
        self.assertEqual(before[a.CHEST],after[a.CHEST])
        get=lambda raw:next(h for h in tomllib.loads(raw.decode())['injection'] if h['id']=='tinkers-chest-pick-color')
        self.assertEqual(get(before[a.MANIFEST]),get(after[a.MANIFEST]))
        self.assertEqual(get(after[a.MANIFEST])['kind'],'RETURN')
    def test_common_owner_private_visibility(self):
        ms=tomllib.loads(a.transform_selected(a.originals())[a.COMMON].decode())['module']
        selected=[m for m in ms if m['name']=='pluto_mining_context']; self.assertEqual(len(selected),1)
        self.assertEqual(selected[0]['visibility'],'pub(crate)')
    def test_revisions_only_latest_not_mixins_header(self):
        before=a.originals(); after=a.transform_selected(before)
        for p in [a.MANIFEST,a.COMMON]:
            self.assertEqual(tomllib.loads(after[p].decode())['compatibility']['horizon_revision'],a.oracle()['official_revision'])
        self.assertEqual(set(a.receipt(before,after)['changed']),{a.MANIFEST,a.COMMON})
    def test_wrong_base_rejected(self):
        for guide in [False,True]:
            before=a.originals(guide); before[a.MANIFEST]+=b'\n'
            with self.assertRaises(ValueError): a.transform_selected(before,guide)
    def test_wrong_variant_rejected(self):
        with self.assertRaises(ValueError): a.transform_selected(a.originals(True),False)
        with self.assertRaises(ValueError): a.transform_selected(a.originals(),True)
    def test_missing_extra_or_policy_change_rejected(self):
        for kind in ['missing','extra','policy']:
            before=a.originals()
            if kind=='missing': del before[a.CHEST]
            elif kind=='extra': before['unreviewed.rs']=b''
            else: before[a.TC+'source-mining-context-shared.rs']+=b'\n'
            with self.assertRaises(ValueError): a.transform_selected(before)
    def test_candidate_drift_rejected(self):
        original=a.oracle(); original['candidate_sha256']['mining-context.rs']='0'*64
        with patch.object(a,'oracle',return_value=original):
            with self.assertRaises(ValueError): a.transform_selected(a.originals())
    def test_inverse_detects_changes(self):
        for path in [a.FACTS,a.CHEST,a.COMMON,a.TC+'source-mining-return.rs']:
            after=a.transform_selected(a.originals());after[path]+=b'\n'
            with self.assertRaises(ValueError): a.restore_selected(after)
    def test_repeat_rejected(self):
        with self.assertRaises(ValueError): a.transform_selected(a.transform_selected(a.originals()))
    def test_receipt_scope(self):
        for guide in [False,True]:
            before=a.originals(guide);r=a.receipt(before,a.transform_selected(before,guide),guide)
            self.assertTrue(r['selected_inverse_verified']); self.assertIsNone(r['compatibility'])
            for k in ['repository_sealed','package_sealed','plan_verified','native_type_verified']: self.assertFalse(r[k])
            self.assertEqual(len(r['changed']),2);self.assertEqual(len(r['added']),2)

if __name__=='__main__':unittest.main()
