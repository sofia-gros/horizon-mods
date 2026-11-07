"""Pure checker regressions. Synthetic IR is not an observed candidate plan."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
from cloud_composition_contracts import canonical, projection, validate_contracts, validate_entries
from check_cloud_insertion_plan import check

FINGERPRINT = hashlib.sha256(b'SYNTHETIC TEST IDENTITY, NOT AN ACTUAL PLAN').hexdigest()


def synthetic():
    c = json.loads((ROOT / 'tools/cloud-composition-contracts.json').read_text())
    c['candidate_observation'] = {
        'runtime_commit': c['candidate_runtime_commit'],
        'plan_sha256': FINGERPRINT,
        'evidence_commit': 'e' * 40,
    }
    p = {'schema': 6, 'modules': [], 'injections': [], 'states': []}
    for kind in ('modules', 'injections', 'states'):
        for row in c[kind]:
            if row.get('fingerprint', False) is None:
                row['fingerprint'] = FINGERPRINT
            entry = copy.deepcopy(row)
            body_key = {'modules': 'source', 'injections': 'helper_source'}.get(kind)
            if body_key:
                # Model bodies exercise integrity logic only. This fixture does
                # not pretend to compile or reproduce native/source semantics.
                text = '// synthetic ' + str(row.get('module_id', row.get('injection_id')))
                row[body_key + '_sha256'] = hashlib.sha256(text.encode()).hexdigest()
                entry.pop(body_key + '_sha256')
                entry[body_key] = text
            p[kind].append(entry)
    by_id={row['injection_id']:row for row in p['injections']}
    p['injections']=[by_id[ident] for group in c['hook_order'] for ident in group['injection_ids']]
    return p, c


class Contracts(unittest.TestCase):
    def test_same_priority_hook_reordering_rejects(self):
        p,c=synthetic()
        group=next(g for g in c['hook_order'] if 'source-splashing-block' in g['injection_ids'])
        self.assertEqual(group['injection_ids'],['source-splashing-block','source-transform-use'])
        indexes=[i for i,r in enumerate(p['injections']) if r['injection_id'] in group['injection_ids']]
        a,b=indexes; p['injections'][a],p['injections'][b]=p['injections'][b],p['injections'][a]
        with self.assertRaisesRegex(ValueError,'execution order'): validate_contracts(p,c)

    def test_order_contract_cannot_omit_duplicate_or_merge_sites(self):
        for mutation in ['omit','repeat','wrong_node']:
            p,c=synthetic()
            if mutation=='omit':c['hook_order'][0]['injection_ids'].pop()
            elif mutation=='repeat':c['hook_order'].append(copy.deepcopy(c['hook_order'][0]))
            else:c['hook_order'][0]['node']=[999]
            with self.assertRaises(ValueError):validate_contracts(p,c)

    def test_old_checker_and_metadata_inverse_are_exact(self):
        delta=json.loads((ROOT/'tests/cloud-composition-guard-delta.json').read_text())
        for name, spec in delta['originals'].items():
            raw=(ROOT/name).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(),spec['new_sha256'])
            if name.endswith('.py'):
                text=raw.decode()
                for edit in reversed(delta['text_changes'][name]):
                    at=edit['new_start']; after=edit['after']
                    self.assertEqual(text[at:at+len(after)],after)
                    text=text[:at]+edit['before']+text[at+len(after):]
            else:
                value=json.loads(raw)
                for edit in reversed(delta['metadata_changes']):
                    obj=value
                    for key in edit['path'][:-1]:obj=obj[key]
                    key=edit['path'][-1]
                    self.assertEqual(obj[key],edit['after'])
                    if edit['before'] is None:del obj[key]
                    else:obj[key]=edit['before']
                text=json.dumps(value,indent=2)+'\n'
            self.assertEqual(hashlib.sha256(text.encode()).hexdigest(),spec['sha256'])

    def integrated(self, mutation=None):
        p,c=synthetic()
        m=json.loads((ROOT/'tools/cloud-insertion-boundaries.json').read_text())
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp); (root/'source.rs').write_bytes(b'official input model')
            h=hashlib.sha256((root/'source.rs').read_bytes()).hexdigest()
            m['official_source_sha256']={'source.rs':h}; p['inputs']={'source.rs':h}
            for row in m['modules']:
                actual=next(e for e in p['modules'] if e['module_id']==row['module_id'])
                body=actual['source'].encode(); (root/row['source_file']).write_bytes(body)
                row['source_sha256']=hashlib.sha256(body).hexdigest()
            if mutation: mutation(p,c,m,root)
            check(p,root,meta=m,helpers=root,composition=c)

    def test_synthetic_complete_checker_accepts_exact_contracts(self):
        self.integrated()

    def test_unknown_module_in_original_exclusive_file_still_rejects(self):
        def mutate(p,c,m,r):
            row=copy.deepcopy(p['modules'][0]); row.update(module_id='extra',name='extra',
                file='crates/horizon-client/src/game/potions.rs');p['modules'].append(row)
        with self.assertRaisesRegex(ValueError,'Unreviewed module'): self.integrated(mutate)

    def test_unknown_original_exclusive_hook_still_rejects(self):
        def mutate(p,c,m,r):
            row=copy.deepcopy(p['injections'][0]); row.update(injection_id='extra',target='other',node=[999],
                file='crates/horizon-client/src/game/potions.rs');p['injections'].append(row)
        with self.assertRaisesRegex(ValueError,'reserved cloud'): self.integrated(mutate)

    def test_unknown_game_state_and_constructor_still_reject(self):
        for constructor in [False,True]:
            def mutate(p,c,m,r):
                row=copy.deepcopy(p['states'][0]);row.update(state_id='extra',name='extra',target='other')
                if constructor:row['constructors']=[{'file':'crates/horizon-client/src/game.rs'}]
                else:row['file']='crates/horizon-client/src/game.rs'
                p['states'].append(row)
            with self.assertRaisesRegex(ValueError,'reserved cloud'):self.integrated(mutate)

    def test_official_input_and_plan_hash_binding_still_reject(self):
        def file_change(p,c,m,r): (r/'source.rs').write_bytes(b'changed')
        def plan_change(p,c,m,r): p['inputs']['source.rs']='0'*64
        for mutate in [file_change,plan_change]:
            with self.assertRaises(ValueError):self.integrated(mutate)

    def test_helper_file_mismatch_still_rejects(self):
        def mutate(p,c,m,r): (r/m['modules'][0]['source_file']).write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'Reviewed cloud source'):self.integrated(mutate)

    def test_synthetic_exact_entries_accept(self):
        p, c = synthetic()
        validate_contracts(p, c)

    def test_published_pending_placement_is_not_a_wildcard(self):
        c = json.loads((ROOT / 'tools/cloud-composition-contracts.json').read_text())
        delta = json.loads((ROOT / 'tests/cloud-plan-observation-delta.json').read_text())
        self.assertEqual(hashlib.sha256((ROOT / 'tools/cloud-composition-contracts.json').read_bytes()).hexdigest(), delta['observed_contract_sha256'])
        for edit in reversed(delta['changes']):
            obj = c
            for key in edit['path'][:-1]: obj = obj[key]
            self.assertEqual(obj[edit['path'][-1]], edit['after'])
            obj[edit['path'][-1]] = edit['before']
        prior = (json.dumps(c, indent=2, ensure_ascii=True) + '\n').encode()
        self.assertEqual(hashlib.sha256(prior).hexdigest(), '19f1c7a36f50f12805caeaaf99c2a3e33f8e273abc619f696fa52b5e648a70c8')
        self.assertIsNone(c['candidate_observation'])
        row = next(r for r in c['modules'] if r['module_id'] == 'source-fluid-cloud')
        self.assertIsNone(row['fingerprint'])
        with self.assertRaisesRegex(ValueError, 'actual-plan observation'):
            validate_contracts(synthetic()[0], c)

    def test_actual_observation_is_only_four_metadata_leaves(self):
        delta = json.loads((ROOT / 'tests/cloud-plan-observation-delta.json').read_text())
        c = json.loads((ROOT / 'tools/cloud-composition-contracts.json').read_text())
        index = next(i for i, row in enumerate(c['modules']) if row['module_id'] == 'source-fluid-cloud')
        self.assertEqual([e['path'] for e in delta['changes']], [
            ['candidate_runtime_commit'], ['candidate_observation'], ['notes'], ['modules', index, 'fingerprint']])
        self.assertEqual(c['candidate_runtime_commit'], 'e9073648b3c4cf4d49f27bc2e1ed60997f430586')
        self.assertEqual(c['candidate_observation']['plan_sha256'], 'dc4fbc3c54144f2f95728986b4e9bfb3700235655b18a3af3c8d1a6113dcb86b')
        self.assertEqual(c['modules'][index]['fingerprint'], '540e1e6e0eacd338678a6e6a981a5496eabb927cbe4a20d7713830339677b042')
        self.assertEqual([len(c[k]) for k in ('modules', 'injections', 'states', 'hook_order')], [19, 27, 45, 16])
        for name, digest in delta['unchanged_file_sha256'].items():
            self.assertEqual(hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), digest, name)
        for key in ('native_type_verified', 'game_tests_executed', 'gameplay_verified'):
            self.assertIs(c['candidate_observation'][key], False)

    def test_unobserved_fingerprint_still_rejects_with_receipt_fields(self):
        p, c = synthetic()
        c['modules'][0]['fingerprint'] = None
        with self.assertRaisesRegex(ValueError, 'Unobserved'):
            validate_contracts(p, c)

    def test_wrong_runtime_observation_rejects(self):
        p, c = synthetic()
        c['candidate_observation']['runtime_commit'] = c['candidate']
        with self.assertRaises(ValueError): validate_contracts(p, c)

    def test_all_required_entries_reject_removal_and_duplication(self):
        for kind in ('modules', 'injections', 'states'):
            p, c = synthetic()
            for i in range(len(p[kind])):
                q = copy.deepcopy(p); q[kind].pop(i)
                with self.assertRaises(ValueError): validate_contracts(q, c)
                q = copy.deepcopy(p); q[kind].append(copy.deepcopy(q[kind][i]))
                with self.assertRaises(ValueError): validate_contracts(q, c)

    def test_every_payload_byte_is_bound(self):
        for kind, key in [('modules', 'source'), ('injections', 'helper_source')]:
            p, c = synthetic()
            for i in range(len(p[kind])):
                q = copy.deepcopy(p); q[kind][i][key] += '\n'
                with self.assertRaises(ValueError): validate_contracts(q, c)

    def test_body_hash_assertion_cannot_replace_real_source(self):
        p, c = synthetic()
        r = p['modules'][0]; r['source_sha256'] = c['modules'][0]['source_sha256']; r.pop('source')
        with self.assertRaises(ValueError): validate_contracts(p, c)

    def test_owner_visibility_and_placement_are_bound(self):
        for key in ('mod_id','mod_version','file','name','visibility','fingerprint'):
            p, c = synthetic(); p['modules'][0][key] = 'changed'
            with self.assertRaises(ValueError): validate_contracts(p, c)

    def test_implicit_default_visibility_only_is_equivalent(self):
        p, c = synthetic()
        for r in p['modules']:
            if r['visibility'] == 'pub(crate)': r.pop('visibility')
        validate_contracts(p, c)

    def test_no_consumer_slot_alias(self):
        p, c = synthetic()
        row = next(r for r in p['modules'] if r['module_id'] == 'source-fluid-cloud')
        extra = copy.deepcopy(row); extra['module_id'] = 'another'; p['modules'].append(extra)
        with self.assertRaises(ValueError): validate_contracts(p, c)

    def test_full_hook_semantics_are_bound(self):
        for key, value in [('priority',99),('node',[0]),('kind','RETURN'),('cancellable',True),
                           ('bindings',[]),('contract',{}),('ordinal',1),('require',1),
                           ('selector','other'),('function_fingerprint','bad')]:
            p, c = synthetic()
            row = next(r for r in p['injections'] if not r['cancellable'])
            row[key] = value
            with self.assertRaises(ValueError, msg=key): validate_contracts(p, c)

    def test_extra_return_on_known_target_rejects(self):
        p, c = synthetic()
        row = copy.deepcopy(p['injections'][0]); row['injection_id']='new'; row['kind']='RETURN'
        row['node']=[999]; p['injections'].append(row)
        with self.assertRaisesRegex(ValueError, 'Unreviewed hook'): validate_contracts(p, c)

    def test_renamed_target_on_same_node_rejects(self):
        p, c = synthetic()
        row = copy.deepcopy(p['injections'][0]); row['injection_id']='new'; row['target']='other'
        p['injections'].append(row)
        with self.assertRaises(ValueError): validate_contracts(p, c)

    def test_new_atomic_allocator_hook_rejects(self):
        p, c = synthetic()
        row = copy.deepcopy(p['injections'][0]); row.update(injection_id='new', node=[999],
            target="horizon_client::game::lan_entities::Game<'_>::take_entity_id")
        p['injections'].append(row)
        with self.assertRaises(ValueError): validate_contracts(p, c)

    def test_module_interior_hook_rejects(self):
        p, c = synthetic()
        row = copy.deepcopy(p['injections'][0]); row.update(injection_id='new', node=[999],
            target='horizon_client::game::pluto_cloud_insertion::insert_native_item')
        p['injections'].append(row)
        with self.assertRaises(ValueError): validate_contracts(p, c)

    def test_unrelated_hook_is_not_a_global_composition_freeze(self):
        p, c = synthetic()
        row = copy.deepcopy(p['injections'][0]); row.update(injection_id='unrelated', node=[999],
            target='horizon_client::unrelated::draw', file='unrelated.rs')
        p['injections'].append(row)
        validate_contracts(p, c)

    def test_state_type_initial_owner_constructor_are_bound(self):
        for key, value in [('ty','u32'),('initial','0'),('mod_id','other'),
                           ('constructors',[]),('name','other'),('target','other')]:
            p,c=synthetic(); p['states'][0][key]=value
            with self.assertRaises(ValueError): validate_contracts(p,c)

    def test_boolean_integer_and_extra_ir_fields_do_not_alias(self):
        p,c=synthetic(); p['injections'][0]['cancellable']=int(p['injections'][0]['cancellable'])
        with self.assertRaises(ValueError): validate_contracts(p,c)
        p,c=synthetic(); p['modules'][0]['future_rewrite']=True
        with self.assertRaises(ValueError): validate_contracts(p,c)

    def test_contract_cannot_repeat_identity_to_reduce_coverage(self):
        p,c=synthetic(); c['modules'].append(copy.deepcopy(c['modules'][0]))
        with self.assertRaises(ValueError): validate_contracts(p,c)

    def test_old_guard_reserved_files_states_and_source_pins_remain(self):
        text=(ROOT/'tools/check_cloud_insertion_plan.py').read_text()
        for required in ["plan.get('inputs', {}).get(relative) != expected",
                "helper = (helpers / required['source_file']).read_bytes()",
                "entry.get('file') in exclusive and slot not in expected_slots",
                "entry.get('file') in protected_states", "entry.get('constructors', [])"]:
            self.assertIn(required,text)
        self.assertIn("reviewed['allows_hook'](entry) is not True",text)


if __name__ == '__main__': unittest.main()
