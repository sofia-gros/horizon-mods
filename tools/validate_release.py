#!/usr/bin/env python3
"""Validate the three release archives against this checkout and release tag."""
import argparse
import json
from pathlib import Path, PurePosixPath
import re
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def inspect(path):
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        if not names or len(names) != len(set(names)) or archive.testzip() is not None:
            raise ValueError(f'{path}: empty, duplicate, or corrupt archive')
        for name in names:
            p = PurePosixPath(name)
            if p.is_absolute() or '..' in p.parts or '\\' in name:
                raise ValueError(f'{path}: unsafe entry {name}')
        return {name: archive.read(name) for name in names}


def validate(tag, directory):
    if not re.fullmatch(r'v\d+\.\d+\.\d+', tag):
        raise ValueError('release tag must be vMAJOR.MINOR.PATCH')
    version = tag[1:]
    compat = json.loads((ROOT / 'tconstruct/compatibility.json').read_text())
    pluto_source = json.loads((ROOT / 'pluto-lib/horizon-native.json').read_text())
    content = inspect(directory / 'tconstruct.hmod')
    native = inspect(directory / 'tconstruct.hnmod')
    pluto = inspect(directory / 'plutolib.hnmod')
    cm = tomllib.loads(content['horizon-mod.toml'].decode())
    nm = json.loads(native['horizon-native.json'])
    pm = json.loads(pluto['horizon-native.json'])
    source = tomllib.loads((ROOT / 'tconstruct/mixins/tconstruct/horizon-mixin.toml').read_text())
    if cm['id'] != 'tconstruct' or nm['id'] != 'tconstruct_native' or pm['id'] != 'pluto_native_creative':
        raise ValueError('unexpected package identity')
    if cm['version'] != version or nm['version'] != version or source['mod']['version'] != version:
        raise ValueError('Tinkers versions do not match the release tag')
    if pm != pluto_source:
        raise ValueError('PlutoLib packaged metadata differs from the checkout')
    for manifest in (nm, pm):
        if manifest['horizon'] != compat['horizon_revision'] or manifest['mixins'] != compat['mixins_revision']:
            raise ValueError('native package compatibility pins differ')
    if source['compatibility']['horizon_revision'] != compat['horizon_revision']:
        raise ValueError('Tinkers source pin differs')
    if not any(n.startswith('assets/tconstruct/') for n in content) or not any(n.startswith('data/tconstruct/') for n in content):
        raise ValueError('Tinkers content is missing')
    for files, ident in ((native, nm['id']), (pluto, pm['id'])):
        key = f'mixins/{ident}/horizon-mixin.toml'
        mixin = tomllib.loads(files[key].decode())
        if mixin['mod']['id'] != ident or mixin['mod']['version'] != files_version(files):
            raise ValueError(f'{ident}: native manifest differs from bundled mixin')
        if mixin['compatibility']['horizon_revision'] != compat['horizon_revision']:
            raise ValueError(f'{ident}: bundled mixin pin differs')
    print(f'Validated {tag}: Tinkers {version}, PlutoLib {pm["version"]}, matching Horizon/Mixins pins')


def files_version(files):
    return json.loads(files['horizon-native.json'])['version']


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tag')
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    validate(args.tag, args.directory)
