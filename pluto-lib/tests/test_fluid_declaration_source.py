"""Source-only checks for the additive declaration helper, not native execution."""
from pathlib import Path
import hashlib
import unittest

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parent
BASE_FLUID_SHA256 = '180ba04c5507535441011ea40e255cdd1e8373462056026dd66daf112b12e43a'
PACKAGER_SHA256 = '1b7888725401aebfe09185239ce9864e2d11e71b4b84b1ee42031dd7258c3e6e'
FLUID = 'pluto-lib/mixins/pluto_native_creative/api/src/fluid.rs'
MODULE = ROOT / 'mixins/pluto_native_creative/api/src/fluid/declaration.rs'


class DeclarationSource(unittest.TestCase):
    def test_original_fluid_body_and_sole_registry_remain_unchanged(self):
        current = (REPO / FLUID).read_text()
        addition = 'mod declaration;\npub use declaration::{register_declarations, Declaration};\n'
        self.assertEqual(current.count(addition), 1)
        self.assertEqual(hashlib.sha256(current.replace(addition, '').encode()).hexdigest(), BASE_FLUID_SHA256)
        self.assertEqual(current.count('static REGISTRY:'), 1)

    def test_owned_fields_and_validation_have_no_implicit_defaults(self):
        source = MODULE.read_text().split('#[cfg(test)]')[0]
        self.assertIn('pub struct Declaration {\n    definition: Definition,\n    render: RenderDefinition,\n}', source)
        self.assertIn('definition.validate()?;\n        render.validate()?;', source)
        self.assertIn('pub fn definition(&self) -> &Definition', source)
        self.assertIn('pub fn render(&self) -> &RenderDefinition', source)
        for forbidden in ['default_for(', 'impl Default', 'static ', 'OnceLock', 'RwLock', 'Mutex', 'unsafe']:
            self.assertNotIn(forbidden, source)

    def test_all_operations_delegate_without_another_owner(self):
        source = MODULE.read_text().split('#[cfg(test)]')[0]
        self.assertEqual(source.count('super::content_with_render(&self.definition, &self.render)'), 1)
        self.assertEqual(source.count('super::render_material_model(&self.definition, &self.render)'), 1)
        self.assertEqual(source.count('super::register_all_with_render('), 1)
        self.assertIn('declarations.into_iter().map(|entry| (entry.definition, entry.render))', source)
        for forbidden in ['Game', 'schedule(', 'replace(', 'Event::', 'register_contact', 'world.write']:
            self.assertNotIn(forbidden, source)

    def test_nested_packaging_and_unrun_tests_are_explicit(self):
        raw = (ROOT / 'tools/package_native.py').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), PACKAGER_SHA256)
        packager = raw.decode()
        self.assertIn('(crate / "src").rglob("*")', packager)
        self.assertIn('path.suffix == ".rs"', packager)
        source = MODULE.read_text()
        self.assertEqual(source.count('#[test]'), 6)
        self.assertIn('register_declarations([fresh, conflict]).is_err()', source)
        self.assertIn('fluid::register(original.definition().clone()).unwrap()', source)
        doc = (ROOT / 'docs/fluid-declarations.md').read_text()
        self.assertIn('prepared but unrun', doc)
        self.assertIn('differs from legacy register(def)', doc)
        self.assertIn('fluid-contact event remains unimplemented', doc)


if __name__ == '__main__':
    unittest.main()
