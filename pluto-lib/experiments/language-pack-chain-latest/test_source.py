"""Exact-source contract checks only; no Git mutation, Cargo, metadata or native execution."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import tomllib
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
INPUTS = json.loads((HERE / 'inputs.json').read_text())
SOURCE = None
REVISION = 'f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
OFFICIAL = 'https://github.com/horizon-rust/horizon.git'
PATCHES = {'wgpu-hal': 'third_party/wgpu-hal', 'naga': 'third_party/naga',
           'libmimalloc-sys': 'third_party/libmimalloc-sys'}
DIRECT = ['horizon-client-lib', 'horizon-client-core', 'horizon-client-render', 'horizon-mods']


def patch_contract(root, packages):
    if root.get('patch') != {'crates-io': {n: {'path': p} for n, p in PATCHES.items()}}:
        raise ValueError('Exact official patch declarations required')
    for name, path in PATCHES.items():
        if packages[path]['package']['name'] != name:
            raise ValueError('Exact patched package owner required')


class LatestPackChainSource(unittest.TestCase):
    def test_eight_frozen_cases(self):
        for path, digest in INPUTS['fixture_inputs'].items():
            self.assertEqual(hashlib.sha256((HERE / path).read_bytes()).hexdigest(), digest, path)
        text = (HERE / 'fixture.rs').read_text()
        names = ['tests::' + n for n in re.findall(r'#\[test\]\s+fn (\w+)\(', text)]
        self.assertEqual(names, INPUTS['expected_test_names'])
        self.assertEqual(len(set(names)), 8)
        self.assertNotIn('#[ignore]', text)

    def test_original_six_bodies_and_helpers_are_unchanged(self):
        old = (ROOT / 'pluto-lib/experiments/language-pack-chain/fixture.rs').read_text()
        new = (HERE / 'fixture.rs').read_text()
        marker = '\n    #[test]\n    fn lenient_comment_layer_merges_before_mod_whole_file_replacement()'
        self.assertEqual(new.count(marker), 1)
        self.assertEqual(new.split(marker)[0] + '}\n', old)
        for path, digest in INPUTS['unchanged_prior_inputs'].items():
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), digest, path)

    def test_actual_owners_without_substitute_parser_or_setter(self):
        text = (HERE / 'fixture.rs').read_text()
        for token in ['packs::folder(', 'packs::enabled(', 'packs::server_pack(',
                      'packs::build_overlay(', 'mod_content::build(', 'Package::write_files(',
                      'Package::read(', 'Content::read(', 'Language::load(',
                      'horizon_client_core::lang::read_lang_file(']:
            self.assertIn(token, text)
        for token in ['struct Language', 'struct Pack ', 'impl ResourceSource', 'fn merge(',
                      'build_overlay_uncached(', 'serde_json::from', 'lang::set(',
                      'mod_content::resource_root(', 'prune_roots(', 'Command::new']:
            self.assertNotIn(token, text)

    def test_real_order_and_strict_lenient_expectations(self):
        text = (HERE / 'fixture.rs').read_text()
        for token in ['s.selected(&["z", "a"])', 'selected.push(server)',
                      'do not silently skip a fixture pack', 'do not resurrect filtered source keys',
                      '# original pack comment', 'strict_file.contains', 'lenient_file.contains',
                      'literal %s %d', 'assert_eq!(direct, lenient_direct)',
                      'assert_eq!(direct, before)']:
            self.assertIn(token, text)
        self.assertNotIn('if let Ok(', text)

    def test_owned_outputs_and_native_cache_delegation(self):
        text = (HERE / 'fixture.rs').read_text()
        for token in ['fs::create_dir(&path)', 'fs::remove_dir_all(&self.0)',
                      'Entry::Link(fs::read_link(&path)',
                      'packs::build_overlay(&base, &selected, &root)',
                      'assert_eq!(tree(&root), before)']:
            self.assertIn(token, text)

    def test_direct_manifests_and_lock_identities(self):
        template = tomllib.loads((HERE / 'Cargo.toml.in').read_text())
        self.assertEqual(template['package']['name'], 'pluto-language-pack-chain-latest-fixture')
        self.assertEqual(set(template['dependencies']), set(DIRECT) | {'zip'})
        packages = tomllib.loads((SOURCE / 'Cargo.lock').read_text())['package']
        for name in DIRECT:
            self.assertEqual(template['dependencies'][name]['path'], '@OFFICIAL@/crates/' + name)
            manifest = SOURCE / 'crates' / name / 'Cargo.toml'
            self.assertEqual(manifest.resolve(), manifest)
            package = tomllib.loads(manifest.read_text())['package']
            self.assertEqual(package['name'], name)
            matches = [p for p in packages if p['name'] == name]
            self.assertEqual(len(matches), 1)
            self.assertEqual(matches[0]['version'], package['version'])
            self.assertIsNone(matches[0].get('source'))
        self.assertEqual(template['dependencies']['zip'],
                         {'version': '=9.1.3', 'default-features': False, 'features': ['deflate']})
        matches = [p for p in packages if p['name'] == 'zip' and p['version'] == '9.1.3']
        self.assertEqual(len(matches), 1)
        self.assertEqual({k: matches[0].get(k) for k in ['name', 'version', 'source', 'checksum']},
                         INPUTS['zip_identity'])

    def test_exact_latest_source_bytes_not_checkout_authority(self):
        self.assertEqual(INPUTS['horizon'], REVISION)
        self.assertEqual(INPUTS['official_url'], OFFICIAL)
        self.assertEqual(len(INPUTS['official_inputs']), 24)
        for entry in INPUTS['official_inputs']:
            path = SOURCE / entry['path']
            self.assertEqual(path.resolve(), path, entry['path'])
            data = path.read_bytes()
            self.assertEqual(len(data), entry['size'], entry['path'])
            blob = hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest()
            self.assertEqual(blob, entry['blob_sha'], entry['path'])
            self.assertEqual(hashlib.sha256(data).hexdigest(), entry['sha256'], entry['path'])
        self.assertFalse(INPUTS['execution']['run_now'])
        self.assertEqual(INPUTS['source_validation_scope'], 'selected bytes only, not a complete clean checkout')

    def test_patch_contract_rejects_redirect_and_owner_substitution(self):
        root = tomllib.loads((SOURCE / 'Cargo.toml').read_text())
        packages = {p: tomllib.loads((SOURCE / p / 'Cargo.toml').read_text()) for p in PATCHES.values()}
        patch_contract(root, packages)
        for name, path in PATCHES.items():
            self.assertEqual((SOURCE / path / 'Cargo.toml').resolve(), SOURCE / path / 'Cargo.toml')
            bad = copy.deepcopy(root); bad['patch']['crates-io'][name] = {'version': '0.0.0'}
            with self.assertRaises(ValueError): patch_contract(bad, packages)
            bad = copy.deepcopy(root); bad['patch']['crates-io'][name] = {'path': '../other/' + name}
            with self.assertRaises(ValueError): patch_contract(bad, packages)
            bad_packages = copy.deepcopy(packages); bad_packages[path]['package']['name'] = 'substitute'
            with self.assertRaises(ValueError): patch_contract(root, bad_packages)

    def test_latest_native_fallback_has_pinned_source_basis(self):
        language = (SOURCE / 'crates/horizon-client-core/src/lang.rs').read_text()
        packs = (SOURCE / 'crates/horizon-client-render/src/packs.rs').read_text()
        self.assertIn('pub fn read_lang_file(bytes: &[u8], map: &mut HashMap<String, String>)', language)
        for token in ['_ if kind == Merge::Lang =>', 'horizon_client_core::lang::read_lang_file(b, &mut strings)',
                      'Ok(Value::Object(m)) => Some(m)', 'serde_json::to_vec(&out).ok()']:
            self.assertIn(token, packs)

    def test_unexecuted_and_no_priority_repair_claim(self):
        text = (HERE / 'README.md').read_text()
        for token in ['uncompiled and unrun', 'No runner or trigger', 'does not prove a cache hit',
                      'Game admission/pack', 'remain unimplemented', 'not** certify a complete']:
            self.assertIn(token, text)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', required=True, type=Path)
    SOURCE = parser.parse_args().source_root.resolve()
    unittest.main(argv=[__file__], verbosity=2)
