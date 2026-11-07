"""Pure source/inverse checks only. These do not compile or execute Rust."""
import hashlib
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
API = 'pluto-lib/mixins/pluto_native_creative/api/src/fluid.rs'
DECL = 'pluto-lib/mixins/pluto_native_creative/api/src/fluid/declaration.rs'
HOST = 'pluto-lib/mixins/pluto_native_creative/world-fluid.rs'
SLOPE = 'pluto-lib/mixins/pluto_native_creative/api/src/fluid/slope.rs'
TC = 'tconstruct/experiments/world-fluid-slope-candidate'

def read(path):
    return (ROOT / path).read_text(encoding='utf-8')

def restore(after, edits):
    for edit in reversed(edits):
        if after.count(edit['after']) != 1:
            raise ValueError('inverse requires one exact replacement')
        after = after.replace(edit['after'], edit['before'])
    return after.encode('utf-8')

class Source(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.proof = json.loads(read('pluto-lib/tests/fluid-slope-inverse.json'))

    def test_complete_inverse_of_all_three_existing_payloads(self):
        self.assertEqual(set(self.proof['files']), {API, DECL, HOST})
        for path, row in self.proof['files'].items():
            data = read(path).encode()
            self.assertEqual(hashlib.sha256(data).hexdigest(), row['after_sha256'])
            before = restore(data.decode(), row['edits'])
            self.assertEqual(hashlib.sha256(before).hexdigest(), row['before_sha256'])
            self.assertEqual(hashlib.sha1(b'blob '+str(len(before)).encode()+b'\0'+before).hexdigest(), row['before_git_blob'])

    def test_inverse_rejects_changed_or_duplicate_payload(self):
        for path, row in self.proof['files'].items():
            data = read(path)
            with self.assertRaises(ValueError): restore(data + data, row['edits'])
            changed = data.replace(row['edits'][-1]['after'], 'changed', 1)
            with self.assertRaises(ValueError): restore(changed, row['edits'])

    def test_definition_and_generation_contracts_have_no_delta(self):
        row = self.proof['files'][API]
        before = restore(read(API), row['edits']).decode()
        after = read(API)
        for start, end in [('pub struct Definition {','/// Display facts'), ('impl Definition {','/// Opaque, monotonic'), ('pub fn content(def:', '#[cfg(test)]')]:
            self.assertEqual(before.split(start,1)[1].split(end,1)[0], after.split(start,1)[1].split(end,1)[0])

    def test_one_registry_owns_policy_and_revision(self):
        source = read(API)
        self.assertEqual(source.count('static REGISTRY:'), 1)
        self.assertIn('let mut slopes = all.slopes.clone();', source)
        self.assertIn('all.slopes = slopes;', source)
        self.assertIn('registry.slopes.clone(),', source)
        self.assertIn('fluid already has another slope policy', source)
        self.assertIn('fluid is not registered', source)
        self.assertNotIn('static ', read(SLOPE))

    def test_legacy_paths_still_select_no_policy(self):
        source = read(API)
        self.assertIn('tick_inner(world, pos, definition, None);', source)
        self.assertIn('None => [true; 4]', source)
        self.assertIn('entries.into_iter().map(|(d, r)| (d, r, None))', source)
        self.assertIn('slope_distance: None', read(DECL))

    def test_native_adapter_keeps_permissions_and_scheduling(self):
        row = self.proof['files'][HOST]
        self.assertEqual(len(row['edits']), 1)
        self.assertIn('fluid::registered_slope_distance(&d.id)', read(HOST))
        self.assertIn('Some(distance) => fluid::tick_with_slope', read(HOST))
        self.assertIn('None => fluid::tick(', read(HOST))
        before = restore(read(HOST), row['edits']).decode()
        self.assertEqual(before.split('impl World for Adapter',1)[1].split('/// Claim only',1)[0], read(HOST).split('impl World for Adapter',1)[1].split('/// Claim only',1)[0])
        self.assertEqual(before.split('pub(super) fn neighbor',1)[1], read(HOST).split('pub(super) fn neighbor',1)[1])

    def test_recursion_and_offsets_are_explicitly_bounded(self):
        source = read(SLOPE)
        self.assertIn('if !(1..=4).contains(&distance)', source)
        self.assertIn('if pass < limit', source)
        self.assertIn('pass + 1, index ^ 1, limit', source)
        self.assertEqual(4*sum(3**n for n in range(5))*2, 968)
        self.assertNotIn('unsafe', source)
        self.assertNotIn('world.replace', source.split('#[cfg(test)]')[0])

    def test_fourteen_rust_cases_are_prepared_not_reported_executed(self):
        names = re.findall(r'#\[test\]\s*fn (\w+)', read(SLOPE))
        self.assertEqual(len(names), 14)
        self.assertEqual(len(set(names)), 14)
        self.assertEqual(names, self.proof['prepared_rust_tests'])
        self.assertEqual(self.proof['rust_execution'], 'NOT RUN')

    def test_tc_candidate_is_excluded_and_preserves_unimplemented_facts(self):
        data = json.loads(read(TC+'/families.json'))
        self.assertEqual(data['source_commit'], 'd82b7664cfda55a8a8e850e97808c77dcdbcb732')
        self.assertEqual([e['name'] for e in data['entries']], ['molten_iron','molten_gold','molten_copper','molten_cobalt'])
        for e in data['entries']:
            v=e['source']
            self.assertEqual(v['policy'], {'drop_off':3,'slope_distance':3,'tick_delay':30,'source_conversion':False})
            self.assertEqual(v['block'], 'tconstruct:'+e['name']+'_fluid')
            self.assertTrue(v['interaction']['burn']['seconds'] > 0)
            self.assertIn('fog', v['visual']); self.assertIn('camera', v['visual'])
            self.assertEqual(v['path_type'], 'lava')
        candidate = read(TC+'/candidate.rs')
        self.assertNotIn('register_declarations(', candidate)
        self.assertNotIn('register_all(', candidate)
        self.assertIn('EXCLUDED candidate', candidate)

    def test_all_twenty_three_prs_remain_individually_not_completed(self):
        ledger=json.loads(read('pluto-lib/docs/world-fluid-intake-20261107.json'))
        self.assertEqual(len(ledger['prs']), 23)
        self.assertEqual(len({r['number'] for r in ledger['prs']}),23)
        self.assertTrue(all(not r['feature_complete'] for r in ledger['prs']))
        self.assertEqual(ledger['ordinary_tc_activation'], False)

if __name__ == '__main__':
    unittest.main()
