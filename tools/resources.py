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
out.mkdir(parents=True)
for path in src.iterdir():
    if path.name not in {'items.json', 'registries.json', 'command-registries.json'}:
        (out / path.name).symlink_to(path, target_is_directory=path.is_dir())
items = json.loads((src / 'items.json').read_text())
entries = [
    ('tconstruct:flint_pick_head', 'minecraft:flint', 'Flint Pick Head'),
    ('tconstruct:wood_binding', 'minecraft:stick', 'Wood Tool Binding'),
    ('tconstruct:wood_handle', 'minecraft:stick', 'Wood Tool Handle'),
    ('tconstruct:flint_pickaxe', 'minecraft:stone_pickaxe', 'Flint Pickaxe'),
]
for name, template, title in entries:
    item = copy.deepcopy(items[template])
    item['minecraft:item_name'] = {'text': title}
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
