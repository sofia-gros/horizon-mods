"""Text/signature checks only; does not compile or run the Rust example."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


class FluidAuthorExample(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = json.loads((HERE / 'inputs.json').read_text())
        cls.sources = {}
        for key, env in [('base', 'PLUTO_MOD_SOURCE'), ('render_source_only', 'PLUTO_RENDER_SOURCE')]:
            evidence = cls.inputs[key]
            for path, expected in evidence['sha256'].items():
                raw = subprocess.check_output(['git', '-C', os.environ[env], 'show',
                                               evidence['commit'] + ':' + path])
                if hashlib.sha256(raw).hexdigest() != expected:
                    raise ValueError('exact MOD source mismatch: ' + path)
                cls.sources[path] = raw.decode()
        cls.example = (HERE / 'example.rs').read_text()
        cls.doc = (ROOT / 'pluto-lib/docs/fluid-mod-author-style.md').read_text()

    def test_registration_is_existing_definition_and_batch(self):
        src = self.sources['pluto-lib/mixins/pluto_native_creative/api/src/fluid.rs']
        body = src.split('pub struct Definition {', 1)[1].split('}', 1)[0]
        expected = {'id': 'String', 'block': 'String', 'bucket': 'String', 'texture': 'String',
                    'tick_delay': 'u16', 'drop_off': 'u8', 'light': 'u8'}
        self.assertEqual([line.strip() for line in body.splitlines() if line.strip()],
                         [f'pub {name}: {kind},' for name, kind in expected.items()])
        self.assertIn('pub fn register_all(definitions: impl IntoIterator<Item = Definition>) -> Result<(), String>', src)
        self.assertIn('fluid::register_all([Definition {', self.example)
        for field in expected:
            self.assertIn(field + ':', self.example)
        self.assertIn('pub const RAINWATER: &str = "meadow:rainwater"', self.example)
        for value in ['tick_delay: 5', 'drop_off: 1', 'light: 0']:
            self.assertIn(value, self.example)

    def test_read_closure_is_query_not_event_or_actor(self):
        src = self.sources['pluto-lib/mixins/pluto_native_creative/api/src/fluid/contact.rs']
        self.assertIn('pub fn query_box_with(\n    bounds: Bounds,\n    limits: Limits,\n    mut sample: impl FnMut(Position) -> Sample,\n) -> Result<Contacts, QueryError>', src)
        self.assertIn('pub struct Contacts', src)
        self.assertIn('pub fluids: BTreeMap<String, Contact>', src)
        self.assertIn('pub fn is_complete(&self) -> bool', src)
        self.assertIn('contact::query_box_with(bounds, contact::Limits { max_cells: 4096 }, sample)', self.example)
        self.assertLess(self.example.index('if !contacts.is_complete()'), self.example.index('Some(contacts.fluids.contains_key(RAINWATER))'))
        self.assertIn('return None;', self.example)
        for forbidden in ['&mut Game', 'Event::', 'EVENT.register', 'register_contact', 'add_player_effect', 'Sample::Air']:
            self.assertNotIn(forbidden, self.example)

    def test_distinct_cutoffs_and_real_preparation_order_are_labeled(self):
        self.assertNotEqual(self.inputs['base']['commit'], self.inputs['render_source_only']['commit'])
        for group in ['base', 'render_source_only']:
            for key in ['commit', 'published']:
                self.assertIn(self.inputs[group][key], self.doc)
        material = self.sources['pluto-lib/docs/terrain-materials.md']
        self.assertIn('BEFORE the `native_blocks`/content-root path', material)
        self.assertIn('without replacing the fluid registry', self.doc)
        self.assertIn('before the old `native_blocks`', self.doc)
        self.assertIn('not proof of upward flow', self.doc)
        self.assertNotIn('RenderDefinition', self.example)

    def test_no_new_api_or_compatibility_and_no_claimed_native_pass(self):
        for phrase in ['not Java syntax, Fabric binary compatibility',
                       'does not expose a multi-listener fluid-contact event',
                       'not an event listener', 'it has\nnot been compiled or run',
                       '2026-11-06']:
            self.assertIn(phrase, self.doc)
        for url in self.inputs['fabric_references']['urls']:
            self.assertIn(url, self.doc)
        self.assertEqual(self.inputs['native_compile'], 'not_run')
        self.assertEqual(self.inputs['runtime'], 'not_run')
        self.assertFalse(self.inputs['production_delta'])
        for path, expected in self.inputs['artifact_sha256'].items():
            self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), expected)


if __name__ == '__main__':
    unittest.main()
