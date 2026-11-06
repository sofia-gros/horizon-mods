import importlib.util, json, tempfile, tomllib, unittest
from pathlib import Path
import candidate as c

class CandidateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        p = c.HERE.parent / 'latest-mining-context' / 'candidate.py'
        s = importlib.util.spec_from_file_location('preserved_mining', p)
        m = importlib.util.module_from_spec(s); s.loader.exec_module(m)
        cls.before = m.transform_manifest(m.revision_only_manifest())
        cls.old = c.oracle()['old_helper'].encode()

    def test_exact_base(self):
        self.assertEqual(c.sha(self.before), c.oracle()['base_manifest_sha256'])

    def test_manifest_inverse(self):
        after = c.transform_manifest(self.before); o = c.oracle()
        self.assertEqual(after.replace(o['new_hook'].encode(), o['old_hook'].encode(), 1), self.before)

    def test_only_one_hook_semantics(self):
        a = tomllib.loads(self.before.decode()); b = tomllib.loads(c.transform_manifest(self.before).decode())
        ai = a.pop('injection'); bi = b.pop('injection')
        self.assertEqual(a, b)
        self.assertEqual(len(ai), len(bi))
        changed = [(x, y) for x, y in zip(ai, bi) if x != y]
        self.assertEqual(len(changed), 1)
        old, new = changed[0]
        self.assertEqual(old['id'], c.HOOK)
        self.assertEqual(new['variable'], {'name':'stack', 'ordinal':0, 'at':'LOAD'})
        self.assertEqual((new['ordinal'], new['require']), (3, 4))
        self.assertEqual(new['kind'], 'MODIFY_VARIABLE')
        self.assertEqual(new['bindings'][0], {'name':'value', 'ty':'ItemStack', 'value':'$value'})
        self.assertEqual(old['bindings'][1:], new['bindings'][1:])
        self.assertEqual(old['target'], new['target'])

    def test_mining_owner_unchanged(self):
        def selected(data):
            m = tomllib.loads(data.decode())
            return [h for h in m['injection'] if 'mining' in h['id']]
        self.assertEqual(selected(self.before), selected(c.transform_manifest(self.before)))

    def test_manifest_drift_rejected(self):
        with self.assertRaises(ValueError): c.transform_manifest(self.before + b'\n')

    def test_second_application_rejected(self):
        with self.assertRaises(ValueError): c.transform_manifest(c.transform_manifest(self.before))

    def test_helper_inverse(self):
        out = c.transform_helper(self.old); o = c.oracle()
        back = out.replace(o['new_header'].encode(), o['old_header'].encode(), 1).replace(o['new_binding'].encode(), o['old_binding'].encode(), 1)
        self.assertEqual(back, self.old)
        self.assertEqual(c.sha(out), o['new_helper_sha256'])

    def test_policy_tail_byte_identical(self):
        out = c.transform_helper(self.old); anchor = b'    if &*stack.item'
        self.assertEqual(out[out.index(anchor):], self.old[self.old.index(anchor):])

    def test_exact_two_helper_lines(self):
        a = self.old.splitlines(); b = c.transform_helper(self.old).splitlines()
        self.assertEqual(len(a), len(b))
        self.assertEqual([i for i,(x,y) in enumerate(zip(a,b)) if x != y], [0,2])

    def test_no_new_clone_codec_or_authority(self):
        out = c.transform_helper(self.old)
        for token in [b'.clone()', b'encode_', b'decode_', b'unsafe', b'thread_local!', b'include_data', b'creative']:
            self.assertNotIn(token, out)
        self.assertEqual(out.count(b'.cloned()'), self.old.count(b'.cloned()'))

    def test_old_helper_drift_rejected(self):
        with self.assertRaises(ValueError): c.transform_helper(self.old.replace(b'Tag::Int', b'Tag::Long'))

    def test_candidate_helper_drift_rejected(self):
        old_here = c.HERE
        with tempfile.TemporaryDirectory() as td:
            p = Path(td)
            (p/'oracles.json').write_bytes((old_here/'oracles.json').read_bytes())
            (p/'source-chest-pick.rs').write_bytes((old_here/'source-chest-pick.rs').read_bytes()+b'\n')
            try:
                c.HERE = p
                with self.assertRaises(ValueError): c.transform_helper(self.old)
            finally: c.HERE = old_here

    def test_incomplete_payload_rejected(self):
        with self.assertRaises(ValueError): c.transform_payload({c.MANIFEST:self.before,c.HELPER:self.old})

    def test_wrong_payload_types_rejected(self):
        with self.assertRaises(ValueError): c.transform_payload({c.MANIFEST:self.before.decode()})

    def test_source_order_is_not_claimed_as_ir(self):
        o=c.oracle(); self.assertFalse(o['actual_ir_verified']); self.assertFalse(o['native_type_verified'])
        self.assertEqual([r['supported_candidate'] for r in o['static_loads']], [False,False,False,True])
        method=o['official_method'].encode()
        self.assertEqual(c.sha(method), o['official_method_sha256'])
        self.assertTrue(method.endswith(b'        Some(stack)\n    }'))
        self.assertLess(method.index(b'decode_components'), method.rindex(b'Some(stack)'))
        self.assertIn(b'let item = picked_item(self.res.behaviours(), state, item)?;', method)

    def test_official_drift_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)/'changed.rs';p.write_bytes(c.oracle()['official_method'].encode())
            with self.assertRaises(ValueError):c.verify_official(p)

if __name__ == '__main__': unittest.main()
