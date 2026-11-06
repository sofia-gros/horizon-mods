"""Pure codec, caller-policy and actual Cargo integration boundaries."""
import hashlib
import json
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / 'tconstruct/tests/builder-wire'
META = json.loads((FIXTURE / 'provenance.json').read_text())


class MenuWordsBoundaries(unittest.TestCase):
    def test_legacy_oracle_and_transport_callers_are_unchanged(self):
        old = (FIXTURE / 'legacy_codec.rs').read_bytes()
        self.assertEqual(hashlib.sha256(old).hexdigest(), META['legacy_sha256'])
        for path, expected in META['unchanged_callers'].items():
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), expected)

    def test_wrapper_delegates_only_codec_and_retains_reject_policy(self):
        old = (FIXTURE / 'legacy_codec.rs').read_text()
        new = (ROOT / 'tconstruct/mixins/tconstruct/source-builder-wire.rs').read_text()
        self.assertEqual(new[new.index('#[cfg(test)]\nmod tests'):], old[old.index('#[cfg(test)]'):])
        self.assertIn('menu_words::encode_utf8_list(patterns)', new)
        self.assertIn('size, words, horizon_client::pluto_inventory::menu_words::EmptyStrings::Reject,', new)
        self.assertNotIn('EmptyStrings::Allow', new)

    def test_real_api_and_locked_seed_are_the_same_dependencies(self):
        manifest = FIXTURE / 'Cargo.toml'
        dep = tomllib.loads(manifest.read_text())['dependencies']['pluto-inventory-api']['path']
        self.assertEqual((FIXTURE / dep).resolve(), ROOT / 'pluto-lib/mixins/pluto_native_creative/inventory-api')
        packages = tomllib.loads(manifest.with_name('Cargo.lock').read_text())['package']
        seed = tomllib.loads((ROOT / 'pluto-lib/tests/inventory-api.Cargo.lock').read_text())['package']
        key = lambda p: (p['name'], p['version'], p.get('source'), p.get('checksum'))
        self.assertEqual({key(p) for p in packages if 'source' in p}, {key(p) for p in seed if 'source' in p})
        api = [p for p in packages if p['name'] == 'pluto-inventory-api']
        self.assertEqual(len(api), 1)
        self.assertNotIn('source', api[0])

    def test_all_rust_paths_resolve_to_the_real_unchanged_sources(self):
        source = ROOT / 'tconstruct/tests/rust/source_builder_wire.rs'
        manifest = tomllib.loads((FIXTURE / 'Cargo.toml').read_text())
        self.assertEqual((FIXTURE / manifest['lib']['path']).resolve(), source)
        for file in [source, FIXTURE / 'compatibility.rs']:
            for path in re.findall(r'#\[path = "([^"]+)"\]', file.read_text()):
                self.assertTrue((file.parent / path).resolve().is_file(), (file, path))
        self.assertIn('pub use pluto_inventory_api as pluto_inventory;', source.read_text())

    def test_workflow_fetches_locked_seed_before_offline_fixture(self):
        workflow = (ROOT / '.github/workflows/mixins.yml').read_text()
        step = 'cargo test --locked --offline --manifest-path tconstruct/tests/builder-wire/Cargo.toml'
        seed = 'PLUTO_TEST_FETCH_LOCKED=1 python3 -m unittest discover -s pluto-lib/tests -v'
        self.assertEqual(workflow.count(step), 1)
        self.assertLess(workflow.index(seed), workflow.index(step))
        self.assertNotIn('rustc --test tconstruct/tests/rust/source_builder_wire.rs', workflow)

    def test_api_has_no_callbacks_transport_state_or_content_rules(self):
        src = ROOT / 'pluto-lib/mixins/pluto_native_creative/inventory-api/src'
        self.assertIn('pub mod menu_words;', (src / 'lib.rs').read_text())
        module = (src / 'menu_words.rs').read_text()
        for forbidden in ['tconstruct:', 'forge:', 'minecraft:', 'horizon_', 'FnMut', 'FnOnce', 'thread_local!', 'Mutex', 'static ', 'unsafe']:
            self.assertNotIn(forbidden, module)
        self.assertIn('pub enum EmptyStrings { Allow, Reject }', module)


if __name__ == '__main__':
    unittest.main()
