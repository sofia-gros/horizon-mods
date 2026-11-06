#!/usr/bin/env python3
"""Prepare actual adapter + exact official structures, with the real items crate.

No host edits/downloads. Cargo metadata is used only when this explicit fixture
preparation is authorized; merely adding this file runs nothing.
"""
import argparse
import json
from pathlib import Path
import shutil
import tomllib
import prepare_advancement_projection_fixture as common
from prepare_recipe_preview_fixture import fragment

ROOT = Path(__file__).resolve().parents[1]
LOCAL = {('pluto-recipe-presentation-tests', '0.0.0', None, None),
         ('pluto-inventory-api', '0.1.0', None, None)}


def verify(output):
    evidence = json.loads((output / 'fixture-sources.json').read_text())
    host = Path(evidence['official_source'])
    common.clean_official(host, evidence['official_revision'])
    for path, sha in evidence['official_sha256'].items():
        if common.digest((host / path).read_bytes()) != sha:
            raise ValueError('Official source drift: ' + path)
    for path, sha in evidence['files_sha256'].items():
        if common.digest((output / path).read_bytes()) != sha:
            raise ValueError('Fixture source drift: ' + path)
    for part in evidence['fragments']:
        text = fragment((host / part['source']).read_text(), part['start'], part['end'])
        if common.digest(text.encode()) != part['sha256'] or text not in (output / part['destination']).read_text():
            raise ValueError('Official declaration/layout fragment is not byte exact')
    official = {common.identity(p) for p in tomllib.loads((host / 'Cargo.lock').read_text())['package']}
    resolved = {common.identity(p) for p in tomllib.loads((output / 'Cargo.lock').read_text())['package']}
    if resolved - official - LOCAL or not LOCAL.issubset(resolved):
        raise ValueError('Unexpected dependency identity')
    required = {'pluto-recipe-presentation-tests': output / 'Cargo.toml',
                'pluto-inventory-api': output / 'inventory-api/Cargo.toml'}
    seen = set()
    for package in common.metadata(output, locked=True)['packages']:
        path = Path(package['manifest_path']).resolve()
        if package['name'] in required:
            if package['source'] is not None or path != required[package['name']].resolve():
                raise ValueError('Actual local crate path changed')
            seen.add(package['name'])
        elif package['source'] is None and not path.is_relative_to(host):
            raise ValueError('Unexpected native dependency path')
    if seen != set(required):
        raise ValueError('Missing actual local package')
    return evidence


def prepare(host, output):
    host = host.resolve()
    native = ROOT / 'mixins/pluto_native_creative'
    pin = tomllib.loads((native / 'horizon-mixin.toml').read_text())['compatibility']['horizon_revision']
    common.clean_official(host, pin)
    if output.exists():
        raise ValueError('Output must be new')
    output.mkdir(parents=True)
    shutil.copytree(native / 'inventory-api', output / 'inventory-api',
                    ignore=shutil.ignore_patterns('target', 'Cargo.lock'))
    shutil.copyfile(native / 'recipe-book-presentation.rs', output / 'presentation.rs')
    shutil.copyfile(ROOT / 'tests/native/recipe_presentation.rs', output / 'lib.rs')
    records = []

    def take(file, start, end, destination):
        text = fragment((host / file).read_text(), start, end)
        records.append(dict(source=file, start=start, end=end, destination=destination,
                            sha256=common.digest(text.encode())))
        return text

    crafting = 'crates/horizon-client/src/crafting.rs'
    book = 'crates/horizon-client/src/recipe_book.rs'
    (output / 'crafting.rs').write_text('use std::collections::HashSet;\nuse serde_json::Value;\n' +
        take(crafting, '#[derive(Clone, Debug)]\npub struct Ingredient', 'impl Ingredient {', 'crafting.rs') +
        take(crafting, '#[derive(Clone, Debug, Default)]\npub struct Placement', 'impl Placement {', 'crafting.rs'))
    parts = [take(book, '#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]\npub enum BookType', 'impl BookType {', 'recipe_book.rs'),
        take(book, '#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]\npub enum Category', 'impl Category {', 'recipe_book.rs'),
        take(book, '#[derive(Clone, Debug)]\npub enum Display', '/// A group of recipes shown as one button', 'recipe_book.rs'),
        take(book, '#[derive(Default)]\npub struct Book', 'pub(crate) fn crafting_category', 'recipe_book.rs')]
    (output / 'recipe_book.rs').write_text('use crate::crafting::{Ingredient,Placement};\nuse horizon_items::items::ItemStack;\nuse std::collections::HashMap;\n' +
        ''.join(parts) + '\n#[path="presentation.rs"] pub mod pluto_presentation;\n')
    manifest = '[package]\nname="pluto-recipe-presentation-tests"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\nserde_json="1"\npluto-inventory-api={path="inventory-api"}\n'
    for name in ('horizon-items', 'horizon-world'):
        manifest += name + '={path=' + json.dumps(str(host / 'crates' / name)) + '}\n'
    manifest += '\n[patch.crates-io]\n'
    for name, entry in tomllib.loads((host / 'Cargo.toml').read_text()).get('patch', {}).get('crates-io', {}).items():
        if set(entry) != {'path'}:
            raise ValueError('Unexpected official patch')
        path = (host / entry['path']).resolve()
        if not path.is_relative_to(host):
            raise ValueError('Official dependency escapes checkout')
        manifest += json.dumps(name) + '={path=' + json.dumps(str(path)) + '}\n'
    (output / 'Cargo.toml').write_text(manifest)
    evidence = dict(official_source=str(host), official_revision=pin,
        official_sha256={p: common.digest((host / p).read_bytes()) for p in (crafting, book, 'Cargo.lock')},
        fragments=records,
        files_sha256={p.relative_to(output).as_posix(): common.digest(p.read_bytes()) for p in output.rglob('*') if p.is_file()},
        scope='Actual production adapter and complete real API; exact native Book/Entry/Ingredient/layout structures and real ItemStack. No Game, UI lifecycle, automatic hook or placement claim.')
    (output / 'Cargo.lock').write_bytes((host / 'Cargo.lock').read_bytes())
    common.metadata(output, locked=False)
    (output / 'fixture-sources.json').write_text(json.dumps(evidence, indent=2) + '\n')
    verify(output)
    print(output)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--horizon-source', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--verify-lock', action='store_true')
    args = parser.parse_args()
    if args.verify_lock:
        verify(args.output)
        print('Exact official fragments, adapter/API hashes and dependency paths verified')
    elif args.horizon_source is None:
        parser.error('--horizon-source is required')
    else:
        prepare(args.horizon_source, args.output)
