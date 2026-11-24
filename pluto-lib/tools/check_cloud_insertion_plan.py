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


def native_metadata():
    return json.loads((ROOT / 'tools/native-cloud-preservation.json').read_text())


def check_native(plan, workspace):
    meta = native_metadata()
    if plan.get('schema') != meta['schema']:
        raise ValueError('Unreviewed native cloud preservation schema')
    root = Path(workspace).resolve()
    for name, expected in meta['sources'].items():
        source = root / name
        if source.is_symlink() or not source.is_file():
            raise ValueError('Native cloud source must be a regular file: ' + name)
        source.resolve().relative_to(root)
        if plan.get('inputs', {}).get(name) != expected or digest(source.read_bytes()) != expected:
            raise ValueError('Native cloud plan/workspace source differs: ' + name)
    permitted = {}
    for section, key, field, source_key in (
            ('modules', 'contact_module', 'source', 'module'),
            ('injections', 'contact_hook', 'helper_source', 'hook')):
        expected = meta[key]
        identifier = 'module_id' if section == 'modules' else 'injection_id'
        selected = [entry for entry in plan.get(section, [])
                    if entry.get(identifier) == expected[identifier]]
        if len(selected) != 1:
            raise ValueError('Expected one reviewed cloud-adjacent contact ' + section)
        entry = selected[0]
        if any(entry.get(k) != value for k, value in expected.items()):
            raise ValueError('Cloud-adjacent contact identity/phase differs')
        if section == 'modules' and entry.get('visibility', 'pub(crate)') != 'pub(crate)':
            raise ValueError('Cloud-adjacent contact visibility differs')
        raw = (ROOT / 'mod' / meta[source_key + '_source']).read_bytes()
        if digest(raw) != meta[source_key + '_sha256'] or entry.get(field, '').encode() != raw:
            raise ValueError('Cloud-adjacent contact source differs')
        permitted[section] = entry
    # Further reviewed attachments into preserved files (the contract's
    # permitted_hooks): when present, one must match its reviewed identity and
    # packaged helper bytes exactly, and it is then exempt from the ban below.
    # Absence is not an error: such a hook only observes the boundary; it is
    # not part of what this guard preserves.
    permitted_hooks = set()
    for expected in meta.get('permitted_hooks', []):
        selected = [entry for entry in plan.get('injections', [])
                    if entry.get('injection_id') == expected['injection_id']]
        if len(selected) > 1:
            raise ValueError('Duplicated reviewed permitted cloud-boundary hook')
        if not selected:
            continue
        entry = selected[0]
        contract = {k: v for k, v in expected.items()
                    if k not in ('helper_source', 'helper_sha256')}
        if any(entry.get(k) != value for k, value in contract.items()):
            raise ValueError('Permitted cloud-boundary hook identity/phase differs')
        raw = (ROOT / 'mod' / expected['helper_source']).read_bytes()
        if digest(raw) != expected['helper_sha256'] or entry.get('helper_source', '').encode() != raw:
            raise ValueError('Permitted cloud-boundary hook source differs')
        permitted_hooks.add(id(entry))
    forbidden_names = {entry['name'] for entry in metadata()['modules']}
    prefixes = tuple(meta['protected_prefixes'])
    for section in ('modules', 'injections', 'states'):
        for entry in plan.get(section, []):
            if any(c.get('file') in meta['sources'] for c in entry.get('constructors', [])):
                raise ValueError('State constructor changes a preserved native cloud source')
            if entry is permitted.get(section) or id(entry) in permitted_hooks:
                continue
            if (entry.get('mod_id') == MOD_ID or entry.get('name') in forbidden_names
                    or entry.get('file') in meta['sources']
                    or entry.get('target', '').startswith(prefixes)
                    or entry.get('function_fingerprint') == meta['contact_hook']['function_fingerprint']):
                raise ValueError('Unreviewed attachment changes a preserved native cloud boundary')


def check(plan, workspace, meta=None, helpers=None):
    if any(entry.get('mod_id') == 'pluto_lib'
           for section in ('injections', 'modules', 'states') for entry in plan.get(section, [])):
        if meta is not None or helpers is not None:
            raise ValueError('Native cloud preservation forbids metadata/helper overrides')
        check_native(plan, workspace)
        return
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
    print('Cloud boundary verified; native preservation supplies no custom insertion authority API.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
