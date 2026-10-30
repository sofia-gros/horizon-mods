#!/usr/bin/env python3
"""Create a separate, disposable resource overlay for the prototype."""
import argparse, copy, hashlib, json, shutil, zipfile
from pathlib import Path
from import_part_visuals import MATERIAL_MODELS

def link_or_copy(source, destination):
    """Preserve the source; Windows may deny symlink creation without developer mode."""
    try:
        destination.symlink_to(source, target_is_directory=source.is_dir())
    except OSError:
        if destination.exists() or destination.is_symlink():
            raise
        if source.is_dir():
            shutil.copytree(source, destination)
        else:
            shutil.copy2(source, destination)


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
        link_or_copy(path, out / path.name)
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
        link_or_copy(namespace, assets / namespace.name)
vanilla = assets / 'minecraft'
vanilla.mkdir()
for child in (src / 'assets/minecraft').iterdir():
    if child.name != 'textures':
        link_or_copy(child, vanilla / child.name)
textures = vanilla / 'textures'
textures.mkdir()
for child in (src / 'assets/minecraft/textures').iterdir():
    if child.name != 'gui':
        link_or_copy(child, textures / child.name)
gui = textures / 'gui'
gui.mkdir()
for child in (src / 'assets/minecraft/textures/gui').iterdir():
    link_or_copy(child, gui / child.name)
for name in ('part_builder', 'tinker_station', 'icons'):
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
shutil.copytree(repository / 'content/assets/tconstruct/sounds', mod_assets / 'sounds')
shutil.copyfile(repository / 'content/assets/tconstruct/sounds.json', mod_assets / 'sounds.json')
# Original dummy plating sprites are ordinary generated item textures.
for part in ('helmet', 'chestplate', 'leggings', 'boots'):
    relative = Path('item/tool/parts') / ('plating_' + part + '.png')
    destination = mod_assets / 'textures' / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(repository / 'content/assets/tconstruct/textures' / relative, destination)
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
    ('tconstruct:venom_bottle', 'minecraft:honey_bottle', 'Venom Bottle'),
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
# Register three-part tools that share the implemented material/stat path.
# Additional actions and modifiers remain separate gameplay work.
tool_templates = {'pickaxe_dig': 'minecraft:stone_pickaxe', 'axe_dig': 'minecraft:stone_axe',
                  'shovel_dig': 'minecraft:stone_shovel', 'hoe_dig': 'minecraft:stone_hoe'}
with zipfile.ZipFile(repository / 'content/reference/tconstruct-recipes.zip') as archive:
    tool_results = set()
    for name in archive.namelist():
        if not name.endswith('.json'): continue
        recipe = json.loads(archive.read(name))
        if recipe.get('type') == 'tconstruct:tool_building' and isinstance(recipe.get('result'), str):
            tool_results.add(recipe['result'])
with zipfile.ZipFile(repository / 'content/reference/tconstruct-tool-definitions.zip') as archive:
    for name in sorted(archive.namelist()):
        if '/tool_definitions/' not in name or not name.endswith('.json'): continue
        short = name.rsplit('/', 1)[-1].removesuffix('.json')
        item_id = 'tconstruct:' + short
        if item_id in existing or item_id not in tool_results: continue
        modules = json.loads(archive.read(name)).get('modules', [])
        part_modules = [module for module in modules if module.get('type') == 'tconstruct:part_stats']
        actions = next((module.get('tool_actions', []) for module in modules if module.get('type') == 'tconstruct:tool_actions'), [])
        if len(part_modules) != 1 or len(part_modules[0].get('parts', [])) != 3: continue
        parts = [part if isinstance(part, str) else part.get('item', '') for part in part_modules[0]['parts']]
        if not all(part.startswith('tconstruct:') and part.split(':', 1)[1] in MATERIAL_MODELS for part in parts): continue
        template = next((tool_templates[action] for action in actions if action in tool_templates), None)
        if template is None: continue
        entries.append((item_id, template, source_names['item.tconstruct.' + short]))
        existing.add(item_id)
for short in MATERIAL_MODELS:
    name = 'tconstruct:' + short
    if name not in existing:
        entries.append((name, 'minecraft:flint', source_names['item.tconstruct.' + short]))
# Register original gold, sand and red-sand casts from their source tags.
casts = []
for kind in ('gold', 'sand', 'red_sand'):
    casts.extend(json.loads((repository / 'content/reference/tconstruct-tags/data/tconstruct/tags/items/casts' / (kind + '.json')).read_text())['values'])
for name in casts:
    short = name.split(':')[1]
    entries.append((name, 'minecraft:flint', source_names['item.tconstruct.' + short]))
    model = json.loads((repository / 'content/assets/tconstruct/models/item' / (short + '.json')).read_text())
    texture = model['textures']['layer0'].split(':', 1)[1] + '.png'
    dest = mod_assets / 'textures' / texture
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(repository / 'content/assets/tconstruct/textures' / texture, dest)
# Original stone plating items are regular DummyMaterialItem instances.
for part in ('helmet', 'chestplate', 'leggings', 'boots'):
    short = part + '_plating_dummy'
    entries.append(('tconstruct:' + short, 'minecraft:flint', source_names['item.tconstruct.' + short]))
for name, template, title in entries:
    item = copy.deepcopy(items[template])
    item['minecraft:item_name'] = {'text': title}
    short = name.split(':')[1]
    item['minecraft:item_model'] = name
    (mod_assets / 'items' / (short + '.json')).write_text(json.dumps({'model': {'type':'minecraft:model','model':'tconstruct:item/' + short}}))
    if short == 'venom_bottle':
        native_model = {'parent': 'minecraft:item/generated', 'textures': {'layer0': 'tconstruct:item/bottle/venom_contents', 'layer1': 'minecraft:item/potion'}}
    elif name in casts or short.endswith('_plating_dummy'):
        native_model = json.loads((repository / 'content/assets/tconstruct/models/item' / (short + '.json')).read_text())
    elif short in MATERIAL_MODELS:
        original_model = json.loads((mod_assets / 'tinkering/source_part_models' / (short + '.json')).read_text())
        texture = original_model['textures']['texture']
        native_model = {'parent': 'minecraft:builtin/generated', 'gui_light': forge_model['gui_light'], 'display': forge_model['display'], 'textures': {'layer0': texture, 'particle': texture}}
    elif template in tool_templates.values() and short not in ('flint_pickaxe', 'pickaxe'):
        native_model = {'parent': 'minecraft:item/handheld', 'textures': {'layer0': 'minecraft:item/' + template.split(':', 1)[1]}}
    else:
        layers = ['pickaxe_handle', 'pickaxe_head', 'pickaxe_binding'] if short in ('flint_pickaxe', 'pickaxe') else [short]
        native_model = {'parent': 'minecraft:item/handheld' if short in ('flint_pickaxe', 'pickaxe') else 'minecraft:item/generated', 'textures': {f'layer{i}': 'tconstruct:item/' + layer for i, layer in enumerate(layers)}}
    (mod_assets / 'models/item' / (short + '.json')).write_text(json.dumps(native_model))
    if short == 'venom_bottle':
        # TinkerFood.VENOM_BOTTLE and ContainerFoodItem (1.20.1): no
        # nutrition, always drinkable, 32 ticks, strength and poison.
        # Its Forge fluid capability and detailed tooltip are not yet ported.
        item['minecraft:food'] = {'can_always_eat': True, 'nutrition': 0, 'saturation': 0.0}
        item['minecraft:consumable'] = {'animation': 'drink', 'consume_seconds': 1.6,
            'has_consume_particles': False, 'sound': 'minecraft:entity.generic.drink',
            'on_consume_effects': [{'type': 'minecraft:apply_effects', 'effects': [
                {'id': 'minecraft:strength', 'duration': 600},
                {'id': 'minecraft:poison', 'duration': 250},
            ]}]}
        item['minecraft:max_stack_size'] = 16
        item['minecraft:use_remainder'] = {'id': 'minecraft:glass_bottle'}
    if name.endswith('pickaxe'):
        item['minecraft:max_damage'] = 85
        item['minecraft:tool']['rules'][-1]['speed'] = 3.5
    items[name] = item
# Prepare source tool models and material sprites for the registered tools.
# The shared renderer can add other tools as their real item registrations land.
tool_manifest = {}
for row in (repository / 'docs/upstream-tool-visuals.sha256').read_text().splitlines():
    digest, name = row.split('  ', 1)
    tool_manifest[name] = digest
with zipfile.ZipFile(repository / 'content/reference/tconstruct-tool-visuals.zip') as archive:
    if set(archive.namelist()) != set(tool_manifest) or len(archive.namelist()) != len(tool_manifest):
        raise ValueError('tool visual archive does not match manifest')
    models = {}
    prefixes = set()
    for name in archive.namelist():
        prefix = 'assets/tconstruct/models/item/'
        if name.startswith(prefix) and '/' not in name.removeprefix(prefix) and name.endswith('.json'):
            short = name.removeprefix(prefix).removesuffix('.json')
            if 'tconstruct:' + short in items:
                payload = archive.read(name)
                if hashlib.sha256(payload).hexdigest() != tool_manifest[name]:
                    raise ValueError('tool model hash mismatch')
                model = json.loads(payload)
                models[name] = payload
                for texture in model.get('textures', {}).values():
                    if isinstance(texture, str) and texture.startswith('tconstruct:item/tool/'):
                        prefixes.add('assets/tconstruct/textures/' + texture.removeprefix('tconstruct:'))
    for name, payload in models.items():
        destination = mod_assets / 'tinkering/source_tool_models' / Path(name).name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(payload)
    for name in archive.namelist():
        if not name.endswith('.png') or not any(name == prefix + '.png' or name.startswith(prefix + '_') for prefix in prefixes):
            continue
        payload = archive.read(name)
        if hashlib.sha256(payload).hexdigest() != tool_manifest[name]:
            raise ValueError('tool sprite hash mismatch')
        relative = name.removeprefix('assets/tconstruct/')
        destination = mod_assets / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists() and destination.read_bytes() != payload:
            raise ValueError(f'conflicting tool sprite: {destination}')
        if not destination.exists():
            destination.write_bytes(payload)
        sprite = relative.removeprefix('textures/').removesuffix('.png')
        alias = 'tconstruct_tool/' + sprite
        model = mod_assets / 'models/item' / (alias + '.json')
        item = mod_assets / 'items' / (alias + '.json')
        model.parent.mkdir(parents=True, exist_ok=True)
        item.parent.mkdir(parents=True, exist_ok=True)
        model.write_text(json.dumps({'parent': 'minecraft:builtin/generated', 'gui_light': forge_model['gui_light'], 'display': forge_model['display'], 'textures': {'layer0': 'tconstruct:' + sprite, 'particle': 'tconstruct:' + sprite}}))
        item.write_text(json.dumps({'model': {'type': 'minecraft:model', 'model': 'tconstruct:item/' + alias}}))
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
