#!/usr/bin/env python3
"""Create a separate, disposable resource overlay for the prototype."""
import argparse, copy, hashlib, json, shutil, zipfile
from pathlib import Path
from import_part_visuals import MATERIAL_MODELS

p = argparse.ArgumentParser()
p.add_argument('source', type=Path)
p.add_argument('output', type=Path)
a = p.parse_args()
src = a.source.resolve(strict=True)
out = a.output.absolute()
if out.exists() or out.is_relative_to(src) or src.is_relative_to(out):
    p.error('output must not exist and must be separate from source')
if (src / 'THIRD_PARTY_LICENSES').exists():
    p.error('source already contains THIRD_PARTY_LICENSES')
if (src / 'assets/tconstruct').exists():
    p.error('source already contains a tconstruct asset namespace')
out.mkdir(parents=True)
for path in src.iterdir():
    if path.name not in {'items.json', 'registries.json', 'command-registries.json', 'assets'}:
        (out / path.name).symlink_to(path, target_is_directory=path.is_dir())
# Keep the source notices in the generated resource distribution as well.
licenses = out / 'THIRD_PARTY_LICENSES'
licenses.mkdir()
repository = Path(__file__).resolve().parents[1]
for name in ('tconstruct', 'mantle', 'forge'):
    shutil.copyfile(repository / 'third_party' / name / 'LICENSE', licenses / (name + '.txt'))
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
# Original Part Builder pattern sprites: staged in the GUI atlas namespace.
pattern_source = Path(__file__).resolve().parents[1] / 'assets/tconstruct/gui/patterns'
pattern_dest = gui / 'tconstruct_patterns'
pattern_dest.mkdir()
for path in sorted(pattern_source.glob('*.png')):
    (pattern_dest / path.name).write_bytes(path.read_bytes())
mod_assets = assets / 'tconstruct'
mod_assets.mkdir()
shutil.copytree(repository / 'assets/tconstruct/textures', mod_assets / 'textures')
(mod_assets / 'items').mkdir()
(mod_assets / 'models/item').mkdir(parents=True)
# Keep the pinned upstream material images and render metadata as a separate
# source layer. The original Forge loader is ported by the Mixins, rather than
# making these original models look like vanilla models without their behavior.
source_manifest = {}
for row in (repository / 'docs/upstream-part-visuals.sha256').read_text().splitlines():
    digest, name = row.split('  ', 1)
    if name in source_manifest:
        raise ValueError('duplicate part visual manifest path')
    source_manifest[name] = digest
forge_model = json.loads((repository / 'content/reference/forge-default-item-model.json').read_text())
with zipfile.ZipFile(repository / 'content/reference/tconstruct-part-visuals.zip') as archive:
    if set(archive.namelist()) != set(source_manifest) or len(archive.namelist()) != len(source_manifest):
        raise ValueError('part visual archive does not match manifest')
    for name in archive.namelist():
        parts = Path(name).parts
        if not name.startswith('assets/tconstruct/') or any(part in ('', '.', '..') for part in parts):
            raise ValueError('invalid part visual path')
        payload = archive.read(name)
        if hashlib.sha256(payload).hexdigest() != source_manifest[name]:
            raise ValueError('part visual hash mismatch')
        relative = name.removeprefix('assets/tconstruct/')
        if relative.startswith(('textures/', 'tinkering/materials/', 'lang/')):
            destination = mod_assets / relative
        elif relative.startswith('models/item/') and relative.endswith('.json'):
            destination = mod_assets / 'tinkering/source_part_models' / Path(relative).name
        else:
            raise ValueError('unexpected part visual path')
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists():
            raise ValueError(f'part visual output exists: {destination}')
        destination.write_bytes(payload)
        if relative.startswith('textures/') and relative.endswith('.png'):
            sprite = relative.removeprefix('textures/').removesuffix('.png')
            alias = 'tconstruct_part/' + sprite
            model = mod_assets / 'models/item' / (alias + '.json')
            item = mod_assets / 'items' / (alias + '.json')
            model.parent.mkdir(parents=True, exist_ok=True)
            item.parent.mkdir(parents=True, exist_ok=True)
            model.write_text(json.dumps({'parent': 'minecraft:builtin/generated', 'gui_light': forge_model['gui_light'], 'display': forge_model['display'], 'textures': {'layer0': 'tconstruct:' + sprite, 'particle': 'tconstruct:' + sprite}}))
            item.write_text(json.dumps({'model': {'type': 'minecraft:model', 'model': 'tconstruct:item/' + alias}}))
items = json.loads((src / 'items.json').read_text())
entries = [
    ('tconstruct:pattern', 'minecraft:paper', 'Pattern'),
    ('tconstruct:flint_pick_head', 'minecraft:flint', 'Flint Pick Head'),
    ('tconstruct:wood_binding', 'minecraft:flint', 'Wooden Tool Binding'),
    ('tconstruct:wood_handle', 'minecraft:flint', 'Wooden Tool Handle'),
    ('tconstruct:flint_pickaxe', 'minecraft:stone_pickaxe', 'Flint Pickaxe'),
    ('tconstruct:pick_head', 'minecraft:flint', 'Pick Head'),
    ('tconstruct:tool_handle', 'minecraft:flint', 'Tool Handle'),
    ('tconstruct:tool_binding', 'minecraft:flint', 'Tool Binding'),
    ('tconstruct:pickaxe', 'minecraft:stone_pickaxe', 'Pickaxe'),
]
# Original material-bearing part and fake-ingot IDs share MaterialItem behavior.
# Their rendering and tooltip name paths are connected; stats, traits and
# recipe-driven crafting remain incomplete, as documented.
source_names = json.loads((repository / 'content/assets/tconstruct/lang/en_us.json').read_text())
existing = {name for name, _, _ in entries}
for short in MATERIAL_MODELS:
    name = 'tconstruct:' + short
    if name not in existing:
        entries.append((name, 'minecraft:flint', source_names['item.tconstruct.' + short]))
for name, template, title in entries:
    item = copy.deepcopy(items[template])
    item['minecraft:item_name'] = {'text': title}
    short = name.split(':')[1]
    item['minecraft:item_model'] = name
    (mod_assets / 'items' / (short + '.json')).write_text(json.dumps({'model': {'type':'minecraft:model','model':'tconstruct:item/' + short}}))
    if short in MATERIAL_MODELS:
        original_model = json.loads((mod_assets / 'tinkering/source_part_models' / (short + '.json')).read_text())
        texture = original_model['textures']['texture']
        native_model = {'parent': 'minecraft:builtin/generated', 'gui_light': forge_model['gui_light'], 'display': forge_model['display'], 'textures': {'layer0': texture, 'particle': texture}}
    else:
        layers = ['pickaxe_handle', 'pickaxe_head', 'pickaxe_binding'] if short in ('flint_pickaxe', 'pickaxe') else [short]
        native_model = {'parent': 'minecraft:item/handheld' if short in ('flint_pickaxe', 'pickaxe') else 'minecraft:item/generated', 'textures': {f'layer{i}': 'tconstruct:item/' + layer for i, layer in enumerate(layers)}}
    (mod_assets / 'models/item' / (short + '.json')).write_text(json.dumps(native_model))
    if name.endswith('pickaxe'):
        item['minecraft:max_damage'] = 85
        item['minecraft:tool']['rules'][-1]['speed'] = 3.5
    items[name] = item
# Material-specific visual models for shared original item IDs.
for short in ('wood_pick_head', 'flint_handle', 'flint_binding'):
    (mod_assets / 'items' / (short + '.json')).write_text(json.dumps({'model': {'type': 'minecraft:model', 'model': 'tconstruct:item/' + short}}))
    (mod_assets / 'models/item' / (short + '.json')).write_text(json.dumps({'parent': 'minecraft:item/generated', 'textures': {'layer0': 'tconstruct:item/' + short}}))
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
