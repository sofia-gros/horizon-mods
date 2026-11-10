#!/usr/bin/env python3
"""Fail-closed composition guard for PlutoLib's native structure adapters.

Checks official source authority and actual packaged module/helper bytes. A
passing plan is not proof that a structure was placed or a save was reloaded.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
MOD_ID = 'pluto_lib'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def metadata():
    return json.loads((ROOT / 'tools/structure-boundaries.json').read_text(encoding='utf-8'))


def check(plan, workspace, meta=None, helpers=None):
    meta = metadata() if meta is None else meta
    helpers = ROOT / 'mod' if helpers is None else Path(helpers)
    workspace = Path(workspace).resolve()
    if plan.get('schema') != meta['schema']:
        raise ValueError('Unreviewed structure plan schema')
    for relative, expected in meta['official_source_sha256'].items():
        source = (workspace / relative).resolve()
        if not source.is_relative_to(workspace) or digest(source.read_bytes()) != expected:
            raise ValueError('Official structure authority source drift: ' + relative)
        if plan.get('inputs', {}).get(relative) != expected:
            raise ValueError('Structure plan is not bound to official input: ' + relative)
    slots = set()
    for required in meta['modules']:
        slots.add((required['file'], required['name']))
        selected = [entry for entry in plan.get('modules', [])
                    if entry.get('module_id') == required['module_id']
                    or (entry.get('file'), entry.get('name')) == (required['file'], required['name'])]
        if len(selected) != 1:
            raise ValueError('Missing, duplicated or relocated structure module: ' + required['name'])
        entry = selected[0]
        for key in ('module_id', 'file', 'name', 'visibility'):
            if entry.get(key, 'pub(crate)' if key == 'visibility' else None) != required[key]:
                raise ValueError('Structure module placement/visibility differs: ' + required['name'])
        source = (helpers / required['source_file']).read_bytes()
        if entry.get('mod_id') != MOD_ID or digest(source) != required['source_sha256'] or entry.get('source', '').encode() != source:
            raise ValueError('Structure module owner or actual package bytes differ: ' + required['name'])
    targets = set()
    injection_ids = set()
    for required in meta['injections']:
        targets.add(required['target'])
        injection_ids.add(required['injection_id'])
        selected = [entry for entry in plan.get('injections', [])
                    if entry.get('injection_id') == required['injection_id'] or entry.get('target') == required['target']]
        if len(selected) != 1:
            raise ValueError('Missing, duplicated or competing structure hook: ' + required['target'])
        entry = selected[0]
        for key in ('injection_id', 'target', 'file', 'function_fingerprint', 'kind', 'cancellable', 'bindings'):
            if entry.get(key) != required[key]:
                raise ValueError('Structure hook contract differs: ' + required['target'] + ' / ' + key)
        source = (helpers / required['source_file']).read_bytes()
        if entry.get('mod_id') != MOD_ID or digest(source) != required['source_sha256'] or entry.get('helper_source', '').encode() != source:
            raise ValueError('Structure hook owner or actual package bytes differ: ' + required['target'])
    exclusive = set(meta['exclusive_files'])
    for entry in plan.get('modules', []):
        if entry.get('file') in exclusive and (entry.get('file'), entry.get('name')) not in slots:
            raise ValueError('Unreviewed module at a private structure boundary')
    for entry in plan.get('injections', []):
        if entry.get('file') in exclusive or entry.get('target') in targets:
            if entry.get('injection_id') not in injection_ids:
                raise ValueError('Unreviewed injection at a private structure boundary')
    for entry in plan.get('states', []):
        if entry.get('file') in exclusive or any(c.get('file') in exclusive for c in entry.get('constructors', [])):
            raise ValueError('Unreviewed structure state/constructor rewrite')
    for entry in plan.get('bridges', []):
        if entry.get('file') in exclusive:
            raise ValueError('Unreviewed bridge at a private structure boundary')


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    try:
        if len(args) != 2:
            raise ValueError('usage: check_structure_plan.py PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        check(json.loads(Path(args[0]).read_text(encoding='utf-8')), Path(args[1]))
    except (ValueError, OSError, KeyError, TypeError) as error:
        print('Structure composition rejected: ' + str(error), file=sys.stderr)
        return 1
    print('Structure native inputs, package bytes, contracts and exclusive boundaries verified.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
