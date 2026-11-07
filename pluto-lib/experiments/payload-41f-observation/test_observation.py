"""Read-only replay of saved actual plan evidence, never a planner or Game run."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
PLUTO = HERE.parents[1]
GUARD = PLUTO / 'tools/check_client_payload_plan.py'
spec = importlib.util.spec_from_file_location('actual_payload_guard', GUARD)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


class ObservedPayload(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = json.loads((HERE / 'evidence.json').read_bytes())
        cls.meta = json.loads((HERE / 'candidate/client-payload-source.json').read_bytes())
        cls.directory = Path(os.environ['GUIDE_PLAN_DIR'])
        cls.official = Path(os.environ['GUIDE_OFFICIAL_ROOT'])
        cls.index_raw = Path(os.environ['GUIDE_INDEX_RECEIPT']).read_bytes()
        cls.index = json.loads(cls.index_raw)
        cls.plan = json.loads((cls.directory / 'plan.json').read_bytes())

    def test_saved_artifact_and_index_receipt_identity(self):
        expected = self.evidence['artifacts']
        for name, row in expected.items():
            raw = (self.directory / name).read_bytes()
            self.assertEqual(len(raw), row['bytes'])
            self.assertEqual(digest(raw), row['sha256'])
        row = self.evidence['index_receipt']
        self.assertEqual(len(self.index_raw), row['bytes'])
        self.assertEqual(digest(self.index_raw), row['sha256'])
        self.assertTrue(self.index['completed'])
        self.assertTrue(self.index['source_verified'])
        self.assertIsNone(self.index['error'])
        item = self.index['index_receipt']
        self.assertTrue(item['completed'])
        self.assertTrue(item['compile']['passed'])
        self.assertTrue(item['observation']['passed'])
        self.assertIsNone(item['error'])
        result = item['observation']['result']
        reconstructed = (json.dumps(result, sort_keys=True, separators=(',', ':')) + '\n').encode()
        phase = [p for p in self.index['phases'] if p['name'] == 'index-observation']
        self.assertEqual(len(phase), 1)
        self.assertEqual(len(reconstructed), phase[0]['raw_bytes'])
        self.assertEqual(digest(reconstructed), phase[0]['raw_sha256'])
        self.assertEqual(digest(reconstructed), self.meta['candidate_observation']['index_result_sha256'])
        # Reconstructed bytes matching the phase hash are not claimed as a
        # separately downloaded raw stdout artifact.
        self.assertEqual(result, self.evidence['index_result'])

    def test_metadata_only_delta_and_unchanged_guard(self):
        self.assertEqual(digest(GUARD.read_bytes()), self.evidence['guard_sha256'])
        raw = (HERE / 'candidate/client-payload-source.json').read_bytes()
        self.assertEqual(digest(raw), self.evidence['metadata_after_sha256'])
        restored = copy.deepcopy(self.meta)
        restored['caller_fingerprints'][restored['callers'][1]] = None
        restored['candidate_observation'] = None
        restored['coverage'] = self.evidence['coverage_before']
        self.assertEqual(digest((json.dumps(restored, indent=2) + '\n').encode()),
                         self.evidence['metadata_before_sha256'])

    def test_actual_complete_plan_and_four_pristine_sources(self):
        guard.check(self.plan, self.meta)
        guard.check_source(self.official, self.meta)

    def test_old_cancellation_and_unknown_observer_or_module_still_reject(self):
        observer = next(h for h in self.plan['injections'] if h['target'] == self.meta['target'])
        update = next(m for m in self.plan['modules'] if m['name'] == self.meta['internal_updates']['name'])
        changes = []
        for name in ('broad-wire-receive', 'haste-wire-receive', 'materials-book-response'):
            def cancel(p, name=name):
                next(h for h in p['injections'] if h['injection_id'] == name)['cancellable'] = True
            changes.append(cancel)
        def duplicate_observer(p):
            row = copy.deepcopy(observer)
            row.update(mod_id='other', target='another::spelling')
            p['injections'].append(row)
        def duplicate_module(p):
            row = copy.deepcopy(update)
            row.update(mod_id='other', module_id='other')
            p['modules'].append(row)
        def drift_module(p):
            next(m for m in p['modules'] if m['name'] == update['name'])['source'] += '\n'
        changes.extend((duplicate_observer, duplicate_module, drift_module))
        for change in changes:
            with self.subTest(change=change.__name__):
                plan = copy.deepcopy(self.plan)
                change(plan)
                with self.assertRaises(ValueError):
                    guard.check(plan, self.meta)
        for field in ('candidate_observation', 'poll'):
            pending = copy.deepcopy(self.meta)
            if field == 'poll': pending['caller_fingerprints'][pending['callers'][1]] = None
            else: pending['candidate_observation'] = None
            with self.subTest(field=field), self.assertRaises(ValueError):
                guard.check(self.plan, pending)


if __name__ == '__main__':
    unittest.main()
