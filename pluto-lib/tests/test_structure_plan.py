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
