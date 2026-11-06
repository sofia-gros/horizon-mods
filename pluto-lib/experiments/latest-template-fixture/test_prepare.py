"""Pure preparer guards; no Cargo, real host preparation or Rust execution."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from unittest import mock

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('template_prepare', HERE / 'prepare.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

class PreparerChecks(unittest.TestCase):
    def test_frozen_source_and_exact_six_names(self):
        meta, seal = m.inputs()
        self.assertEqual(len(meta['expected_tests']), 6)
        self.assertEqual(len(seal['original_group']), 7)
        self.assertEqual(len(seal['preparer_group']), 4)

    def test_no_runtime_or_workflow_in_new_group(self):
        meta, _ = m.inputs()
        self.assertEqual(set(meta['helper_sha256']), {'prepare.py', 'test_prepare.py', 'README.md'})
        self.assertEqual(meta['test_source'], 'fe5f2d4159e9e70b2819b826e5dfe2951b5cb896')
        self.assertEqual(meta['official'], 'f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd')

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
            for name in ['lib.rs', 'native-template.rs', 'Cargo.toml', 'Cargo.lock', m.RECEIPT]: (output / name).write_bytes(b'')
            self.assertEqual(set(m.fixture_tree(output)), {'lib.rs', 'native-template.rs', 'Cargo.toml', 'Cargo.lock'})
            (output / 'target').mkdir()
            with self.assertRaises(ValueError): m.fixture_tree(output)
            (output / 'target').rmdir(); (output / 'lib.rs').unlink(); (output / 'lib.rs').symlink_to('Cargo.lock')
            with self.assertRaises(ValueError): m.fixture_tree(output)

    def test_original_input_inventory_cannot_be_silently_extended(self):
        meta = json.loads((HERE / 'inputs.json').read_text())
        self.assertEqual(set(meta['original_group_sha256']), {'template_tests.rs', 'README.md', 'inputs.json', 'test_source.py', 'fixture-literals.patch', 'fixture-records.json', 'official-audit.json'})
        self.assertEqual(meta['original_group_sha256']['template_tests.rs'], 'c887cef350080f698f033bcaf87b3daca08da0ceaaf763a57b360a659879be2b')

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

    def test_fragment_exact_bytes_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            host = Path(directory); (host / 'native.rs').write_bytes(b'prefix\nSTART\nbody\nEND\nsuffix')
            meta = {'native_template': {'path': 'native.rs', 'start': 'START', 'end': 'END', 'sha256': m.sha(b'START\nbody\n')}}
            self.assertEqual(m.template_body(host, meta), b'START\nbody\n')

    def test_duplicate_boundary_or_changed_body_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            host = Path(directory); path = host / 'native.rs'
            meta = {'native_template': {'path': 'native.rs', 'start': 'START', 'end': 'END', 'sha256': m.sha(b'STARTbody')}}
            for data in (b'STARTbodySTARTEND', b'STARTchangedEND', b'STARTbodyENDEND', b'ENDSTARTbody'):
                path.write_bytes(data)
                with self.assertRaises(ValueError): m.template_body(host, meta)

    def controlled(self):
        return patch.dict(m.os.environ, {'PLUTO_FIXTURE_CONTROLLER': 'latest-item-contracts-v1'})

    def test_uncontrolled_process_launch_fails_before_spawn(self):
        with patch.dict(m.os.environ, {}, clear=True), patch.object(m.subprocess, 'Popen') as spawn:
            with self.assertRaisesRegex(ValueError, 'controller'): m.run(['never-executed'])
            spawn.assert_not_called()
        with self.controlled(), patch.object(m.os, 'getpid', return_value=7), patch.object(m.os, 'getpgrp', return_value=8), patch.object(m.subprocess, 'Popen') as spawn:
            with self.assertRaisesRegex(ValueError, 'controller'): m.run(['never-executed'])
            spawn.assert_not_called()

    def test_inner_process_inherits_outer_group(self):
        process = mock.Mock(returncode=0); process.communicate.return_value=('value', ''); process.poll.return_value=0
        with self.controlled(), patch.object(m.os, 'getpid', return_value=7), patch.object(m.os, 'getpgrp', return_value=7), patch.object(m.subprocess, 'Popen', return_value=process) as spawn, patch.object(m.os, 'killpg') as group:
            self.assertEqual(m.run(['never-executed']), 'value')
        self.assertIs(spawn.call_args.kwargs['start_new_session'], False)
        process.communicate.assert_called_once_with(timeout=60)
        process.wait.assert_called_once_with(timeout=2)
        process.kill.assert_not_called(); group.assert_not_called()

    def test_inner_timeout_kills_only_direct_child_and_returns_failure(self):
        process = mock.Mock(returncode=None); process.poll.return_value=None
        process.communicate.side_effect=m.subprocess.TimeoutExpired(['never-executed'],60)
        with self.controlled(), patch.object(m.os, 'getpid', return_value=7), patch.object(m.os, 'getpgrp', return_value=7), patch.object(m.subprocess, 'Popen', return_value=process), patch.object(m.os, 'killpg') as group:
            with self.assertRaises(m.subprocess.TimeoutExpired): m.run(['never-executed'])
        process.kill.assert_called_once_with(); process.wait.assert_called_once_with(timeout=2); group.assert_not_called()

if __name__ == '__main__': unittest.main()
