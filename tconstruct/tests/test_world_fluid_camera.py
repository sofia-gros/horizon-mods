"""Deferred camera declaration, original-image and package contracts."""
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'content'
MANIFEST = CONTENT / 'reference/world-fluid-camera-assets.json'


class WorldFluidCameraTests(unittest.TestCase):
    def test_all_placeable_families_use_source_camera_defaults(self):
        paths = list((CONTENT / 'data/tconstruct/registered_fluid').glob('*.json'))
        self.assertEqual(len(paths), 70)
        textures = set()
        for path in paths:
            with self.subTest(fluid=path.stem):
                visual = json.loads(path.read_text())['visual']
                # The source named() builder calls camera() on the still root.
                texture = visual['still'].rsplit('/', 1)[0] + '/camera'
                self.assertEqual(visual['camera'], {'texture': texture, 'opacity': 0.1})
                namespace, texture_path = texture.split(':', 1)
                textures.add(f'assets/{namespace}/textures/{texture_path}.png')
        manifest = json.loads(MANIFEST.read_text())
        self.assertEqual(manifest['families'], 70)
        self.assertEqual(textures, set(manifest['files']))
        self.assertEqual(len(textures), 61)

    def test_exact_upstream_static_camera_images(self):
        manifest = json.loads(MANIFEST.read_text())
        self.assertEqual(manifest['revision'], 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2')
        for relative, entry in manifest['files'].items():
            with self.subTest(texture=relative):
                data = (CONTENT / relative).read_bytes()
                self.assertEqual(hashlib.sha256(data).hexdigest(), entry['sha256'])
                git_blob = b'blob ' + str(len(data)).encode() + b'\0' + data
                self.assertEqual(hashlib.sha1(git_blob).hexdigest(), entry['git_blob_sha'])
                self.assertEqual(entry['source'], 'src/generated/resources/' + relative)
                self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
                width, height = struct.unpack('>II', data[16:24])
                self.assertEqual(width, height)
                self.assertIn(width, (16, 32))
                self.assertFalse((CONTENT / (relative + '.mcmeta')).exists())

    def test_camera_declarations_and_assets_survive_packaging(self):
        sys.path.insert(0, str(ROOT / 'tools'))
        try:
            spec = importlib.util.spec_from_file_location('camera_package', ROOT / 'tools/package_hmod.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            with tempfile.TemporaryDirectory() as tmp:
                target = Path(tmp) / 'tconstruct.hmod'
                module.package(target)
                with zipfile.ZipFile(target) as archive:
                    self.assertEqual(archive.read(MANIFEST.relative_to(CONTENT).as_posix()), MANIFEST.read_bytes())
                    for relative in json.loads(MANIFEST.read_text())['files']:
                        self.assertEqual(archive.read(relative), (CONTENT / relative).read_bytes())
                    for path in (CONTENT / 'data/tconstruct/registered_fluid').glob('*.json'):
                        packed = json.loads(archive.read(path.relative_to(CONTENT).as_posix()))
                        self.assertEqual(packed['visual']['camera'], json.loads(path.read_text())['visual']['camera'])
        finally:
            sys.path.pop(0)


if __name__ == '__main__':
    unittest.main()
