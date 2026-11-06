"""Pure preparer guards; no Cargo, real host preparation or Rust execution."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('default_prepare', HERE / 'prepare.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

class PreparerChecks(unittest.TestCase):
    def test_frozen_source_and_exact_four_names(self):
        meta, seal = m.inputs()
        self.assertEqual(len(meta['expected_tests']), 4)
        self.assertEqual(len(seal['original_group']), 4)
        self.assertEqual(len(seal['preparer_group']), 4)

    def test_no_runtime_or_workflow_in_new_group(self):
        meta, _ = m.inputs()
        self.assertEqual(set(meta['helper_sha256']), {'prepare.py', 'test_prepare.py', 'README.md'})
        self.assertEqual(meta['runtime_source'], '62f050a1c4eab72da11032f4d6f059323c7e1e8a')
        self.assertEqual(meta['codec32_separate'], '37f0977a8bc6348b7735bc8818294143501716ab')

    def test_extra_file_or_empty_directory_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'a').write_bytes(b'one')
            expected = {'a': m.sha(b'one')}
            m.exact_files(root, expected)
            (root / '.cargo').mkdir()
            with self.assertRaises(ValueError): m.exact_files(root, expected)
            (root / '.cargo').rmdir(); (root / 'unexpected').write_bytes(b'')
            with self.assertRaises(ValueError): m.exact_files(root, expected)

    def test_symlink_in_frozen_group_is_not_regular(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'target').write_bytes(b'one')
            (root / 'a').symlink_to('target')
            with self.assertRaises(ValueError): m.exact_files(root, {'a': m.sha(b'one'), 'target': m.sha(b'one')})

    def test_official_seal_keeps_link_and_regular_mode(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'file').write_bytes(b'one')
            (root / 'outside').symlink_to('/nonexistent-test-only-target')
            first = m.tree(root)
            self.assertEqual(first['outside'], {'kind': 'link', 'target': '/nonexistent-test-only-target'})
            (root / 'file').chmod(0o755)
            self.assertNotEqual(first['file']['mode'], m.tree(root)['file']['mode'])
            self.assertEqual(first['file']['sha256'], m.tree(root)['file']['sha256'])

    def test_manifest_uses_actual_official_paths_no_features(self):
        host = Path('/official')
        manifest = m.tomllib.loads(m.manifest(host))
        self.assertEqual(manifest['dependencies']['serde_json'], '1')
        self.assertEqual(manifest['lib']['path'], 'lib.rs')
        self.assertNotIn('patch', manifest)
        self.assertNotIn('features', manifest)
        for name in ('horizon-items', 'horizon-client-core', 'horizon-world'):
            self.assertEqual(manifest['dependencies'][name], {'path': str(host / 'crates' / name)})

    def identities(self):
        host, output = Path('/official'), Path('/fixture')
        native = [{'name': name, 'version': '0.1.0'} for name in m.NATIVE]
        external = {'name': 'serde_json', 'version': '1.0.151', 'source': 'registry+official', 'checksum': 'test-checksum'}
        original = {'package': native + [external]}
        adapted = {'package': copy.deepcopy(original['package']) + [{'name': m.NAME, 'version': '0.0.0'}]}
        packages = [dict(p, id='pkg:' + p['name'], source=p.get('source'), manifest_path=str(host / 'crates' / p['name'] / 'Cargo.toml')) for p in native]
        packages += [dict(external, id='pkg:serde_json', manifest_path='/registry/serde_json/Cargo.toml')]
        packages += [dict(name=m.NAME, version='0.0.0', source=None, id='fixture', manifest_path=str(output / 'Cargo.toml'))]
        return host, output, {'packages': packages, 'workspace_members': ['fixture']}, original, adapted

    def test_expected_official_identity_subset_passes(self):
        m.validate_dependencies(*self.identities())

    def test_foreign_lock_identity_rejected(self):
        args = self.identities(); args[-1]['package'][0]['version'] = '999'
        with self.assertRaises(ValueError): m.validate_dependencies(*args)

    def test_duplicate_native_name_rejected(self):
        args = self.identities(); duplicate = dict(args[2]['packages'][0], id='different-id')
        args[2]['packages'].append(duplicate)
        with self.assertRaises(ValueError): m.validate_dependencies(*args)

    def test_native_path_escape_rejected(self):
        args = self.identities(); args[2]['packages'][0]['manifest_path'] = '/unapproved/crates/horizon-core/Cargo.toml'
        with self.assertRaises(ValueError): m.validate_dependencies(*args)

    def test_registry_substitution_for_native_rejected(self):
        args = self.identities(); args[2]['packages'][0]['source'] = 'registry+official'
        with self.assertRaises(ValueError): m.validate_dependencies(*args)

    def test_extra_workspace_member_rejected(self):
        args = self.identities(); args[2]['workspace_members'].append('other')
        with self.assertRaises(ValueError): m.validate_dependencies(*args)

    def test_generated_fixture_rejects_extra_directory_and_link(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            for name in ['lib.rs', 'Cargo.toml', 'Cargo.lock', m.RECEIPT]: (output / name).write_bytes(b'')
            self.assertEqual(set(m.fixture_tree(output)), {'lib.rs', 'Cargo.toml', 'Cargo.lock'})
            (output / 'target').mkdir()
            with self.assertRaises(ValueError): m.fixture_tree(output)
            (output / 'target').rmdir(); (output / 'lib.rs').unlink(); (output / 'lib.rs').symlink_to('Cargo.lock')
            with self.assertRaises(ValueError): m.fixture_tree(output)

    def test_original_input_inventory_cannot_be_silently_extended(self):
        meta = json.loads((HERE / 'inputs.json').read_text())
        self.assertEqual(set(meta['original_group_sha256']), {'catalog_tests.rs', 'README.md', 'inputs.json', 'test_source.py'})
        self.assertEqual(meta['original_group_sha256']['catalog_tests.rs'], '864db7e085caea9ac6ef7a5105e88746d3c2ab867cddfa411bad44d83287c123')

    def test_clean_query_includes_ignored_and_all_untracked(self):
        calls = []
        def fake(args):
            calls.append(args)
            if args[-2:] == ['--get', 'remote.origin.url']: return 'https://github.com/horizon-rust/horizon.git\n'
            if args[-2:] == ['rev-parse', 'HEAD']: return m.PIN + '\n'
            return '!! unexpected\n'
        with patch.object(m, 'run', side_effect=fake):
            with self.assertRaisesRegex(ValueError, 'ignored'): m.clean_official(Path('/official'))
        self.assertIn(['git', '-C', '/official', 'status', '--porcelain', '--untracked-files=all', '--ignored=matching'], calls)

if __name__ == '__main__': unittest.main()
