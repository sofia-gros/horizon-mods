#!/usr/bin/env python3
"""Create a separate, disposable resource overlay for the prototype."""
import argparse, copy, json
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument('source', type=Path)
p.add_argument('output', type=Path)
a = p.parse_args()
src = a.source.resolve(strict=True)
out = a.output.absolute()
if out.exists() or out.is_relative_to(src) or src.is_relative_to(out):
    p.error('output must not exist and must be separate from source')
if (src / 'assets/tconstruct').exists():
    p.error('source already contains a tconstruct asset namespace')
out.mkdir(parents=True)
for path in src.iterdir():
    if path.name not in {'items.json', 'registries.json', 'command-registries.json', 'assets'}:
        (out / path.name).symlink_to(path, target_is_directory=path.is_dir())
assets = out / 'assets'
assets.mkdir()
for namespace in (src / 'assets').iterdir():
    if namespace.name not in {'tconstruct', 'minecraft'}:
        (assets / namespace.name).symlink_to(namespace, target_is_directory=namespace.is_dir())
vanilla = assets / 'minecraft'
vanilla.mkdir()
for child in (src / 'assets/minecraft').iterdir():
    if child.name != 'textures':
        (vanilla / child.name).symlink_to(child, target_is_directory=child.is_dir())
textures = vanilla / 'textures'
textures.mkdir()
for child in (src / 'assets/minecraft/textures').iterdir():
    if child.name != 'gui':
        (textures / child.name).symlink_to(child, target_is_directory=child.is_dir())
gui = textures / 'gui'
gui.mkdir()
for child in (src / 'assets/minecraft/textures/gui').iterdir():
    (gui / child.name).symlink_to(child, target_is_directory=child.is_dir())
for name in ('part_builder', 'tinker_station'):
    (gui / ('tconstruct_' + name + '.png')).write_bytes((Path(__file__).resolve().parents[1] / 'assets/tconstruct/gui' / (name + '.png')).read_bytes())
mod_assets = assets / 'tconstruct'
mod_assets.mkdir()
(mod_assets / 'textures').symlink_to(Path(__file__).resolve().parents[1] / 'assets/tconstruct/textures', target_is_directory=True)
(mod_assets / 'items').mkdir()
(mod_assets / 'models/item').mkdir(parents=True)
items = json.loads((src / 'items.json').read_text())
entries = [
    ('tconstruct:pattern', 'minecraft:paper', 'Pattern'),
    ('tconstruct:flint_pick_head', 'minecraft:flint', 'Flint Pick Head'),
    ('tconstruct:wood_binding', 'minecraft:stick', 'Wood Tool Binding'),
    ('tconstruct:wood_handle', 'minecraft:stick', 'Wood Tool Handle'),
    ('tconstruct:flint_pickaxe', 'minecraft:stone_pickaxe', 'Flint Pickaxe'),
]
for name, template, title in entries:
    item = copy.deepcopy(items[template])
    item['minecraft:item_name'] = {'text': title}
    short = name.split(':')[1]
    item['minecraft:item_model'] = name
    layers = ['pickaxe_handle', 'pickaxe_head', 'pickaxe_binding'] if short == 'flint_pickaxe' else [short]
    (mod_assets / 'items' / (short + '.json')).write_text(json.dumps({'model': {'type':'minecraft:model','model':'tconstruct:item/' + short}}))
    (mod_assets / 'models/item' / (short + '.json')).write_text(json.dumps({'parent': 'minecraft:item/handheld' if short == 'flint_pickaxe' else 'minecraft:item/generated', 'textures': {f'layer{i}': 'tconstruct:item/' + layer for i, layer in enumerate(layers)}}))
    if name.endswith('pickaxe'):
        item['minecraft:max_damage'] = 85
        item['minecraft:tool']['rules'][-1]['speed'] = 3.5
    items[name] = item
(out / 'items.json').write_text(json.dumps(items))
commands = json.loads((src / 'command-registries.json').read_text())
commands['items'].extend(name for name, _, _ in entries)
item_registry = commands.get('by_registry', {}).get('minecraft:item')
if item_registry is not None:
    item_registry['ids'].extend(name for name, _, _ in entries)
(out / 'command-registries.json').write_text(json.dumps(commands))
registries = json.loads((src / 'registries.json').read_text())
ids = registries['minecraft:item']['entries']
next_id = max(v['protocol_id'] for v in ids.values()) + 1
for index, (name, _, _) in enumerate(entries):
    ids[name] = {'protocol_id': next_id + index}
(out / 'registries.json').write_text(json.dumps(registries))
