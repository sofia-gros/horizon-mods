"""Keep the content package within Horizon's loader limit and retain renderer aliases."""
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

MOD = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(MOD / 'tools'))
from package_hmod import package


class ContentPackageTest(unittest.TestCase):
    def test_loader_limit_and_source_sprite_aliases(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'tconstruct.hmod'
            package(output)
            with zipfile.ZipFile(output) as packed:
                names = set(packed.namelist())
            self.assertLessEqual(len(names - {'horizon-mod.toml'}), 65_536)
            for kind in ('part', 'tool'):
                with zipfile.ZipFile(MOD / 'content/reference' / f'tconstruct-{kind}-visuals.zip') as source:
                    sprites = [n.removeprefix('assets/tconstruct/textures/').removesuffix('.png')
                               for n in source.namelist()
                               if n.startswith('assets/tconstruct/textures/') and n.endswith('.png')]
                for sprite in sprites:
                    alias = f'tconstruct_{kind}/{sprite}.json'
                    self.assertIn(f'assets/tconstruct/items/{alias}', names)
                    self.assertIn(f'assets/tconstruct/models/item/{alias}', names)


if __name__ == '__main__':
    unittest.main()
