"""Declaration-only source checks; no packaging, planner or native execution."""
from pathlib import Path
import ast
import hashlib
import json
import tomllib
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
R = json.loads((HERE / 'receipt.json').read_text())
COMPONENTS = ('pluto_native_creative', 'pluto_transport_3a', 'pluto_tool_rules',
              'pluto_component_display_3a', 'pluto_component_bar', 'pluto_items_b2')
def blob(b): return hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()
def original(path, data):
    out = data.replace(R['horizon'].encode(), R['previous_horizon'].encode())
    if path.endswith('.json'):
        out = out.replace(R['mixins'].encode(), R['previous_mixins'].encode())
    return out

class SourceTests(unittest.TestCase):
    def test_six_changed_files_only(self):
        want = {'pluto-lib/horizon-native.json'} | {
            f'pluto-lib/mixins/{x}/horizon-mixin.toml' for x in COMPONENTS[1:]}
        self.assertEqual(set(R['changed']), want)
    def test_after_identities(self):
        for p, row in R['changed'].items():
            b = (ROOT/p).read_bytes()
            self.assertEqual(blob(b), row['after_git_blob'])
            self.assertEqual(hashlib.sha256(b).hexdigest(), row['after_sha256'])
    def test_full_inverse(self):
        for p, row in R['changed'].items():
            b = original(p, (ROOT/p).read_bytes())
            self.assertEqual(blob(b), row['before_git_blob'])
            self.assertEqual(hashlib.sha256(b).hexdigest(), row['before_sha256'])
    def test_component_contract(self):
        header = json.loads((ROOT/'pluto-lib/horizon-native.json').read_text())
        for component in COMPONENTS:
            manifest = tomllib.loads((ROOT/f'pluto-lib/mixins/{component}/horizon-mixin.toml').read_text())
            self.assertEqual(manifest['mod']['id'], component)
            self.assertEqual(manifest['mod']['version'], header['version'])
            self.assertEqual(manifest['compatibility'], {'horizon_revision': header['horizon']})
    def test_header_only_two_values(self):
        path = 'pluto-lib/horizon-native.json'; data = (ROOT/path).read_bytes()
        after, before = json.loads(data), json.loads(original(path,data))
        self.assertEqual({k for k in after if after[k] != before[k]}, {'horizon','mixins'})
        self.assertEqual((after['horizon'], after['mixins']), (R['horizon'],R['mixins']))
    def test_unchanged_native_and_packager(self):
        for p in ('pluto-lib/mixins/pluto_native_creative/horizon-mixin.toml','pluto-lib/tools/package_native.py'):
            self.assertEqual(blob((ROOT/p).read_bytes()), R['inputs'][p]['git_blob'])
        tree = ast.parse((ROOT/'pluto-lib/tools/package_native.py').read_text())
        values = [ast.literal_eval(n.value) for n in tree.body if isinstance(n,ast.Assign)
                  and any(isinstance(t,ast.Name) and t.id == 'COMPONENTS' for t in n.targets)]
        self.assertEqual(values, [COMPONENTS])
    def test_other_sections_unchanged(self):
        for p in R['changed']:
            if not p.endswith('.toml'): continue
            b = (ROOT/p).read_bytes()
            after, before = tomllib.loads(b.decode()), tomllib.loads(original(p,b).decode())
            after.pop('compatibility'); before.pop('compatibility')
            self.assertEqual(after,before)
    def test_tamper_does_not_reconstruct(self):
        for p, row in R['changed'].items():
            self.assertNotEqual(blob(original(p,(ROOT/p).read_bytes()+b'\n')),row['before_git_blob'])

if __name__ == '__main__': unittest.main()
