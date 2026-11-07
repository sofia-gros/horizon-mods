"""The decoder fixture permits only its exact fixture and actual API identity."""
from pathlib import Path
import importlib.util
import sys
import unittest

TOOLS = Path(__file__).resolve().parents[1] / 'tools'
sys.path.insert(0, str(TOOLS))
spec = importlib.util.spec_from_file_location('fluid_query_fixture', TOOLS / 'prepare_fluid_query_fixture.py')
fixture = importlib.util.module_from_spec(spec); spec.loader.exec_module(fixture)


class FluidQueryFixtureGuard(unittest.TestCase):
    def test_requires_both_exact_local_crates(self):
        packages = [{'name': ident[0], 'version': ident[1]} for ident in (fixture.FIXTURE_IDENTITY, fixture.API_IDENTITY)]
        fixture.check_lock({'package': []}, {'package': packages})
        for selected in (packages[:1], packages[1:]):
            with self.assertRaises(ValueError):
                fixture.check_lock({'package': []}, {'package': selected})

    def test_rejects_unknown_or_changed_dependency_identity(self):
        packages = [{'name': ident[0], 'version': ident[1]} for ident in (fixture.FIXTURE_IDENTITY, fixture.API_IDENTITY)]
        for extra in ({'name':'other', 'version':'0.0.0'},
                      {'name':'pluto-api', 'version':'0.1.1'},
                      {'name':'pluto-api', 'version':'0.1.0', 'source':'registry+unexpected'}):
            with self.assertRaises(ValueError):
                fixture.check_lock({'package': []}, {'package': packages + [extra]})

    def test_minimal_loader_names_are_strict_and_preserve_alias_identity(self):
        source = 'named_blocks! {\n    air = "air",\n    cave_air = "cave_air",\n    air_alias = "air",\n}\n'
        self.assertEqual(fixture.required_blocks(source), ['minecraft:air', 'minecraft:cave_air'])
        for bad in ('', 'named_blocks! {\n    stone = "stone",\n}\n',
                    source.replace('air_alias = "air",', 'unexpected syntax')):
            with self.assertRaises(ValueError):
                fixture.required_blocks(bad)

    def test_world_fixture_uses_real_loaders_and_no_skip_fallback(self):
        source = (TOOLS.parent / 'tests/native/world-fluid-view.rs').read_text()
        for required in ('BlockRegistry::from_nbt', 'Behaviours::load(&registry, &root)',
                         'nbt::write_named', 'World::new', 'Chunk::with_layout',
                         'LevelShape::NETHER', 'minecraft:cave_air', 'membership_revision()'):
            self.assertIn(required, source)
        for forbidden in ('HORIZON_RESOURCES', 'return None', 'struct World', 'struct Behaviours',
                          'struct BlockRegistry', 'set_cache_dir', 'unsafe'):
            self.assertNotIn(forbidden, source)

    def test_real_api_manifest_and_all_rust_sources_are_hashed(self):
        api = TOOLS.parent / 'mixins/pluto_native_creative/api'
        hashes = fixture.api_inputs(api)
        self.assertIn('Cargo.toml', hashes)
        for path in ('src/lib.rs', 'src/fluid.rs', 'src/fluid/contact.rs'):
            self.assertIn(path, hashes)
        self.assertEqual(len(hashes), 1 + len(list((api / 'src').rglob('*.rs'))))


if __name__ == '__main__':
    unittest.main()
