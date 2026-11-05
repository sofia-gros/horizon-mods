"""Deferred slime-fluid drowning policy and package contracts."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
FLUIDS = CONTENT / 'data/tconstruct/registered_fluid'
TAG = CONTENT / 'data/forge/tags/entity_type/slimes.json'
SLIME_FLUIDS = {'earth_slime', 'sky_slime', 'ender_slime', 'magma', 'ichor'}
SLIME_TYPES = {
    'minecraft:slime', 'minecraft:magma_cube', 'tconstruct:sky_slime',
    'tconstruct:ender_slime', 'tconstruct:terracube',
}


class WorldFluidDrowningTests(unittest.TestCase):
    def test_only_source_slime_fluid_types_exclude_the_source_shared_tag(self):
        declarations = {p.stem: json.loads(p.read_text()) for p in FLUIDS.glob('*.json')}
        self.assertEqual(len(declarations), 70)
        declared = {name for name, data in declarations.items() if 'drowning_immune_tag' in data}
        self.assertEqual(declared, SLIME_FLUIDS)
        for name in SLIME_FLUIDS:
            with self.subTest(fluid=name):
                self.assertEqual(declarations[name]['drowning_immune_tag'], 'forge:slimes')
                self.assertTrue(declarations[name]['interaction']['can_drown'])
        tag = json.loads(TAG.read_text())
        self.assertEqual(set(tag['values']), SLIME_TYPES)
        self.assertFalse(tag.get('replace', False))
        provenance = json.loads((CONTENT / 'reference/world-fluid-drowning-source.json').read_text())
        self.assertEqual(set(provenance['fluids']), SLIME_FLUIDS)
        self.assertEqual(set(provenance['entity_types']), SLIME_TYPES)

    def test_fluid_immunity_and_entity_tag_survive_content_packaging(self):
        sys.path.insert(0, str(ROOT / 'tools'))
        try:
            spec = importlib.util.spec_from_file_location('drowning_package', ROOT / 'tools/package_hmod.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            with tempfile.TemporaryDirectory() as tmp:
                target = Path(tmp) / 'tconstruct.hmod'
                module.package(target)
                with zipfile.ZipFile(target) as archive:
                    paths = [TAG] + [FLUIDS / f'{name}.json' for name in SLIME_FLUIDS]
                    for path in paths:
                        packed = json.loads(archive.read(path.relative_to(CONTENT).as_posix()))
                        self.assertEqual(packed, json.loads(path.read_text()))
        finally:
            sys.path.pop(0)


if __name__ == '__main__':
    unittest.main()
