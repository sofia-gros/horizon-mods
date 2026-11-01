#!/usr/bin/env python3
"""Package tracked content and original source layers as a Horizon content mod.

This deliberately never reads the player's prepared Minecraft resource root.
"""
import argparse
from pathlib import Path, PurePosixPath
import zipfile

from package_hnmod import ROOT, MOD, tracked, write_zip_file

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
