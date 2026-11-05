"""Generate alloy-specific Anvil models and preserve their identity in recipes."""
import json
from pathlib import Path
root=Path(__file__).resolve().parents[1] / 'content'; asset=root/'assets/tconstruct'; data=root/'data/tconstruct'
metals=['slimesteel','amethyst_bronze','rose_gold','pig_iron','cinderslime','queens_slime','manyullyn','hepatizon','knightmetal','knightslime','netherite']
for anvil in ('tinkers_anvil','scorched_anvil'):
 for metal in metals:
  texture='minecraft:block/netherite_block' if metal=='netherite' else f'tconstruct:block/storage/{metal}' + ('_side' if metal=='queens_slime' else '/top' if metal=='pig_iron' else '')
  name=f'{anvil}_{metal}'
  models={f'models/block/table/{name}.json':{'parent':f'tconstruct:block/table/{anvil}','textures':{'particle':texture,'top':texture}},f'models/item/{name}.json':{'parent':f'tconstruct:block/table/{name}'},f'items/{name}.json':{'model':{'type':'minecraft:model','model':f'tconstruct:item/{name}'}}}
  for path,value in models.items():
   dest=asset/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(json.dumps(value,separators=(',',':'))+'\n')
  file=data/'recipe'/f'{name}.json';value=json.loads(file.read_text());value['key']['s']='#tconstruct:' + ('seared_blocks' if anvil == 'tinkers_anvil' else 'scorched_blocks');value['result']['components']={'minecraft:custom_data':{'tconstruct:anvil':{'Metal':value['key']['m'],'Variant':metal,'Texture':texture}},'minecraft:item_model':f'tconstruct:{name}'};file.write_text(json.dumps(value,indent=2)+'\n')

# Forge compatibility alloys use material-bearing fake storage blocks. The
# shared crafting guard checks the source Material on all three inputs.
for anvil, tag in [('tinkers_anvil', 'seared_blocks'), ('scorched_anvil', 'scorched_blocks')]:
 for metal in ('bronze', 'constantan', 'invar', 'electrum', 'pewter', 'nicrosil'):
  variant = 'compat_' + metal
  name = f'{anvil}_{variant}'
  material = f'tconstruct:{metal}'
  texture = f'tconstruct:block/storage/fallback_tconstruct_{metal}'
  models = {f'models/block/table/{name}.json': {'parent': f'tconstruct:block/table/{anvil}', 'textures': {'particle': texture, 'top': texture}},
            f'models/item/{name}.json': {'parent': f'tconstruct:block/table/{name}'},
            f'items/{name}.json': {'model': {'type': 'minecraft:model', 'model': f'tconstruct:item/{name}'}}}
  for path, value in models.items():
   dest = asset / path; dest.parent.mkdir(parents=True, exist_ok=True); dest.write_text(json.dumps(value, separators=(',', ':')) + '\n')
  value = {'type': 'minecraft:crafting_shaped', 'category': 'misc', 'pattern': ['mmm', ' s ', 'sss'],
           'key': {'m': 'tconstruct:fake_storage_block', 's': f'#tconstruct:{tag}'},
           'result': {'id': f'tconstruct:{anvil}', 'components': {'minecraft:custom_data': {'tconstruct:anvil': {
               'Metal': material, 'RequiredMaterial': material, 'Variant': variant, 'Texture': texture}},
               'minecraft:item_model': f'tconstruct:{name}'}}}
  (data / 'recipe' / f'{name}.json').write_text(json.dumps(value, indent=2) + '\n')

# Mixed ordinary alloy blocks match the original shaped recipe but use its
# fallback appearance. Sorted after the specific material-preserving variants.
for anvil, tag in [('tinkers_anvil', 'seared_blocks'), ('scorched_anvil', 'scorched_blocks')]:
 blocks = [f'tconstruct:{metal}_block' for metal in metals if metal != 'netherite'] + ['minecraft:netherite_block']
 value = {'type': 'minecraft:crafting_shaped', 'category': 'misc', 'pattern': ['mmm', ' s ', 'sss'],
          'key': {'m': blocks, 's': f'#tconstruct:{tag}'}, 'result': {'id': f'tconstruct:{anvil}'}}
 (data / 'recipe' / f'zz_{anvil}_mixed_alloys.json').write_text(json.dumps(value, indent=2) + '\n')
