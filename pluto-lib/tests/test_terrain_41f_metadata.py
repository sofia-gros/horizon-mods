"""Actual selected IR replay and negatives; never a full plan/apply/type proof."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = Path(os.environ.get('PLUTO_TERRAIN_RUNTIME_ROOT', ROOT))
NATIVE = RUNTIME / 'mixins/pluto_native_creative'
REVIEW = json.loads((ROOT / 'docs/terrain-41f-guard-review.json').read_bytes())
sha = lambda b: hashlib.sha256(b).hexdigest()
META = {k: json.loads((ROOT / ('tools/terrain-' + k + '-source.json')).read_bytes())
        for k in ('binding', 'cell')}
GUARDS = {}
for kind in META:
    path = RUNTIME / ('tools/check_terrain_' + kind + '_plan.py')
    spec = importlib.util.spec_from_file_location('terrain_41f_' + kind, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    GUARDS[kind] = module


def old(kind):
    result = copy.deepcopy(META[kind])
    for change in REVIEW['changes'][kind]:
        entry = result
        for key in change['path'][:-1]:
            entry = entry[key]
        entry[change['path'][-1]] = change['before']
    return result


def selected_projection():
    # Rehydrate ONLY retained selected nodes with hash-verified exact payloads.
    # No missing full-plan nodes are invented; this cannot certify all12 guards.
    w = REVIEW['witness']
    assert not w['competitors'] and w['status'] == 'observed_only'
    result = {'schema': w['plan']['schema'], 'inputs': copy.deepcopy(w['selected_official_source_sha256']),
              'injections': copy.deepcopy(w['hooks']), 'modules': copy.deepcopy(w['modules'])}
    for hook in result['injections']:
        meta = next(m for m in META.values() if m['target'] == hook['target'])
        name = meta.get('helper', 'terrain-cell-head.rs')
        raw = (NATIVE / name).read_bytes()
        assert sha(raw) == hook['payload']['sha256'] and len(raw) == hook['payload']['bytes']
        hook['helper_source'] = raw.decode('utf-8')
    for module in result['modules']:
        source = next(m for meta in META.values() for m in meta['modules']
                      if 'pluto_native_creative-' + m['module_id'] == module['module_id'])
        raw = (NATIVE / source['source']).read_bytes()
        assert sha(raw) == module['payload']['sha256'] and len(raw) == module['payload']['bytes']
        module['source'] = raw.decode('utf-8')
    return result


class Terrain41fMetadata(unittest.TestCase):
    def test_guard_programs_unchanged(self):
        for name, digest in REVIEW['unchanged_checkers'].items():
            self.assertEqual(sha((RUNTIME / 'tools' / name).read_bytes()), digest)

    def test_exact_eight_leaf_delta_and_whole_inverse(self):
        self.assertEqual([len(REVIEW['changes'][k]) for k in META], [7, 1])
        self.assertEqual(META['binding']['location']['node'], [398, 2])
        self.assertEqual(META['binding']['modules'][2]['fingerprint'],
                         '540e1e6e0eacd338678a6e6a981a5496eabb927cbe4a20d7713830339677b042')
        for kind in META:
            raw = (json.dumps(old(kind), indent=2) + '\n').encode()
            self.assertEqual(sha(raw), REVIEW['metadata_sha256'][kind]['before'])
            self.assertEqual(sha((ROOT / ('tools/terrain-' + kind + '-source.json')).read_bytes()),
                             REVIEW['metadata_sha256'][kind]['after'])

    def test_actual_selected_projection(self):
        plan = selected_projection()
        self.assertEqual((len(plan['injections']), len(plan['modules'])), (2, 6))
        for kind, guard in GUARDS.items():
            guard.check(plan, META[kind])

    def test_historical_metadata_still_rejects_actual_projection(self):
        for kind, message in [('binding', 'Assembly function changed'), ('cell', 'streaming.rs')]:
            with self.assertRaisesRegex(ValueError, message):
                GUARDS[kind].check(selected_projection(), old(kind))

    def test_old_locator_or_module_placement_rejected(self):
        for change in [lambda p: p['injections'][0].update(node=[386, 2]),
                       lambda p: p['modules'][2].update(fingerprint=old('binding')['modules'][2]['fingerprint'])]:
            plan = selected_projection(); change(plan)
            with self.assertRaises(ValueError): GUARDS['binding'].check(plan, META['binding'])

    def test_assembly_alias_and_duplicate_hooks_rejected(self):
        for kind in ('HEAD', 'RETURN', 'AFTER_CALL', 'WRAP_OPERATION'):
            plan = selected_projection(); extra = copy.deepcopy(plan['injections'][0])
            extra.update(target='alias::assemble', kind=kind, mod_id='foreign')
            plan['injections'].append(extra)
            with self.assertRaises(ValueError): GUARDS['binding'].check(plan, META['binding'])

    def test_all_mesher_file_hooks_still_reserved(self):
        for kind in ('HEAD', 'RETURN', 'AFTER_CALL', 'WRAP_OPERATION', 'MODIFY_VARIABLE'):
            plan = selected_projection(); extra = copy.deepcopy(plan['injections'][1])
            extra.update(target='alias::face', kind=kind, node=[76, 2],
                         function_fingerprint='different', mod_id='tconstruct')
            plan['injections'].append(extra)
            with self.assertRaises(ValueError): GUARDS['cell'].check(plan, META['cell'])

    def test_module_owner_missing_duplicate_and_source_rejected(self):
        for kind, guard in GUARDS.items():
            for mutation in ('owner', 'missing', 'duplicate', 'source'):
                plan = selected_projection()
                if mutation == 'owner': plan['modules'][0]['mod_id'] = 'foreign'
                if mutation == 'missing': plan['modules'].pop(0)
                if mutation == 'duplicate': plan['modules'].append(copy.deepcopy(plan['modules'][0]))
                if mutation == 'source': plan['modules'][0]['source'] += '// drift\n'
                with self.assertRaises(ValueError): guard.check(plan, META[kind])

    def test_contract_bindings_priority_cancellation_helper_rejected(self):
        for kind, index in [('binding', 0), ('cell', 1)]:
            changes = [{'bindings': []}, {'cancellable': not selected_projection()['injections'][index]['cancellable']},
                       {'helper_source': 'changed'}, {'mod_id': 'foreign'}, {'function_fingerprint': 'changed'}]
            if kind == 'cell': changes += [{'priority': 0}]
            for change in changes:
                plan = selected_projection(); plan['injections'][index].update(change)
                with self.assertRaises(ValueError): GUARDS[kind].check(plan, META[kind])

    def test_every_official_input_remains_exact(self):
        for kind, guard in GUARDS.items():
            for path in META[kind]['source_sha256']:
                plan = selected_projection(); plan['inputs'][path] = '0' * 64
                with self.assertRaises(ValueError): guard.check(plan, META[kind])

    def test_native_mesher_body_hunks_and_face_filter_unchanged(self):
        before = old('cell'); current = META['cell']
        for key in ('file_sha256', 'function_sha256', 'start', 'end', 'changes', 'face_start', 'face_end',
                    'target', 'function_fingerprint', 'bindings', 'location', 'modules', 'helper_sha256'):
            self.assertEqual(current[key], before[key])
        self.assertEqual(current['function_sha256'], '5cbeb23de61b6cf8092a436c7eb2cea410fb9fba7b71c5fa08669d30c54b739f')

    def test_observation_provenance_and_unexecuted_scope(self):
        self.assertEqual(REVIEW['witness']['plan']['sha256'],
                         '7139236d6484c9cf5a862d3e8358d60b2b4e0bd3a8c4c0968c6d313735670f8b')
        self.assertEqual(REVIEW['witness']['plan']['bytes'], 6383021)
        self.assertEqual(REVIEW['witness']['competitors'], [])
        self.assertIsNone(REVIEW['witness']['compatibility'])
        self.assertFalse(REVIEW['validation']['full_updated_guard_execution'])
        self.assertFalse(any(REVIEW['validation'][k] for k in ('native', 'apply', 'type', 'GPU')))


if __name__ == '__main__': unittest.main()
