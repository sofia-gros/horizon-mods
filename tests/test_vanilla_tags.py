import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('vanilla_tags', ROOT / 'tools/vanilla_tags.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class VanillaTags(unittest.TestCase):
    def test_extraction_and_reject_existing_or_wrong_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            jar = root / 'client.jar'
            with zipfile.ZipFile(jar, 'w') as archive:
                archive.writestr('data/minecraft/tags/items/planks.json', '{"values":[]}')
                archive.writestr('assets/irrelevant', 'ignore')
            digest = hashlib.sha1(jar.read_bytes()).hexdigest()
            out = root / 'pack'
            with self.assertRaisesRegex(ValueError, 'does not match'):
                module.extract(jar, out)
            self.assertFalse(out.exists())
            self.assertEqual(module.extract(jar, out, digest), 1)
            self.assertEqual((out / 'data/minecraft/tags/items/planks.json').read_text(), '{"values":[]}')
            with self.assertRaisesRegex(ValueError, 'already exists'):
                module.extract(jar, out, digest)

    def test_forge_tags_use_sha256_and_their_own_namespace(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            jar = root / 'forge.jar'
            with zipfile.ZipFile(jar, 'w') as archive:
                archive.writestr('data/forge/tags/items/ingots/iron.json', '{"values":["minecraft:iron_ingot"]}')
                archive.writestr('data/minecraft/tags/items/planks.json', '{"values":[]}')
            digest = hashlib.sha256(jar.read_bytes()).hexdigest()
            out = root / 'forge-pack'
            self.assertEqual(module.extract(jar, out, digest, prefix=module.FORGE_PREFIX, algorithm='sha256'), 1)
            self.assertTrue((out / 'data/forge/tags/items/ingots/iron.json').is_file())
            self.assertFalse((out / 'data/minecraft').exists())
