"""Execute structure composition policy with small synthetic plan inputs."""
from pathlib import Path
import copy
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('structure_plan', ROOT / 'tools/check_structure_plan.py')
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)


class StructurePlan(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.workspace = Path(self.temp.name) / 'official'
        self.helpers = Path(self.temp.name) / 'helpers'
        self.helpers.mkdir()
        self.meta = GUARD.metadata()
        # Synthetic AST fixtures have no production direct-source outputs.
        # Direct-proof policy is exercised separately with an explicit receipt.
        self.meta.pop('direct_attachments', None)
        self.meta.pop('direct_helpers', None)
        self.meta.pop('direct_review_status', None)
        for name in self.meta['official_source_sha256']:
            path = self.workspace / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(('synthetic native input ' + name + '\n').encode())
            self.meta['official_source_sha256'][name] = GUARD.digest(path.read_bytes())
        self.plan = {'schema': 6, 'inputs': dict(self.meta['official_source_sha256']),
                     'modules': [], 'injections': [], 'states': [], 'bridges': []}
        for key, fields, content_key in [
            ('modules', ('module_id', 'file', 'name', 'visibility'), 'source'),
            ('injections', ('injection_id', 'target', 'file', 'function_fingerprint', 'kind', 'cancellable', 'bindings'), 'helper_source'),
        ]:
            for required in self.meta[key]:
                content = ('synthetic package input ' + required['source_file'] + '\n').encode()
                (self.helpers / required['source_file']).write_bytes(content)
                required['source_sha256'] = GUARD.digest(content)
                entry = {field: copy.deepcopy(required[field]) for field in fields}
                if key == 'injections':
                    for field in ('selector', 'ordinal', 'require', 'site_fingerprint', 'variable', 'parameter'):
                        if field in required:
                            entry[field] = copy.deepcopy(required[field])
                entry.update(mod_id='pluto_lib')
                entry[content_key] = content.decode()
                self.plan[key].append(entry)

    def check(self, plan=None):
        GUARD.check(self.plan if plan is None else plan, self.workspace, self.meta, self.helpers)

    def test_exact_authority_and_package_bytes_pass_and_real_byte_drift_rejects(self):
        self.check()
        for relative in self.meta['official_source_sha256']:
            path = self.workspace / relative
            before = path.read_bytes()
            path.write_bytes(before + b'changed')
            with self.subTest(native=relative), self.assertRaises(ValueError):
                self.check()
            path.write_bytes(before)
            plan = copy.deepcopy(self.plan)
            plan['inputs'][relative] = 'different'
            with self.subTest(input=relative), self.assertRaises(ValueError):
                self.check(plan)
        for required in self.meta['modules'] + self.meta['injections']:
            path = self.helpers / required['source_file']
            before = path.read_bytes()
            path.write_bytes(before + b'changed')
            with self.subTest(package=required['source_file']), self.assertRaises(ValueError):
                self.check()
            path.write_bytes(before)
        self.check()

    def test_direct_attachment_proof_binds_output_helper_and_exclusive_ownership(self):
        # Synthetic proof exercises policy only; no production review is granted.
        hook = self.meta['injections'][0]
        site, relative = hook['injection_id'], hook['file']
        output = b'adapted native source\n'
        entry = {'id': site, 'file': relative,
                 'before': self.meta['official_source_sha256'][relative],
                 'after': GUARD.digest(output), 'helper_sha256': hook['source_sha256']}
        (self.workspace / relative).write_bytes(output)
        self.plan['inputs'][relative] = entry['after']
        self.meta.update(direct_review_status='reviewed', horizon_revision='fixture-pin',
            direct_attachments=[entry], direct_helpers={site: hook['source_file']})
        receipt = {'horizon_revision': 'fixture-pin', 'attachments': [copy.deepcopy(entry)]}
        original = copy.deepcopy(self.plan)
        self.plan['injections'] = [e for e in self.plan['injections'] if e['injection_id'] != site]
        def check(plan=None, proof=receipt):
            GUARD.check(self.plan if plan is None else plan, self.workspace, self.meta, self.helpers, proof)
        check()
        with self.assertRaisesRegex(ValueError, 'also owned'):
            check(original)
        with self.assertRaisesRegex(ValueError, 'require their receipt'):
            check(proof=None)
        changed = copy.deepcopy(receipt)
        changed['attachments'][0]['after'] = GUARD.digest(b'changed')
        with self.assertRaisesRegex(ValueError, 'reviewed contract'):
            check(proof=changed)
        changed = copy.deepcopy(self.plan)
        changed['inputs'][relative] = GUARD.digest(b'changed')
        with self.assertRaisesRegex(ValueError, 'bind direct'):
            check(changed)
        source = self.workspace / relative
        before = source.read_bytes()
        source.write_bytes(before + b'drift')
        with self.assertRaisesRegex(ValueError, 'output bytes'):
            check()
        source.write_bytes(before)
        helper = self.helpers / hook['source_file']
        before = helper.read_bytes()
        helper.write_bytes(before + b'drift')
        with self.assertRaisesRegex(ValueError, 'helper bytes'):
            check()
        helper.write_bytes(before)
        # Even an internally consistent receipt/contract must anchor its first
        # rewrite to the separately recorded official source, not its output.
        for preimage in (None, entry['after']):
            original_hash = self.meta['official_source_sha256'].pop(relative)
            if preimage is not None:
                self.meta['official_source_sha256'][relative] = preimage
            with self.assertRaisesRegex(ValueError, 'start at official source'):
                check()
            self.meta['official_source_sha256'][relative] = original_hash
        # Multiple edits of one source must form a continuous chain.
        second = {'id': 'second-direct-edit', 'file': relative,
                  'before': entry['after'], 'after': GUARD.digest(b'final native source')}
        self.meta['direct_attachments'].append(second)
        receipt['attachments'].append(copy.deepcopy(second))
        source.write_bytes(b'final native source')
        self.plan['inputs'][relative] = second['after']
        check()
        second['before'] = entry['before']
        receipt['attachments'][1]['before'] = entry['before']
        with self.assertRaisesRegex(ValueError, 'source chain differs'):
            check()
        self.meta['direct_review_status'] = 'pending'
        with self.assertRaisesRegex(ValueError, 'await combined'):
            check()

    def test_composite_state_contract_accepts_only_exact_owned_constructor(self):
        state = {'state_id': 'owned-state', 'target': 'example::Owner',
                 'file': self.meta['exclusive_files'][0], 'deserialize': False,
                 'constructors': [{'file': self.meta['exclusive_files'][0], 'node': [1, 0]}]}
        self.meta['states'] = [state]
        self.plan['states'] = [dict(copy.deepcopy(state), mod_id='pluto_lib', mod_version='0.0.0', priority=0)]
        self.check()
        for change in ('owner', 'constructor', 'duplicate', 'missing', 'foreign'):
            with self.subTest(change=change):
                plan = copy.deepcopy(self.plan)
                if change == 'owner':
                    plan['states'][0]['target'] = 'example::Other'
                elif change == 'constructor':
                    plan['states'][0]['constructors'][0]['node'] = [2, 0]
                elif change == 'duplicate':
                    plan['states'].append(copy.deepcopy(plan['states'][0]))
                elif change == 'missing':
                    plan['states'].clear()
                else:
                    plan['states'][0]['mod_id'] = 'foreign'
                with self.assertRaises(ValueError):
                    self.check(plan)

    def test_reviewed_call_site_contracts_reject_changed_or_missing_fields(self):
        required = self.meta['injections'][0]
        entry = self.plan['injections'][0]
        contract = {'selector': 'set_reference_file', 'ordinal': 1,
                    'require': 2, 'site_fingerprint': 'reviewed-site'}
        required.update(contract)
        entry.update(contract)
        self.check()
        for field in contract:
            for remove in (False, True):
                with self.subTest(field=field, missing=remove):
                    plan = copy.deepcopy(self.plan)
                    if remove:
                        del plan['injections'][0][field]
                    else:
                        plan['injections'][0][field] = 'changed'
                    with self.assertRaises(ValueError):
                        self.check(plan)

    def test_missing_competing_relocated_and_contract_changed_authorities_reject(self):
        mutations = []
        for field in ['module_id', 'file', 'name', 'visibility', 'mod_id', 'source']:
            mutations.append((f'module-{field}', lambda p, f=field: p['modules'][0].__setitem__(f, 'changed')))
        mutations.extend([
            ('missing-module', lambda p: p['modules'].pop()),
            ('duplicate-module', lambda p: p['modules'].append(dict(p['modules'][0], module_id='alias'))),
            ('missing-hook', lambda p: p['injections'].pop()),
            ('competing-hook', lambda p: p['injections'].append(dict(p['injections'][0], injection_id='alias'))),
        ])
        for field in ['injection_id', 'target', 'file', 'function_fingerprint', 'kind', 'mod_id', 'helper_source']:
            mutations.append((f'hook-{field}', lambda p, f=field: p['injections'][0].__setitem__(f, 'changed')))
        mutations.extend([
            ('cancellation', lambda p: p['injections'][0].__setitem__('cancellable', not p['injections'][0]['cancellable'])),
            ('bindings', lambda p: p['injections'][0].__setitem__('bindings', [])),
            ('schema', lambda p: p.__setitem__('schema', 7)),
            ('extra-module', lambda p: p['modules'].append({'module_id': 'unreviewed', 'file': self.meta['exclusive_files'][0], 'name': 'foreign'})),
            ('extra-hook', lambda p: p['injections'].append({'injection_id': 'unreviewed', 'file': self.meta['exclusive_files'][0], 'target': 'foreign'})),
            ('state', lambda p: p['states'].append({'file': self.meta['exclusive_files'][0]})),
            ('state-constructor', lambda p: p['states'].append({'file': 'elsewhere', 'constructors': [{'file': self.meta['exclusive_files'][0]}]})),
            ('bridge', lambda p: p['bridges'].append({'file': self.meta['exclusive_files'][0]})),
        ])
        for label, mutate in mutations:
            with self.subTest(change=label):
                plan = copy.deepcopy(self.plan)
                mutate(plan)
                with self.assertRaises(ValueError):
                    self.check(plan)
        # Omitting the guard must fail before any unrelated guard can run,
        # even when the kept hooks have lost all their module declarations.
        for modules in [self.plan['modules'], []]:
            plan = copy.deepcopy(self.plan)
            plan['modules'] = modules
            path = Path(self.temp.name) / 'omitted-guard.json'
            path.write_text(json.dumps(plan), encoding='utf-8', newline='\n')
            result = subprocess.run([sys.executable, str(ROOT / 'tools/check_sliced_plan.py'),
                                     str(path), str(self.workspace), 'protection'], capture_output=True)
            self.assertEqual(result.returncode, 2)


if __name__ == '__main__':
    unittest.main()
