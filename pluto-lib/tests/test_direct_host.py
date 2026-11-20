"""Exercise direct state insertion through Rust constructors and owner lifetimes."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('direct_host', ROOT / 'pluto-lib/tools/prepare_direct_host.py')
direct = importlib.util.module_from_spec(spec)
sys.path.insert(0, str(ROOT / 'pluto-lib/tools'))
try:
    spec.loader.exec_module(direct)
    import plan_direct_host as planner
finally:
    sys.path.pop(0)


class DirectHostTests(unittest.TestCase):
    def test_production_contracts_gate_apply_and_compile(self):
        import json
        from unittest.mock import patch
        pin = '441f49d57e344bd56b90fd4af5c4e443ff3937c9'
        for contract_exit in (0, 1):
            with self.subTest(contract_exit=contract_exit), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                output = root / 'prepared'
                def prepare(source, dest):
                    (dest / 'bundle').mkdir(parents=True)
                    (dest / 'horizon-overlay').mkdir()
                    (dest / 'bundle/horizon-mixin.toml').write_text(newline='\n', data='[mod]\nid="pluto_lib"\n')
                    return {'horizon_revision': pin}
                def checkout(source, workspace, revision):
                    workspace.mkdir()
                def run(command, **kwargs):
                    code = contract_exit if str(command[1]).endswith('check_sliced_plan.py') else 0
                    return subprocess.CompletedProcess(command, code, 'fixture output\n', '')
                with patch.object(planner, 'prepare', side_effect=prepare), \
                     patch.object(planner, 'checkout_source', side_effect=checkout), \
                     patch.object(planner.subprocess, 'check_output', return_value=pin), \
                     patch.object(planner.subprocess, 'run', side_effect=run) as commands:
                    result = planner.plan(root / 'source', output, root / 'mixins',
                                          check_native=True, require_contracts=True)
                self.assertEqual(result, contract_exit)
                calls = [call.args[0] for call in commands.call_args_list]
                self.assertEqual(calls[0][1], 'plan')
                self.assertEqual(set(calls[1][4].split(',')), set(planner.GUARDS))
                self.assertEqual(Path(calls[1][5]), output / 'direct-host.json')
                receipt = json.loads((output / 'contract-check.json').read_text())
                self.assertEqual(receipt['exit'], contract_exit)
                if contract_exit:
                    self.assertEqual(len(calls), 2, 'failed contracts must prevent apply and Cargo')
                    self.assertFalse((output / 'native-check.json').exists())
                else:
                    self.assertEqual(calls[2][1], 'apply')
                    self.assertEqual(calls[3][:3], ['cargo', '+1.95.0', 'check'])

    def test_checkout_uses_git_blob_lf_despite_global_windows_conversion(self):
        import os
        from unittest.mock import patch
        with tempfile.TemporaryDirectory(prefix='direct-checkout-') as temp:
            root = Path(temp)
            source, stage = root / 'source', root / 'stage'
            config = root / 'gitconfig'
            config.write_text(newline='\n', data='[core]\n\tautocrlf = true\n\teol = crlf\n', encoding='utf-8')
            env = {'GIT_CONFIG_GLOBAL': str(config), 'GIT_CONFIG_NOSYSTEM': '1',
                   'GIT_AUTHOR_NAME': 'Fixture', 'GIT_AUTHOR_EMAIL': 'fixture@example.invalid',
                   'GIT_COMMITTER_NAME': 'Fixture', 'GIT_COMMITTER_EMAIL': 'fixture@example.invalid'}
            with patch.dict(os.environ, env):
                subprocess.run(['git', 'init', str(source)], check=True, capture_output=True)
                def git(*args, data=None):
                    return subprocess.check_output(['git', '-C', str(source), *args], input=data).strip()
                content = b'pub fn example() {\n    // pinned LF bytes\n}\n'
                blob = git('hash-object', '-w', '--stdin', data=content)
                tree = git('mktree', data=b'100644 blob ' + blob + b'\tprobe.rs\n')
                revision = git('commit-tree', tree.decode(), '-m', 'Fixture').decode()
                git('update-ref', 'refs/heads/main', revision)
                git('symbolic-ref', 'HEAD', 'refs/heads/main')
                planner.checkout_source(source, stage, revision)
                self.assertEqual((stage / 'probe.rs').read_bytes(), content)
                self.assertEqual(git('show', revision + ':probe.rs'), content.strip())
                self.assertFalse((source / 'probe.rs').exists(), 'reference working tree stays untouched')
                value = subprocess.check_output(['git', '-C', str(stage), 'config', '--local', 'core.autocrlf'])
                self.assertEqual(value.strip(), b'false')

    def test_api_stage_rewrites_only_host_edge_and_refuses_reuse(self):
        import tomllib
        with tempfile.TemporaryDirectory(prefix='direct-api-') as temp:
            stage = Path(temp)
            workspace = stage / 'Cargo.toml'
            workspace.write_text(newline='\n', data='[workspace]\nmembers = [\n]\n', encoding='utf-8')
            package = planner.stage_api_check(stage, ROOT / 'pluto-lib/mod')
            original = tomllib.loads((ROOT / 'pluto-lib/mod/api/Cargo.toml').read_text(encoding='utf-8'))
            rewritten = tomllib.loads((stage / 'native-api/pluto_lib/Cargo.toml').read_text(encoding='utf-8'))
            self.assertEqual(package, original['package']['name'])
            original['dependencies']['horizon-client'] = {
                'path': '../../crates/horizon-client', 'default-features': False}
            self.assertEqual(rewritten, original)
            self.assertEqual(tomllib.loads(workspace.read_text())['workspace']['members'], ['native-api/pluto_lib'])
            with self.assertRaisesRegex(ValueError, 'already exists'):
                planner.stage_api_check(stage, ROOT / 'pluto-lib/mod')

    def test_transferred_binding_and_owner_drift_cannot_be_discarded(self):
        import copy
        import tomllib
        parsed = tomllib.loads((ROOT / 'pluto-lib/mod/horizon-mixin.toml').read_text(encoding='utf-8'))
        direct.validate_transferred_declarations(parsed)
        mutations = [
            ('injection', 'pluto-item-after-fall', lambda e: e['bindings'][-1].update(value='other_contact')),
            ('injection', 'pluto-living-attributes-reload', lambda e: e['bindings'][2].update(value='other')),
            ('injection', 'pluto-item-placement-root-path', lambda e: e['variable'].update(at='LOAD')),
            ('state', 'pluto-block-entity-cache-generation', lambda e: e.update(target='other::Owner')),
            ('state', 'pluto-living-drop-state', lambda e: e['constructor'][0].update(require=2)),
        ]
        for kind, site, change in mutations:
            candidate = copy.deepcopy(parsed)
            change(next(e for e in candidate[kind] if e['id'] == site))
            with self.subTest(site=site), self.assertRaisesRegex(ValueError, 'full declaration changed'):
                direct.validate_transferred_declarations(candidate)
        candidate = copy.deepcopy(parsed)
        candidate['state'].append(copy.deepcopy(next(e for e in candidate['state']
            if e['id'] == 'pluto-living-drop-state')))
        with self.assertRaisesRegex(ValueError, 'exactly one'):
            direct.validate_transferred_declarations(candidate)

    def test_missing_constructor_rejected(self):
        with self.assertRaisesRegex(ValueError, 'constructor'):
            direct.transform('struct KeptEntry {}', 'cache', 'KeptEntry', 'u64', '7')

    def test_direct_rules_require_unique_anchors(self):
        for rule in direct.RULES:
            with self.subTest(site=rule['id']):
                original = '\n'.join(before for before, _ in rule['edits'])
                expected = '\n'.join(after for _, after in rule['edits'])
                helper = 'fn test_helper() {}'
                suffix = '\n' + helper if rule.get('append_helper', True) else ''
                self.assertEqual(direct.apply_rule(original, rule, helper), expected + suffix)
                if not rule['edits']:
                    continue  # an append-only rule has no anchor; its official file hash is the drift check
                for drifted in ('', original + '\n' + original):
                    with self.assertRaisesRegex(ValueError, 'expected one source anchor'):
                        direct.apply_rule(drifted, rule, helper)

    def test_item_prototypes_preserve_surrounding_load_and_reject_ambiguous_transfer(self):
        transfer = '        Ok(Self {\n            items,\n            mob_visibility,'
        original = 'fallible_load()?;\n' + transfer + '\n        })\n'
        helper = 'fn pluto_item_prototypes() {}'
        changed = direct.attach_item_prototypes(original, helper)
        self.assertEqual(changed, original.replace('            items,',
            '            items: pluto_item_prototypes(items, &raw),') + '\n' + helper)
        for drifted in ('', original + original):
            with self.assertRaisesRegex(ValueError, 'unique final items transfer'):
                direct.attach_item_prototypes(drifted, helper)

    def test_source_drift_rejected_before_any_output(self):
        with tempfile.TemporaryDirectory(prefix='direct-drift-') as temp:
            source = Path(temp) / 'source'
            relative = next(iter(direct.STATES.values()))[0]
            path = source / relative
            path.parent.mkdir(parents=True)
            path.write_text(newline='\n', data='changed host input', encoding='utf-8')
            output = Path(temp) / 'output'
            with self.assertRaisesRegex(ValueError, 'fingerprint mismatch'):
                direct.prepare(source, output)
            self.assertFalse(output.exists())

    def test_actual_state_helpers_with_direct_fields_and_native_constructor_shapes(self):
        # A minimal host fixture retains the relevant native shapes: a closure
        # before KeptEntry construction and CombatTracker's derived Default /
        # struct update. The transformation and both state helpers are real.
        cache = direct.transform('''
struct KeptEntry {
    vertices: Vec<u32>,
}
fn keep(old: Option<KeptEntry>) -> KeptEntry {
    let vertices = old.map_or_else(Vec::new, |e| e.vertices);
            KeptEntry {
                vertices,
            }
}
''', 'pluto-block-entity-cache-generation', 'KeptEntry', 'u64',
            'crate::game::pluto_block_entity_cache::frame_generation()')
        combat = direct.transform('''
#[derive(Default)]
struct CombatTracker { damage: u32 }
fn message_tracker() -> CombatTracker {
    let damage = Some(4).map(|n| n + 1).unwrap();
    CombatTracker { damage, ..CombatTracker::default() }
}
''', 'pluto-living-drop-state', 'CombatTracker',
            'crate::game::combat::pluto_living_drop_state::HeadDeaths',
            '::core::default::Default::default()')
        module = ROOT / 'pluto-lib/mod'
        rust = '''mod game {
    pub mod pluto_block_entity_cache { include!(CACHE_PATH); }
    pub mod combat { pub mod pluto_living_drop_state { include!(DROP_PATH); } }
}
'''.replace('CACHE_PATH', '"' + (module / 'pluto-block-entity-cache.rs').as_posix() + '"').replace('DROP_PATH', '"' + (module / 'pluto-living-drop-state.rs').as_posix() + '"')
        rust += cache + combat + '''
#[test]
fn direct_storage_preserves_owner_and_capture() {
    use game::pluto_block_entity_cache as cache;
    cache::begin_lookup(Some(11));
    let entry = keep(None);
    cache::begin_lookup(Some(12));
    assert_eq!(*entry.__horizon_mixins_pluto_lib_pluto_block_entity_cache_generation(), 11);
    assert!(!cache::reusable(*entry.__horizon_mixins_pluto_lib_pluto_block_entity_cache_generation(), Some(12)));
    let next = keep(Some(entry));
    assert_eq!(*next.__horizon_mixins_pluto_lib_pluto_block_entity_cache_generation(), 12);
    let first = CombatTracker::default();
    let second = message_tracker();
    assert_eq!(second.damage, 5);
    let state = first.__horizon_mixins_pluto_lib_pluto_living_drop_state();
    assert!(state.claim(7).unwrap());
    assert!(!state.claim(7).unwrap());
    assert!(second.__horizon_mixins_pluto_lib_pluto_living_drop_state().claim(7).unwrap());
    state.begin_wave([7]).unwrap();
    assert!(!state.available(7).unwrap());
    state.clear().unwrap();
    assert!(state.claim(7).unwrap());
}
'''
        with tempfile.TemporaryDirectory(prefix='direct-state-') as temp:
            source = Path(temp) / 'states.rs'
            binary = Path(temp) / 'states.exe'
            source.write_text(newline='\n', data=rust, encoding='utf-8')
            subprocess.run(['rustc', '+1.95.0', '--edition=2021', '--test', str(source), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
