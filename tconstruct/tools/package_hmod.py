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

MANIFEST = b'''id = "tconstruct"
name = "Tinkers' Construct content"
version = "0.1.0"
api = "0.2"
side = "both"
capabilities = ["registries"]
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

    # The source visual archives are owned by the mod and do not contain the
    # player's Minecraft textures. Retain Forge models in the paths read by
    # our renderer instead of asking the vanilla model loader to interpret them.
    for rel, source in tracked('tconstruct/assets/tconstruct/textures'):
        name = rel.relative_to('tconstruct').as_posix()
        if name not in files:
            add(name, source.read_bytes())
    for archive_name, model_dir in (
        ('tconstruct-part-visuals.zip', 'source_part_models'),
        ('tconstruct-tool-visuals.zip', 'source_tool_models'),
    ):
        with zipfile.ZipFile(MOD / 'content/reference' / archive_name) as archive:
            for info in archive.infolist():
                if info.is_dir() or not info.filename.startswith('assets/tconstruct/'):
                    continue
                relative = info.filename.removeprefix('assets/tconstruct/')
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
