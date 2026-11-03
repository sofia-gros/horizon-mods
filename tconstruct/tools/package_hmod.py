#!/usr/bin/env python3
"""Package tracked content and original source layers as a Horizon content mod.

This deliberately never reads the player's prepared Minecraft resource root.
"""
import argparse
import ast
import json
from pathlib import Path, PurePosixPath
import zipfile

from package_hnmod import MOD, tracked, write_zip_file
from import_part_visuals import MATERIAL_MODELS
from creative_materials import build_variants
from creative_blocks import add_block_items

MANIFEST = b'''id = "tconstruct"
name = "Tinkers' Construct content"
version = "0.1.0"
api = "0.2"
side = "both"
capabilities = ["registries"]
namespaces = ["mantle", "forge"]
'''


def package(output):
    files = {'horizon-mod.toml': MANIFEST}

    def add(name, data):
        path = PurePosixPath(name)
        if path.is_absolute() or any(part in ('', '.', '..') for part in path.parts):
            raise ValueError(f'invalid content path: {name}')
        if name in files and files[name] != data:
            raise ValueError(f'conflicting content path: {name}')
        files[name] = data

    for rel, source in tracked('tconstruct/content/assets', 'tconstruct/content/data'):
        add(rel.relative_to('tconstruct/content').as_posix(), source.read_bytes())

    # GUI and fluid atlas sprites must be exposed under vanilla atlas keys at
    # startup. Store our own source pixels in our namespace until then.
    runtime = 'assets/tconstruct/runtime/'
    add(runtime + 'forge-harvest-tags.zip', (MOD / 'content/reference/forge-47.4.0-harvest-tags.zip').read_bytes())
    add(runtime + 'tconstruct-melter-fluids.json', (MOD / 'content/reference/tconstruct-melter-fluids.json').read_bytes())
    for name in ('part_builder', 'tinker_station', 'worktable', 'icons', 'generic', 'panel', 'blank_extra_row'):
        add(runtime + 'gui/tconstruct_' + name + '.png', (MOD / 'assets/tconstruct/gui' / (name + '.png')).read_bytes())
    icon_sources = json.loads((MOD / 'content/assets/tconstruct/runtime/modifier-icon-sources.json').read_text())
    for name in icon_sources['mod']:
        add(runtime + 'gui/tconstruct_modifier_' + name + '.png', (MOD / 'assets/tconstruct/gui/modifiers' / (name + '.png')).read_bytes())
    for name, source in icon_sources['mod_item'].items():
        add(runtime + 'gui/tconstruct_modifier_' + name + '.png', (MOD / 'assets/tconstruct/textures' / (source + '.png')).read_bytes())
    for rel, source in tracked('tconstruct/assets/tconstruct/gui/patterns'):
        add(runtime + 'gui/tconstruct_patterns/' + rel.name, source.read_bytes())
    with zipfile.ZipFile(MOD / 'content/reference/tconstruct-plate-armor-textures.zip') as archive:
        for item in archive.infolist():
            if item.is_dir():
                continue
            if '/' in item.filename or not item.filename.endswith('.png'):
                raise ValueError('invalid plate sprite')
            add(runtime + 'gui/tconstruct_plate_' + item.filename, archive.read(item))
    with zipfile.ZipFile(MOD / 'content/reference/tconstruct-smeltery-assets.zip') as archive:
        add(runtime + 'gui/tconstruct_heating_structure.png', archive.read('assets/tconstruct/textures/gui/heating_structure.png'))
    with zipfile.ZipFile(MOD / 'content/reference/tconstruct-melter-assets.zip') as archive:
        add(runtime + 'gui/tconstruct_melter.png', archive.read('assets/tconstruct/textures/gui/melter.png'))
        add(runtime + 'block/tconstruct_fluid_fallback.png', archive.read('assets/tconstruct/textures/fluid/molten/still.png'))
        add(runtime + 'block/tconstruct_fluid_fallback.png.mcmeta', archive.read('assets/tconstruct/textures/fluid/molten/still.png.mcmeta'))
        animations = json.loads((MOD / 'content/reference/tconstruct-melter-fluid-animation.json').read_text())
        fluids = json.loads(archive.read('assets/tconstruct/melter-fluid-textures.json'))
        for fluid, texture in fluids.items():
            add(runtime + 'block/tconstruct_fluid_' + fluid.removeprefix('tconstruct:') + '.png', archive.read('assets/tconstruct/' + texture))
            if fluid in animations:
                add(runtime + 'block/tconstruct_fluid_' + fluid.removeprefix('tconstruct:') + '.png.mcmeta', json.dumps(animations[fluid]).encode())

    # The source visual archives are owned by the mod and do not contain the
    # player's Minecraft textures. Retain Forge models in the paths read by
    # our renderer instead of asking the vanilla model loader to interpret them.
    for rel, source in tracked('tconstruct/assets/tconstruct/textures'):
        name = rel.relative_to('tconstruct').as_posix()
        if name not in files:
            add(name, source.read_bytes())
    sprite_aliases = {'part': set(), 'tool': set()}
    for archive_name, model_dir, kind in (
        ('tconstruct-part-visuals.zip', 'source_part_models', 'part'),
        ('tconstruct-tool-visuals.zip', 'source_tool_models', 'tool'),
    ):
        with zipfile.ZipFile(MOD / 'content/reference' / archive_name) as archive:
            for info in archive.infolist():
                if info.is_dir() or not info.filename.startswith('assets/tconstruct/'):
                    continue
                relative = info.filename.removeprefix('assets/tconstruct/')
                if relative.startswith('textures/') and relative.endswith('.png'):
                    sprite_aliases[kind].add(relative.removeprefix('textures/').removesuffix('.png'))
                if relative.startswith('models/item/') and relative.endswith('.json'):
                    relative = 'tinkering/' + model_dir + '/' + PurePosixPath(relative).name
                elif not relative.startswith(('textures/', 'tinkering/materials/')):
                    continue
                name = 'assets/tconstruct/' + relative
                if name not in files:
                    add(name, archive.read(info))

    # Derive declarations from the same source lists as the desktop overlay.
    # Horizon resolves each vanilla template on the player's own installation.
    source = ast.parse((MOD / 'tools/resources.py').read_text(encoding='utf-8'))
    entries = next(ast.literal_eval(node.value) for node in source.body
                   if isinstance(node, ast.Assign) and any(
                       isinstance(target, ast.Name) and target.id == 'entries'
                       for target in node.targets))
    templates = {name.split(':', 1)[1]: template for name, template, _ in entries}
    with zipfile.ZipFile(MOD / 'content/reference/tconstruct-tool-definitions.zip') as archive:
        equipment = {name.rsplit('/', 1)[-1].removesuffix('.json')
                     for name in archive.namelist()
                     if '/tool_definitions/' in name and name.endswith('.json')}
    equipment.discard('minotaur_axe')
    if 'slime_chestplate' in equipment:
        equipment.remove('slime_chestplate')
        equipment.add('slimy_chestplate')
    equipment.add('crystalshot')
    special = {'hand_axe': 'minecraft:stone_axe', 'mattock': 'minecraft:stone_axe',
               'kama': 'minecraft:stone_hoe', 'pickadze': 'minecraft:stone_pickaxe'}
    for short in equipment:
        templates.setdefault(short, special.get(short, 'minecraft:flint'))
    for short in MATERIAL_MODELS:
        templates.setdefault(short, 'minecraft:flint')
    with zipfile.ZipFile(MOD / 'content/reference/tconstruct-fluid-buckets.zip') as archive:
        for short in json.loads(archive.read('manifest.json'))['buckets']:
            templates[short] = 'minecraft:water_bucket'
            add('assets/tconstruct/textures/item/fluid_bucket/' + short + '.png',
                archive.read('textures/' + short + '.png'))
    for kind in ('gold', 'sand', 'red_sand'):
        tag = MOD / 'content/reference/tconstruct-tags/data/tconstruct/tags/items/casts' / (kind + '.json')
        for item in json.loads(tag.read_text())['values']:
            templates[item.split(':', 1)[1]] = 'minecraft:flint'
    for part in ('helmet', 'chestplate', 'leggings', 'boots'):
        templates[part + '_plating_dummy'] = 'minecraft:flint'
    for short, template in sorted(templates.items()):
        declaration = 'data/tconstruct/horizon/item/' + short + '.json'
        block = 'data/tconstruct/horizon/block/' + short + '.json'
        if declaration in files or block in files:
            continue
        data = {'template': template}
        if short in equipment:
            data['max_stack_size'] = (64 if short in ('arrow', 'crystalshot') else
                                      16 if short in ('shuriken', 'throwing_axe') else
                                      2 if short == 'dagger' else 1)
        add(declaration, json.dumps(data, separators=(',', ':')).encode())

    tabs = json.loads((MOD / 'content/creative-tabs.json').read_text())
    tabs = build_variants(tabs, MOD / 'content/reference/tconstruct-tool-definitions.zip', MOD / 'content/reference/tconstruct-materials.zip')
    tabs = add_block_items(tabs, MOD / 'content/data/tconstruct/horizon/block')
    add(runtime + 'creative-tabs.json', json.dumps(tabs).encode())
    add(runtime + 'equipment.json', json.dumps(sorted(equipment)).encode())

    # Make the source sprites addressable by the same vanilla model aliases
    # used by the material and tool renderers in the desktop overlay.
    forge_model = json.loads((MOD / 'content/reference/forge-default-item-model.json').read_text())
    for kind, sprites in sprite_aliases.items():
        for sprite in sorted(sprites):
            alias = 'tconstruct_' + kind + '/' + sprite
            model_path = 'assets/tconstruct/models/item/' + alias + '.json'
            item_path = 'assets/tconstruct/items/' + alias + '.json'
            if model_path not in files:
                add(model_path, json.dumps({'parent': 'minecraft:builtin/generated',
                    'gui_light': forge_model['gui_light'], 'display': forge_model['display'],
                    'textures': {'layer0': 'tconstruct:' + sprite, 'particle': 'tconstruct:' + sprite}}).encode())
            if item_path not in files:
                add(item_path, json.dumps({'model': {'type': 'minecraft:model',
                    'model': 'tconstruct:item/' + alias}}).encode())
    for short, template in sorted(templates.items()):
        item_path = 'assets/tconstruct/items/' + short + '.json'
        model_path = 'assets/tconstruct/models/item/' + short + '.json'
        if item_path not in files:
            add(item_path, json.dumps({'model': {'type': 'minecraft:model',
                'model': 'tconstruct:item/' + short}}).encode())
        if model_path not in files:
            source_model = 'assets/tconstruct/tinkering/source_part_models/' + short + '.json'
            if source_model in files:
                texture = json.loads(files[source_model])['textures']['texture']
            elif 'assets/tconstruct/textures/item/materials/' + short + '.png' in files:
                texture = 'tconstruct:item/materials/' + short
            elif 'assets/tconstruct/textures/item/' + short + '.png' in files:
                texture = 'tconstruct:item/' + short
            else:
                texture = template.replace('minecraft:', 'minecraft:item/')
            add(model_path, json.dumps({'parent': 'minecraft:item/generated',
                'textures': {'layer0': texture}}).encode())

    # These are original layers, not Horizon's merged tag tables. Keeping them
    # under assets prevents the content loader from flattening their semantics.
    prefix = 'assets/tconstruct/source-packs/'
    tags = 'tconstruct/content/reference/tconstruct-tags/data'
    for rel, source in tracked(tags):
        add(prefix + 'tconstruct-original-pack/data/' + rel.relative_to(tags).as_posix(), source.read_bytes())
    archives = {
        'tconstruct-recipes.zip': 'tconstruct-original-pack',
        'tconstruct-materials.zip': 'tconstruct-original-pack',
        'tconstruct-tool-definitions.zip': 'tconstruct-original-pack',
        'tconstruct-station-layouts.zip': 'tconstruct-original-pack',
        'tconstruct-fluid-tags.zip': 'tconstruct-original-pack',
        'tconstruct-fluid-effects.zip': 'tconstruct-original-pack',
        'tconstruct-block-tags.zip': 'tconstruct-original-pack',
        'forge-47.4.0-data.zip': 'forge-original-pack',
    }
    for filename, layer in archives.items():
        with zipfile.ZipFile(MOD / 'content/reference' / filename) as archive:
            for item in archive.infolist():
                if item.is_dir():
                    continue
                path = PurePosixPath(item.filename)
                if not item.filename.startswith('data/') or path.suffix != '.json':
                    raise ValueError(f'invalid original data file: {item.filename}')
                add(prefix + layer + '/' + item.filename, archive.read(item))
    for name in ('tconstruct', 'mantle', 'forge'):
        add(f'assets/tconstruct/licenses/{name}.txt', (MOD / 'third_party' / name / 'LICENSE').read_bytes())
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'w') as archive:
        for name, data in sorted(files.items()):
            write_zip_file(archive, name, data)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, required=True)
    package(parser.parse_args().output)
