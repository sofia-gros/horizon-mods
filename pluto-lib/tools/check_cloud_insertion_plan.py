#!/usr/bin/env python3
"""Fail-closed complete-plan composition guard for native cloud insertion.

Checks the actual copied package bytes and official inputs, not a promise that
matching numeric epochs confer authority. No native execution proof is implied.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
MOD_ID = 'pluto_native_creative'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def metadata():
    return json.loads((ROOT / 'tools/cloud-insertion-boundaries.json').read_text())


def check(plan, workspace, meta=None, helpers=None):
    meta = metadata() if meta is None else meta
    helpers = ROOT / 'mixins/pluto_native_creative' if helpers is None else Path(helpers)
    workspace = Path(workspace).resolve()
    if plan.get('schema') != meta['schema']:
        raise ValueError('Unreviewed cloud insertion plan schema')
    for relative, expected in meta['official_source_sha256'].items():
        path = (workspace / relative).resolve()
        if not path.is_relative_to(workspace) or digest(path.read_bytes()) != expected:
            raise ValueError('Official cloud authority/publication source drift: ' + relative)
        if plan.get('inputs', {}).get(relative) != expected:
            raise ValueError('Cloud plan is not bound to the reviewed official input: ' + relative)
    expected_slots = set()
    for required in meta['modules']:
        expected_slots.add((required['file'], required['name']))
        selected = [entry for entry in plan.get('modules', [])
                    if entry.get('module_id') == required['module_id']
                    or (entry.get('file'), entry.get('name')) == (required['file'], required['name'])]
        if len(selected) != 1:
            raise ValueError('Missing, duplicated or relocated cloud module: ' + required['name'])
        entry = selected[0]
        for key in ('module_id', 'file', 'name'):
            if entry.get(key) != required[key]:
                raise ValueError('Cloud module identity/placement differs: ' + required['name'])
        # Official schema 6 omits the default pub(crate) value when serializing.
        visibility = entry.get('visibility', 'pub(crate)')
        if entry.get('mod_id') != MOD_ID or visibility != required['visibility']:
            raise ValueError('Cloud module owner/visibility differs: ' + required['name'])
        helper = (helpers / required['source_file']).read_bytes()
        if digest(helper) != required['source_sha256'] or entry.get('source', '').encode() != helper:
            raise ValueError('Reviewed cloud source differs from actual packaged module: ' + required['name'])
    exclusive = set(meta['exclusive_files'])
    protected_states = exclusive | set(meta['protected_state_files'])
    prefixes = tuple(meta['protected_target_prefixes'])
    for entry in plan.get('modules', []):
        slot = (entry.get('file'), entry.get('name'))
        if entry.get('file') in exclusive and slot not in expected_slots:
            raise ValueError('Unreviewed module at a private cloud authority boundary')
    for entry in plan.get('injections', []):
        if entry.get('file') in exclusive or entry.get('target', '').startswith(prefixes):
            raise ValueError('Hook changes a reserved cloud authority/publication function')
    for entry in plan.get('states', []):
        if entry.get('file') in protected_states or entry.get('target', '').startswith(prefixes):
            raise ValueError('State changes a reserved cloud authority/publication type')
        if any(c.get('file') in protected_states for c in entry.get('constructors', [])):
            raise ValueError('State constructor rewrites a reserved cloud boundary')


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    try:
        if len(args) != 2:
            raise ValueError('usage: check_cloud_insertion_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        check(json.loads(Path(args[0]).read_text()), Path(args[1]))
    except (ValueError, OSError, KeyError, TypeError) as error:
        print('Cloud insertion composition rejected: ' + str(error), file=sys.stderr)
        return 1
    print('Cloud insertion native inputs, package bytes, placement and exclusive boundaries verified.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
