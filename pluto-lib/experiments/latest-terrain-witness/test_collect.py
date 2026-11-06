#!/usr/bin/env python3
"""Synthetic-plan negative tests using the eight actual pinned Rust payloads.

These tests do not run or simulate the official planner/renderer.
"""
import copy
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('collector', HERE / 'collect.py')
C = importlib.util.module_from_spec(spec)
spec.loader.exec_module(C)
CONTRACT = C.decode((HERE / 'contract.json').read_bytes())
ROOT = Path(os.environ['PLUTO_WITNESS_MOD_ROOT']) if 'PLUTO_WITNESS_MOD_ROOT' in os.environ else HERE.parents[2]
SOURCES = {}
for entry in CONTRACT['hooks'] + CONTRACT['modules']:
    name = entry.get('helper', entry.get('source'))
    raw = C.regular(ROOT / 'pluto-lib/mixins/pluto_native_creative' / name, C.SOURCE_BYTES)
    if C.sha(raw) != entry['payload_sha256']:
        raise ValueError('actual pinned payload mismatch: ' + name)
    SOURCES[name] = raw.decode()


def plan():
    hooks = []
    for i, expected in enumerate(CONTRACT['hooks']):
        row = {k: copy.deepcopy(v) for k, v in expected.items() if k not in ('helper', 'payload_sha256')}
        row.update(node=[900+i, 3], function_fingerprint=str(i+1)*64,
                   helper_source=SOURCES[expected['helper']])
        hooks.append(row)
    modules = []
    for expected in CONTRACT['modules']:
        row = {k: v for k, v in expected.items() if k not in ('source', 'payload_sha256')}
        row.update(fingerprint='a'*64, source=SOURCES[expected['source']])
        modules.append(row)
    return {'schema': 6, 'transformer': 'synthetic-schema-test', 'rustc': 'NOT AN ACTUAL PLAN',
            'inputs': copy.deepcopy(CONTRACT['official_source_sha256']),
            'injections': hooks, 'modules': modules}


def encoded(value):
    return json.dumps(value).encode()


class WitnessTests(unittest.TestCase):
    def check(self, value):
        return C.collect(encoded(value), CONTRACT, SOURCES)

    def rejected(self, value):
        with self.assertRaises((ValueError, KeyError, TypeError)):
            self.check(value)

    def test_exact_payloads_and_observed_new_locations(self):
        p = plan(); result = self.check(p)
        self.assertEqual((len(result['hooks']), len(result['modules'])), (2, 6))
        self.assertEqual(result['hooks'][0]['node'], [900, 3])
        self.assertEqual(result['modules'][0]['fingerprint'], 'a'*64)
        self.assertEqual(result['plan']['sha256'], C.sha(encoded(p)))
        self.assertIsNone(result['compatibility'])
        self.assertFalse(result['native_executed'])
        self.assertEqual(result['status'], 'observed_only')
        self.assertNotIn('helper_source', result['hooks'][0])
        self.assertNotIn('source', result['modules'][0])
        p['injections'][0]['argument'] = None
        self.assertEqual(self.check(p)['hooks'][0]['site'], {'argument': None})

    def test_missing_hook_or_module(self):
        for field in ('injections', 'modules'):
            p = plan(); p[field].pop(); self.rejected(p)

    def test_duplicate_hook_id_or_module_destination(self):
        for field in ('injections', 'modules'):
            p = plan(); p[field].append(copy.deepcopy(p[field][0])); self.rejected(p)

    def test_reused_module_id_elsewhere_is_ambiguous(self):
        p = plan(); row = copy.deepcopy(p['modules'][0]); row['name'] += '_other'
        p['modules'].append(row); self.rejected(p)

    def test_wrong_selected_owner_or_id(self):
        for field, key in (('injections', 'mod_id'), ('modules', 'mod_id'),
                           ('injections', 'injection_id'), ('modules', 'module_id')):
            p = plan(); p[field][0][key] = 'foreign'; self.rejected(p)

    def test_actual_hook_or_module_payload_drift(self):
        for field, key in (('injections', 'helper_source'), ('modules', 'source')):
            p = plan(); p[field][0][key] += '\n'; self.rejected(p)

    def test_original_payload_table_cannot_hide_drift(self):
        sources = dict(SOURCES); path = CONTRACT['hooks'][0]['helper']; sources[path] += '\n'
        p = plan(); p['injections'][0]['helper_source'] = sources[path]
        with self.assertRaises(ValueError):
            C.collect(encoded(p), CONTRACT, sources)

    def test_bound_owner_phase_and_visibility_drift(self):
        for key, value in (('target', 'foreign::method'), ('file', 'other.rs'), ('priority', 0),
                           ('kind', 'RETURN'), ('cancellable', True), ('bindings', [])):
            p = plan(); p['injections'][0][key] = value; self.rejected(p)
        p = plan(); p['modules'][0]['visibility'] = 'pub(crate)'; self.rejected(p)
        for field in ('modules', 'injections'):
            p = plan(); p[field][0]['unknown_effective_field'] = True; self.rejected(p)

    def test_fingerprint_and_node_shape_rejected(self):
        for key, value in (('node', []), ('node', [True]), ('node', [-1]),
                           ('node', [1]*33), ('function_fingerprint', 'not-a-sha')):
            p = plan(); p['injections'][0][key] = value; self.rejected(p)

    def test_source_input_drift_or_missing(self):
        for mode in ('changed', 'missing'):
            p = plan(); path = next(iter(p['inputs']))
            if mode == 'changed': p['inputs'][path] = '0'*64
            else: del p['inputs'][path]
            self.rejected(p)

    def test_alias_and_whole_mesher_competitors_are_observations(self):
        p = plan()
        for i, kind in enumerate(('target', 'node', 'fingerprint', 'mesher')):
            row = copy.deepcopy(p['injections'][0])
            row.update(mod_id='other', injection_id='other-'+kind)
            if kind != 'target': row['target'] = 'different::spelling'
            if kind != 'node': row['node'] = [77+i]
            if kind != 'fingerprint': row['function_fingerprint'] = 'b'*64
            if kind == 'mesher': row['file'] = C.MESHER
            p['injections'].append(row)
        result = self.check(p)
        self.assertEqual(len(result['competitors']), 4)
        self.assertIsNone(result['compatibility'])
        self.assertEqual(result['competitors'][0]['mod_id'], 'other')

    def test_unrelated_hook_not_reported(self):
        p = plan(); row = copy.deepcopy(p['injections'][0])
        row.update(injection_id='unrelated', target='other', file='elsewhere.rs',
                   node=[1], function_fingerprint='f'*64)
        p['injections'].append(row)
        self.assertEqual(self.check(p)['competitors'], [])

    def test_plan_row_and_input_limits(self):
        with patch.object(C, 'MAX_ROWS', 5): self.rejected(plan())
        with patch.object(C, 'MAX_INPUTS', 12): self.rejected(plan())
        with patch.object(C, 'PLAN_BYTES', 10): self.rejected(plan())

    def test_competitor_and_output_limits(self):
        p = plan(); row = copy.deepcopy(p['injections'][0]); row['injection_id'] = 'competitor'
        p['injections'].append(row)
        with patch.object(C, 'MAX_COMPETITORS', 0): self.rejected(p)
        with patch.object(C, 'OUTPUT_BYTES', 10): self.rejected(plan())

    def test_duplicate_json_and_nonfinite_rejected(self):
        for raw in (b'{"schema":6,"schema":6}', b'{"value":NaN}', b'{"value":Infinity}'):
            with self.assertRaises(ValueError): C.decode(raw)

    def test_wrong_schema(self):
        p = plan(); p['schema'] = 7; self.rejected(p)

    def test_regular_limits_and_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); path = root/'input'; path.write_bytes(b'123')
            self.assertEqual(C.regular(path, 3), b'123')
            with self.assertRaises(ValueError): C.regular(path, 2)
            (root/'link').symlink_to(path)
            with self.assertRaises(ValueError): C.regular(root/'link', 3)
            with self.assertRaises(ValueError): C.regular(root, 10)

    def test_parent_symlink_and_unsafe_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root/'real').mkdir(); (root/'real/f').write_bytes(b'x')
            (root/'alias').symlink_to(root/'real', target_is_directory=True)
            with self.assertRaises(ValueError): C.regular(root/'alias/f', 10)
        for path in ('/root', '../a', 'a/../b', 'a//b', 'a\\b', './a'):
            with self.assertRaises(ValueError): C.relative(path)

    def test_selected_source_verification_rejects_mismatch(self):
        c = copy.deepcopy(CONTRACT)
        # The reader is modeled, not official bytes. Real payload bytes are still pinned above.
        with patch.object(C, 'regular', return_value=b'wrong'):
            with self.assertRaisesRegex(ValueError, 'official source mismatch'):
                C.verify_sources(c, '/unused', ROOT)

    def test_native_payload_source_mismatch_rejected(self):
        c = copy.deepcopy(CONTRACT)
        c['official_source_sha256'] = {k:C.sha(b'x') for k in c['official_source_sha256']}
        with patch.object(C, 'regular', return_value=b'x'):
            with self.assertRaisesRegex(ValueError, 'source payload mismatch'):
                C.verify_sources(c, '/unused', ROOT)


if __name__ == '__main__':
    unittest.main()
