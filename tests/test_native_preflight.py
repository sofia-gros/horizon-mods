"""The old CLI view must not remove executable mod declarations."""
import importlib.util
from pathlib import Path
import tomllib
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools/smoke-local'))
SPEC = importlib.util.spec_from_file_location('native_preflight', ROOT / 'tools/smoke-local/native_preflight.py')
PREFLIGHT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PREFLIGHT)


class NativePreflight(unittest.TestCase):
    def test_factory_guard_is_selected_for_real_operations_not_requested_names(self):
        requested = ['protection']
        self.assertEqual(PREFLIGHT.required_guards({}, requested), ['protection', 'structures'])
        for section, entry in [
            ('modules', {'module_id': 'pluto-block-entity-access'}),
            ('modules', {'module_id': 'alias', 'file': 'crates/horizon-client/src/entry.rs',
                         'name': 'pluto_block_entity_access'}),
            ('modules', {'module_id': 'alias', 'file': 'crates/horizon-client-render/src/lib.rs',
                         'name': 'pluto_client_block_entity_state'}),
            ('states', {'state_id': 'pluto-block-entity-client-state'}),
            ('states', {'state_id': 'alias', 'target': 'horizon_client_render::chunk::block_entity::ModBlockEntity'}),
            ('states', {'state_id': 'alias', 'file': 'crates/horizon-client-render/src/chunk/block_entity.rs'}),
            ('states', {'state_id': 'alias', 'constructors': [{'file': 'crates/horizon-client-render/src/chunk/block_entity.rs'}]}),
            ('injections', {'injection_id': 'pluto-block-entity-factory'}),
            ('injections', {'mod_id': 'foreign', 'injection_id': 'alias',
                            'target': "horizon_client::game::native_hooks::Game<'_>::new_block_entity"}),
        ]:
            with self.subTest(section=section, entry=entry):
                plan = {section: [entry]}
                names = PREFLIGHT.required_guards(plan, requested)
                self.assertEqual(names, ['block_entities', 'protection', 'structures'])
                self.assertEqual(PREFLIGHT.required_guards(plan, names), names)
        self.assertEqual(requested, ['protection'])

    def test_cli_view_omits_only_trait_metadata_and_preserves_all_execution_records(self):
        source = '''[mod]
id="example"
version="1.0.0"
[compatibility]
horizon_revision="pin"
[api]
path="api"
[dependencies]
pluto_lib="^1"
[[module]]
id="module"
file="game.rs"
name="extension"
source="extension.rs"
[[injection]]
id="hook"
target="game::run"
kind="HEAD"
helper="hook.rs"
[[state]]
id="state"
target="game::World"
ty="u32"
initial="0"
[[state.constructor]]
target="game::World::new"
selector="Self"
require=1
[[crate]]
id="support"
path="support"
[[post_build]]
id="finish"
crate="support"
bin="finish"
'''
        original = tomllib.loads(source)
        expected = {key: value for key, value in original.items() if key not in ('api', 'dependencies')}
        actual = tomllib.loads(PREFLIGHT.cli_manifest(source))
        self.assertEqual(actual, expected)
        self.assertEqual(tomllib.loads(source), original)
        self.assertEqual(tomllib.loads(PREFLIGHT.cli_manifest(PREFLIGHT.cli_manifest(source))), expected)
        self.assertTrue(PREFLIGHT.has_operations(expected))
        api_only = {'mod': {'id': 'consumer', 'version': '1'}, 'api': {'path': 'api'}}
        self.assertFalse(PREFLIGHT.has_operations(api_only))
        for section in ('injection', 'bridge', 'module', 'state', 'crate', 'post_build'):
            with self.subTest(operation=section):
                self.assertTrue(PREFLIGHT.has_operations(dict(api_only, **{section: [{'id': 'kept'}]})))
