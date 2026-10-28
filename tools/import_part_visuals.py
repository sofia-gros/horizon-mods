#!/usr/bin/env python3
"""Archive pinned TConstruct part models, source sprites and render metadata."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import zipfile

REV = 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2'
PARTS = ('pick_head hammer_head small_axe_head broad_axe_head small_blade broad_blade '
         'adze_head large_plate bow_limb bow_grip bowstring arrow_head arrow_shaft '
         'fletching tool_binding tough_binding tool_handle tough_handle helmet_plating '
         'chestplate_plating leggings_plating boots_plating maille shield_core ribcage '
         'shell laces repair_kit').split()


def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args])


def build(repo: Path, output: Path, manifest: Path):
    repo = repo.resolve(strict=True)
    if output.exists() or output.is_symlink() or manifest.exists() or manifest.is_symlink():
        raise ValueError('output and manifest must not already exist')
    names = git(repo, 'ls-tree', '-r', '--name-only', REV).decode().splitlines()
    model_paths, roots = [], []
    for item in PARTS:
        suffix = f'/assets/tconstruct/models/item/{item}.json'
        matches = [path for path in names if path.startswith(('src/main/resources/', 'src/generated/resources/')) and path.endswith(suffix)]
        if len(matches) != 1:
            raise ValueError(f'expected one source model for {item}, got {matches}')
        model_paths.append(matches[0])
        model = json.loads(git(repo, 'show', f'{REV}:{matches[0]}'))
        if model.get('loader') != 'tconstruct:material':
            raise ValueError(f'unexpected source model loader for {item}')
        texture = model['textures']['texture']
        if not texture.startswith('tconstruct:item/tool/'):
            raise ValueError(f'unexpected texture namespace for {item}')
        roots.append('assets/tconstruct/textures/' + texture.split(':', 1)[1])
    textures, info = [], []
    for name in names:
        if not name.startswith(('src/main/resources/', 'src/generated/resources/')):
            continue
        path = name.split('/resources/', 1)[1]
        if any(path in (root + '.png', root + '.png.mcmeta') or path.startswith(root + '_') for root in roots):
            textures.append(name)
        elif path.startswith('assets/tconstruct/tinkering/materials/') and path.endswith('.json'):
            info.append(name)
    paths = sorted(model_paths + textures + info)
    entries = [path.split('/resources/', 1)[1] for path in paths]
    if len(entries) != len(set(entries)) or not textures or not info:
        raise ValueError('duplicate or missing part visual resources')
    data = git(repo, 'archive', REV, *paths)
    rows = []
    with tarfile.open(fileobj=io.BytesIO(data)) as tar, zipfile.ZipFile(output, 'x', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as out:
        for source, name in zip(paths, entries):
            contents = tar.extractfile(source).read()
            entry = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            entry.compress_type = zipfile.ZIP_DEFLATED
            out.writestr(entry, contents)
            rows.append(f'{hashlib.sha256(contents).hexdigest()}  {name}')
    manifest.write_text('\n'.join(rows) + '\n')
    print(f'Archived {len(model_paths)} models, {len(textures)} sprites and {len(info)} render definitions')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('manifest', type=Path)
    args = parser.parse_args()
    build(args.source, args.output, args.manifest)


if __name__ == '__main__':
    main()
