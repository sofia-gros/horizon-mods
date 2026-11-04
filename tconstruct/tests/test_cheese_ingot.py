"""Bounded source, registration and casting-data checks for Cheese Ingot."""
import hashlib, json, sys, tempfile, unittest, zipfile
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'tools'))
from package_hmod import package

def git_blob(data):
    return hashlib.sha1(f'blob {len(data)}\0'.encode()+data).hexdigest()

class CheeseIngot(unittest.TestCase):
    def test_fixed_source_food_and_assets(self):
        source = (ROOT/'tests/fixtures/tinker_food_source.java').read_bytes()
        self.assertEqual(git_blob(source), 'f3156aa8a0793b6e1d3de368e30311f6d3c46c57')
        self.assertIn('CHEESE = (new FoodProperties.Builder()).nutrition(3).saturationMod(0.4F).build()', source.decode())
        for path, sha in [('assets/tconstruct/textures/item/materials/cheese_ingot.png','4e33cf438ee3612af40e3696e7313dd1311593cb'), ('assets/tconstruct/models/item/cheese_ingot.json','f08300ee121b8c3c9738ed8dc9c1ebe49eaffbba')]:
            self.assertEqual(git_blob((ROOT/'content'/path).read_bytes()), sha)
        item = json.loads((ROOT/'content/data/tconstruct/horizon/item/cheese_ingot.json').read_text())
        self.assertEqual(item['template'], 'minecraft:cooked_porkchop')
        self.assertEqual(item['components']['minecraft:food'], {'nutrition':3,'saturation':2.4})

    def test_source_casting_and_package(self):
        with tempfile.TemporaryDirectory() as temp:
            output=Path(temp)/'tconstruct.hmod'; package(output)
            with zipfile.ZipFile(output) as z:
                for path in ('data/tconstruct/horizon/item/cheese_ingot.json', 'assets/tconstruct/items/cheese_ingot.json', 'assets/tconstruct/models/item/cheese_ingot.json', 'assets/tconstruct/textures/item/materials/cheese_ingot.png'):
                    self.assertEqual(z.read(path), (ROOT/'content'/path).read_bytes())
                self.assertNotIn('data/tconstruct/horizon/block/cheese_block.json', z.namelist())
            with zipfile.ZipFile(ROOT/'content/reference/tconstruct-recipes.zip') as z:
                for kind,consumed in [('gold',False),('sand',True)]:
                    r=json.loads(z.read(f'data/tconstruct/recipes/smeltery/casting/cheese_ingot_{kind}_cast.json'))
                    self.assertEqual(r['result'],'tconstruct:cheese_ingot')
                    self.assertEqual(r['fluid'],{'amount':250,'tag':'forge:milk'})
                    self.assertEqual(r['cooling_time'],2400)
                    self.assertEqual(r.get('cast_consumed',False),consumed)
if __name__ == '__main__': unittest.main()
