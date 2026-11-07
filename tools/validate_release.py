#!/usr/bin/env python3
"""Validate a product's release files against this checkout and its tag (`pluto-lib-vX.Y.Z`, `tconstruct-vX.Y.Z`)."""
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


TAG = re.compile(r'(?P<product>pluto-lib|tconstruct)-v(?P<version>\d+\.\d+\.\d+(?:-rc\.\d+)?)')


def validate(tag, directory):
    """Check the files built for `<product>-v<semver>` against this checkout. Returns the product name."""
    m = TAG.fullmatch(tag)
    if not m:
        raise ValueError('release tag must be <product>-vMAJOR.MINOR.PATCH (pluto-lib or tconstruct, optional -rc.N)')
    if m['product'] == 'pluto-lib':
        validate_pluto_lib(m['version'], directory)
        return 'pluto-lib'
    version = m['version']
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


def validate_pluto_lib(version, directory):
    compat = json.loads((ROOT / 'tconstruct/compatibility.json').read_text())
    source = tomllib.loads((ROOT / 'pluto-lib/mod/horizon-mixin.toml').read_text())
    files = inspect(directory / 'pluto-lib.hnmod')
    manifest = json.loads(files['horizon-native.json'])
    if manifest['id'] != 'pluto_lib' or source['mod']['id'] != 'pluto_lib':
        raise ValueError('unexpected package identity')
    if manifest['version'] != version or source['mod']['version'] != version:
        raise ValueError('PlutoLib version does not match the release tag')
    if manifest['horizon'] != compat['horizon_revision'] or manifest['mixins'] != compat['mixins_revision']:
        raise ValueError('PlutoLib compatibility pins differ from compatibility.json')
    bundled = tomllib.loads(files['mixins/pluto_lib/horizon-mixin.toml'].decode())
    if bundled['mod'] != source['mod'] or bundled['compatibility']['horizon_revision'] != compat['horizon_revision']:
        raise ValueError('bundled manifest differs from the checkout')
    print(f'Validated pluto-lib-v{version}: Horizon {manifest["horizon"][:8]}, Mixins {manifest["mixins"][:8]}')


def files_version(files):
    return json.loads(files['horizon-native.json'])['version']


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tag')
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    validate(args.tag, args.directory)
