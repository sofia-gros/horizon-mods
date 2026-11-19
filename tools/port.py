#!/usr/bin/env python3
"""Clean-product boundary, deterministic work queue and development packaging.

Structural checks are not proof of native linkage or Java parity.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = Path('tconstruct/mixins/tconstruct')
CONTENT = Path('tconstruct/content')
# Content-mod inputs are discovered on disk but each must match one reviewed shape.
CONTENT_PATHS = (
    re.compile(r'horizon-mod\.toml'), re.compile(r'assets/tconstruct/lang/(?:en_us|ja_jp)\.json'), re.compile(r'data/tconstruct/horizon/(?:item|block|menu)/[a-z0-9_]+\.json'),
    # Machine-copied original data (tools/import_original_data.py): decoded by the production loaders in tests.
    re.compile(r'data/tconstruct/tinkering/(?:tool_definitions|modifiers|materials/(?:definition|stats|traits))/[a-z0-9_]+\.json'),
    re.compile(r'data/tconstruct/tinkering/tags/(?:materials|modifiers|damage_types)(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    re.compile(r'data/tconstruct/tags/(?:item|block|entity_type)(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    # Client assets imported from the original (MIT, assets/tconstruct/license): item definitions, models, blockstates, textures, equipment assets.
    re.compile(r'assets/tconstruct/license'), re.compile(r'assets/tconstruct/(?:items|blockstates|models|textures|equipment)(?:/[a-z0-9_]+)*/[a-z0-9_]+\.(?:json|png|png\.mcmeta)'),
    re.compile(r'data/tconstruct/recipes/tools/(?:materials|modifiers|parts/builder|recycling)(?:/[a-z0-9_]+)*/[a-z0-9_]+\.json'),
    # The damage type armor modules hurt their wearer with (data pack damage_type definition), and its entries in the two vanilla tags it joins.
    re.compile(r'data/tconstruct/damage_type/[a-z0-9_]+\.json'), re.compile(r'data/minecraft/tags/damage_type/(?:bypasses_armor|bypasses_effects)\.json'),
)
DAMAGE_TYPE_KEYS = {'message_id', 'scaling', 'exhaustion', 'effects', 'death_message_type'}
ITEM_KEYS = {'template', 'components', 'crafting_remainder', 'max_stack_size'}
BLOCK_KEYS = {'template', 'properties', 'item', 'components'}
MENU_KEYS = {'slots', 'data_slots', 'results'}


def plan(root):
    data = json.loads((root / 'docs/port-plan.json').read_text(encoding='utf-8'))
    if data.get('schema') != 1:
        raise ValueError('unsupported plan schema')
    tasks = data['tasks']
    ids = [t['id'] for t in tasks]
    if len(ids) != len(set(ids)):
        raise ValueError('duplicate task IDs')
    by_id = {t['id']: t for t in tasks}
    visiting, visited = set(), set()
    def visit(ident):
        if ident in visiting:
            raise ValueError('cyclic task dependencies')
        if ident in visited:
            return
        if ident not in by_id:
            raise ValueError(f'unknown prerequisite: {ident}')
        visiting.add(ident)
        for parent in by_id[ident]['depends_on']:
            visit(parent)
        visiting.remove(ident)
        visited.add(ident)
    active = 0
    for task in tasks:
        visit(task['id'])
        if task['status'] not in ('pending', 'in_progress', 'blocked', 'done'):
            raise ValueError('invalid task state')
        active += task['status'] == 'in_progress'
        if task['status'] == 'blocked' and not task.get('blocker'):
            raise ValueError('blocked task needs a concrete prerequisite')
        if task['status'] in ('done', 'in_progress'):
            if any(by_id[d]['status'] != 'done' for d in task['depends_on']):
                raise ValueError(f'{task["id"]}: prerequisite not complete')
        if task['status'] == 'done':
            if not task.get('evidence'):
                raise ValueError('completed task needs evidence')
            if task['kind'] == 'gameplay' and not any(e.get('stage') == 'game' and e.get('input_digest') and e.get('scenario') for e in task['evidence']):
                raise ValueError('gameplay completion needs input-bound game evidence')
    if active > 1:
        raise ValueError('only one active integration unit')
    return data


def next_task(root):
    data = plan(root)
    done = {t['id'] for t in data['tasks'] if t['status'] == 'done'}
    for state in ('in_progress', 'pending'):
        for task in data['tasks']:
            if task['status'] == state and set(task['depends_on']) <= done:
                return task
    return None


def checked_files(root):
    data = plan(root)
    base = root / BUNDLE
    if base.is_symlink() or not base.is_dir():
        raise ValueError('bundle must be a real directory')
    names = data['bundle_files']
    if len(names) != len(set(names)):
        raise ValueError('duplicate bundle input')
    files = {}
    for name in names:
        rel = Path(name)
        if rel.is_absolute() or '..' in rel.parts or '\\' in name or ':' in name:
            raise ValueError(f'unsafe bundle path: {name}')
        source = base / rel
        if any(p.is_symlink() for p in [source, *source.parents] if p != root.parent):
            raise ValueError(f'symlink bundle input: {name}')
        source.resolve().relative_to(base.resolve())
        files[name] = source.read_bytes()
    actual = {p.relative_to(base).as_posix() for p in base.rglob('*') if p.is_file()}
    if actual != set(names):
        raise ValueError(f'unreviewed/missing bundle files: {sorted(actual ^ set(names))}')
    manifest = tomllib.loads(files['horizon-mixin.toml'].decode())
    if set(manifest) != {'mod', 'compatibility', 'api', 'dependencies'}:
        raise ValueError('Tinkers may declare only mod, compatibility, api and dependencies; no injections/modules/state/bridges/crates')
    if manifest['mod']['id'] != 'tconstruct' or manifest['api'] != {'path': 'api'}:
        raise ValueError('unexpected product or API entry')
    if manifest['dependencies'] != {'pluto_lib': '^0.1'}:
        raise ValueError('Tinkers must depend only on the public PlutoLib bundle')
    pins = json.loads((root / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    if manifest['compatibility'] != {'horizon_revision': pins['horizon_revision']}:
        raise ValueError('Horizon pin differs')
    cargo = tomllib.loads(files['api/Cargo.toml'].decode())
    if cargo['package']['version'] != manifest['mod']['version']:
        raise ValueError('product versions differ')
    if set(cargo) - {'package', 'lib', 'dependencies'} or set(cargo['package']) - {'name', 'version', 'edition', 'publish'}:
        raise ValueError('unreviewed Cargo build/configuration surface')
    if cargo['lib'] != {'path': 'src/lib.rs'} or cargo['package'].get('publish') is not False:
        raise ValueError('unexpected crate entry/release permission')
    loader_dependency = {'git': 'https://github.com/horizon-rust/horizon',
                         'rev': pins['horizon_revision'], 'default-features': False}
    if cargo.get('dependencies', {}).get('horizon-client') != loader_dependency:
        raise ValueError('required loader metadata must use the official pinned Horizon with defaults disabled')
    for name, value in cargo.get('dependencies', {}).items():
        if name == 'horizon-client':
            continue  # Owner-approved metadata only; Rust host references remain forbidden below.
        if name not in data['cargo_dependencies'] or not isinstance(value, str) or name.startswith(('horizon', 'tconstruct')):
            raise ValueError(f'forbidden/unreviewed Cargo dependency: {name}')
    forbidden = re.compile(r'\b(?:horizon(?:_[A-Za-z0-9_]+)?|horizon_mixins|include|include_str|include_bytes|env|option_env)!?\s*(?:::|!|\()|\b(?:legacy|__horizon_mixins_)\b|#\s*\[\s*path\s*=|\bextern\s+crate|\bunsafe\b|\bstd\s*::\s*(?:process|fs|net)\b')
    for name, payload in files.items():
        if name.endswith('.rs') and forbidden.search(payload.decode('utf-8')):
            raise ValueError(f'host/archive/dynamic-source boundary in {name}')
    # Other native manifests/build scripts must not hide outside the packaged tree.
    for p in (root / 'tconstruct').rglob('*'):
        if p.is_symlink():
            raise ValueError(f'symlink in active product: {p}')
        if p.is_file() and p.name in ('horizon-mixin.toml', 'Cargo.toml', 'build.rs') and p not in {base / n for n in names}:
            raise ValueError(f'unreviewed build entry: {p}')
    digest = hashlib.sha256()
    for name, payload in sorted(files.items()):
        digest.update(name.encode() + b'\0' + payload + b'\0')
    return files, manifest, pins, digest.hexdigest()


def checked_content(root, version):
    """The clean Horizon content mod (.hmod): manifest plus declaratively reviewed data.

    Registration is data-only: Horizon has no code path for content mods and PlutoLib
    exposes no registration API, so every declaration must be a plain vanilla-template item.
    """
    base = root / CONTENT
    if base.is_symlink() or not base.is_dir():
        raise ValueError('content root must be a real directory')
    files = {}
    for path in sorted(p for p in base.rglob('*') if p.is_file()):
        if any(q.is_symlink() for q in [path, *path.parents] if q != root.parent):
            raise ValueError(f'symlink content input: {path}')
        name = path.relative_to(base).as_posix()
        if not any(shape.fullmatch(name) for shape in CONTENT_PATHS):
            raise ValueError(f'unreviewed content file: {name}')
        files[name] = path.read_bytes()
    if 'horizon-mod.toml' not in files:
        raise ValueError('content manifest missing')
    expected = {'id': 'tconstruct', 'name': "Tinkers' Construct content", 'version': version,
                'api': '0.2', 'side': 'both', 'capabilities': ['registries']}
    if tomllib.loads(files['horizon-mod.toml'].decode()) != expected:
        raise ValueError('content manifest differs from the reviewed one (version must match the native product)')
    for name, payload in files.items():
        if name.endswith('.json'):
            item = json.loads(payload.decode())
            if '/tinkering/' in name:
                if not isinstance(item, dict):
                    raise ValueError(f'tinkering data must be a JSON object: {name}')
                continue
            if '/recipes/' in name:
                if '/recipes/tools/modifiers/' in name:
                    reviewed = ('tconstruct:modifier', 'tconstruct:incremental_modifier', 'tconstruct:swappable_modifier', 'tconstruct:modifier_salvage', 'tconstruct:remove_modifier', 'tconstruct:multilevel_modifier', 'tconstruct:multilevel_incremental_modifier', 'tconstruct:overslime_modifier')
                elif '/recipes/tools/parts/builder/' in name:
                    reviewed = ('tconstruct:part_builder',)
                elif '/recipes/tools/recycling/' in name:
                    reviewed = ('tconstruct:part_builder_tool_recycling',)
                else:
                    reviewed = ('tconstruct:material',)
                if not isinstance(item, dict) or item.get('type') not in reviewed:
                    raise ValueError(f'only {" or ".join(reviewed)} recipes are reviewed here: {name}')
                continue
            if name.startswith('assets/tconstruct/') and not name.startswith('assets/tconstruct/lang/'):
                if not isinstance(item, dict):
                    raise ValueError(f'a client asset must be a JSON object: {name}')
                continue
            if name.startswith('assets/tconstruct/lang/'):
                # A language file: a flat object of translation keys to plain strings.
                if not isinstance(item, dict) or not all(isinstance(key, str) and isinstance(value, str) for key, value in item.items()):
                    raise ValueError(f'a language file must map keys to strings: {name}')
                continue
            if '/tags/' in name:
                if not isinstance(item, dict) or set(item) - {'values', 'replace', 'remove'} or not isinstance(item.get('values'), list):
                    raise ValueError(f'tag must be an object with a values list: {name}')
                continue
            if name.startswith('data/tconstruct/damage_type/'):
                if not isinstance(item, dict) or set(item) - DAMAGE_TYPE_KEYS or not isinstance(item.get('message_id'), str) or item.get('scaling') not in ('never', 'always', 'when_caused_by_living_non_player') or not isinstance(item.get('exhaustion'), (int, float)):
                    raise ValueError(f'damage type needs a message id, a scaling and an exhaustion, and only reviewed keys: {name}')
                continue
            if '/menu/' in name:
                if not isinstance(item, dict) or set(item) - MENU_KEYS or not isinstance(item.get('slots'), int) or not 1 <= item['slots'] <= 256:
                    raise ValueError(f'menu declaration needs a slot count and only reviewed keys: {name}')
                if not all(isinstance(slot, int) and 0 <= slot < item['slots'] for slot in item.get('results', [])):
                    raise ValueError(f'menu result slots must exist: {name}')
                continue
            keys = BLOCK_KEYS if '/block/' in name else ITEM_KEYS
            if not isinstance(item, dict) or set(item) - keys or not str(item.get('template', '')).startswith('minecraft:'):
                raise ValueError(f'declaration must copy a vanilla template with reviewed keys: {name}')
            if not all(isinstance(key, str) for key in item.get('components', {})):
                raise ValueError(f'invalid components in {name}')
            if '/block/' in name and not isinstance(item.get('properties', {}), dict):
                raise ValueError(f'block properties must be an object: {name}')
    digest = hashlib.sha256()
    for name, payload in sorted(files.items()):
        digest.update(name.encode() + b'\x00' + payload + b'\x00')
    return files, digest.hexdigest()


def package_content(root, output):
    files, manifest, _, _ = checked_files(root)
    content, digest = checked_content(root, manifest['mod']['version'])
    output = output.resolve()
    output.relative_to((root / '.local').resolve())
    if output.exists():
        raise ValueError('output exists; choose a new package name')
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'x') as archive:
        for name, payload in sorted(content.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, payload)
    with zipfile.ZipFile(output) as archive:
        if archive.testzip() or set(archive.namelist()) != set(content) or any(archive.read(n) != b for n, b in content.items()):
            raise ValueError('content package read-back differs')
    return digest


def release_preflight(root):
    """Reject unfinished clean ports; passing is not permission to publish.

    Artifact identity, native/runtime evidence review and tag validation remain
    separate gates. Development packaging deliberately does not call this.
    """
    data = plan(root)
    if not any(task['id'] == 'R5' and task['kind'] == 'gameplay' for task in data['tasks']):
        raise ValueError('release requires the whole-product R5 gameplay review')
    unfinished = [task['id'] for task in data['tasks'] if task['status'] != 'done']
    if unfinished:
        raise ValueError('clean port is not release-ready; unfinished tasks: ' + ', '.join(unfinished))
    return checked_files(root)[3]


def package_entries(root):
    """Return the exact checked archive payload, shared by packaging and validation."""
    files, manifest, pins, digest = checked_files(root)
    entries = {'horizon-native.json': (json.dumps({'schema': 1, 'id': 'tconstruct_native',
        'version': manifest['mod']['version'], 'game': '26.3', 'horizon': pins['horizon_revision'],
        'mixins': pins['mixins_revision']}, indent=2) + '\n').encode()}
    for name, payload in files.items():
        if name == 'horizon-mixin.toml':
            payload = payload.replace(b'id = "tconstruct"', b'id = "tconstruct_native"', 1)
        entries['mixins/tconstruct_native/' + name] = payload
    if len(entries) > 512 or sum(map(len, entries.values())) > 64 * 1024 * 1024:
        raise ValueError('official native archive limits exceeded')
    return entries, digest


def package(root, output):
    entries, digest = package_entries(root)
    output = output.resolve()
    # Packages are disposable outputs, never source/archive overwrites.
    output.relative_to((root / '.local').resolve())
    if output.exists():
        raise ValueError('output exists; choose a new package name')
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'x') as archive:
        for name, payload in sorted(entries.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, payload)
    with zipfile.ZipFile(output) as archive:
        if archive.testzip() or set(archive.namelist()) != set(entries) or any(archive.read(n) != b for n, b in entries.items()):
            raise ValueError('package read-back differs')
    return digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['next', 'check', 'package', 'package-content', 'release-check'])
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    try:
        if args.action == 'next':
            task = next_task(ROOT)
            if task is None:
                print('BLOCKED: no ready work; report concrete prerequisites once.')
                return 2
            print(json.dumps(task, ensure_ascii=False, indent=2))
        elif args.action == 'release-check':
            print(f'PASS: release prerequisites; product sha256={release_preflight(ROOT)}')
            print('Artifact/tag validation, native/runtime evidence review and publishing authorization are still required.')
        elif args.action == 'check':
            files, manifest, _, digest = checked_files(ROOT)
            print(f'PASS: {len(files)} explicit clean-product inputs; zero injection declarations; sha256={digest}')
            content, content_digest = checked_content(ROOT, manifest['mod']['version'])
            print(f'PASS: {len(content)} content-mod inputs (declarations only); sha256={content_digest}')
            print('Native linkage, gameplay parity and full retained CI: NOT VERIFIED by this check.')
        elif args.action == 'package-content':
            if args.output is None:
                raise ValueError('--output is required')
            print(f'PACKAGED content mod (not loaded/game-verified): {package_content(ROOT, args.output)}')
        else:
            if args.output is None:
                raise ValueError('--output is required')
            print(f'PACKAGED (not compiled/game-verified): {package(ROOT, args.output)}')
        return 0
    except (ValueError, KeyError, OSError, TypeError) as error:
        print(f'FAIL: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
