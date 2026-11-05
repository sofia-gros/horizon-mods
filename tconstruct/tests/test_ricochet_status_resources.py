"""Source Ricochet status identity and original icon packaging, deferred tests."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'


class RicochetStatusResources(unittest.TestCase):
    def test_source_effect_and_icon_are_packaged(self):
        definition = json.loads((CONTENT / 'data/tconstruct/horizon/mob_effect/ricochet.json').read_text())
        self.assertEqual(definition, {'category': 'neutral', 'color': 0x01CBCD})
        manifest = json.loads((CONTENT / 'reference/slime-effect-assets.json').read_text())
        self.assertEqual(manifest['revision'], 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2')
        path = 'assets/tconstruct/textures/mob_effect/ricochet.png'
        data = (CONTENT / path).read_bytes()
        self.assertEqual(hashlib.sha256(data).hexdigest(), manifest['files'][path]['sha256'])
        self.assertEqual(manifest['files'][path]['source'], 'src/main/resources/' + path)
        sys.path.insert(0, str(ROOT / 'tools'))
        try:
            spec = importlib.util.spec_from_file_location('ricochet_package', ROOT / 'tools/package_hmod.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            with tempfile.TemporaryDirectory() as tmp:
                target = Path(tmp) / 'tconstruct.hmod'
                module.package(target)
                with zipfile.ZipFile(target) as archive:
                    self.assertEqual(archive.read(path), data)
                    self.assertEqual(json.loads(archive.read('data/tconstruct/horizon/mob_effect/ricochet.json')), definition)
                    sky = json.loads(archive.read('data/tconstruct/registered_fluid/sky_slime.json'))
                    self.assertEqual(sky['interaction']['effect'], {
                        'id': 'tconstruct:ricochet', 'duration_ticks': 100,
                        'amplifier': 0, 'no_curatives': True})
        finally:
            sys.path.pop(0)


if __name__ == '__main__':
    unittest.main()
