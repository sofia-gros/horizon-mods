"""Exact source assembly checks; not Rust, official plan or gameplay proof."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parent
META = json.loads((ROOT / 'docs/common-composite-source-20261106.json').read_text())
NATIVE = ROOT / 'mixins/pluto_native_creative'

def digest(value):
    return hashlib.sha256(value).hexdigest()

class CompositeSource(unittest.TestCase):
    def test_both_complete_api_sets_remain_exact(self):
        for api in META['apis'].values():
            root = REPO / api['path']
            actual = {p.relative_to(root).as_posix(): digest(p.read_bytes())
                      for p in root.rglob('*') if p.is_file()}
            self.assertEqual(actual, api['files'])

    def test_exact_additive_manifest_recovers_original(self):
        value = (NATIVE / 'horizon-mixin.toml').read_bytes()
        expected = META['manifest']
        suffix = expected['suffix'].encode()
        self.assertEqual(digest(value), expected['sha256'])
        self.assertTrue(value.endswith(suffix))
        self.assertEqual(digest(value[:-len(suffix)]), expected['original_sha256'])
        parsed = tomllib.loads(value.decode())
        self.assertEqual([m['id'] for m in tomllib.loads(suffix.decode())['module']], expected['module_ids'])
        for key in ('crate', 'module', 'injection', 'state'):
            ids = [x['id'] for x in parsed.get(key, [])]
            self.assertEqual(len(ids), len(set(ids)))

    def test_all_runtime_sources_are_exact_and_regular(self):
        root = ROOT / 'mixins'
        files = list(root.rglob('*'))
        self.assertFalse(any(p.is_symlink() for p in files))
        actual = {p.relative_to(REPO).as_posix(): digest(p.read_bytes())
                  for p in files if p.is_file()}
        self.assertEqual(actual, META['runtime_files'])

    def test_complete_bundle_has_single_known_owners(self):
        path = ROOT / 'tools/package_native.py'
        self.assertEqual(digest(path.read_bytes()), META['unchanged_package_sha256'])
        spec = importlib.util.spec_from_file_location('composite_package', path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        manifest, payload = module._bundle(json.loads((ROOT / 'horizon-native.json').read_text()))
        parsed = tomllib.loads(manifest.decode())
        self.assertEqual({k:len(parsed[k]) for k in ('crate','module','injection','state')},
                         {'crate':2,'module':69,'injection':34,'state':3})
        for name in ('pluto_language','pluto_language_text','pluto_presentation'):
            self.assertEqual(sum(m['name'] == name for m in parsed['module']), 1)
        for name in ('language.rs','language-text.rs','recipe-book-presentation.rs','guide-layout.rs','rendering.rs'):
            self.assertEqual(payload['pluto_native_creative/'+name], (NATIVE/name).read_bytes())

if __name__ == '__main__':
    unittest.main()
