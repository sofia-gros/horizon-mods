"""Generate alloy-specific Anvil models and preserve their identity in recipes."""
import json
from pathlib import Path
root=Path(__file__).resolve().parents[1] / 'content'; asset=root/'assets/tconstruct'; data=root/'data/tconstruct'
metals=['slimesteel','amethyst_bronze','rose_gold','cinderslime','queens_slime','manyullyn','hepatizon','knightslime','netherite']
for anvil in ('tinkers_anvil','scorched_anvil'):
 for metal in metals:
  texture='minecraft:block/netherite_block' if metal=='netherite' else f'tconstruct:block/storage/{metal}' + ('_side' if metal=='queens_slime' else '')
  name=f'{anvil}_{metal}'
  models={f'models/block/table/{name}.json':{'parent':f'tconstruct:block/table/{anvil}','textures':{'particle':texture,'top':texture}},f'models/item/{name}.json':{'parent':f'tconstruct:block/table/{name}'},f'items/{name}.json':{'model':{'type':'minecraft:model','model':f'tconstruct:item/{name}'}}}
  for path,value in models.items():
   dest=asset/path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(json.dumps(value,separators=(',',':'))+'\n')
  file=data/'recipe'/f'{name}.json';value=json.loads(file.read_text());value['result']['components']={'minecraft:custom_data':{'tconstruct:anvil':{'Metal':metal,'Texture':texture}},'minecraft:item_model':f'tconstruct:{name}'};file.write_text(json.dumps(value,indent=2)+'\n')
