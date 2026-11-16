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


def check_direct_inputs(plan, workspace, meta, receipt, helpers):
    """Verify reviewed direct outputs without inventing Mixins AST sites.

    The caller still checks the remaining Mixins operations. A direct receipt is
    evidence to compare, never permission to update the reviewed contract.
    """
    required = meta.get('direct_attachments', [])
    actual = receipt.get('attachments', [])
    if not required or meta.get('direct_review_status') != 'reviewed':
        raise ValueError('Direct attachment boundaries await combined plan review')
    if receipt.get('horizon_revision') != meta.get('horizon_revision'):
        raise ValueError('Direct attachment Horizon revision differs')
    ids = [entry.get('id') for entry in required]
    if any(not isinstance(site, str) or not site for site in ids) or len(ids) != len(set(ids)):
        raise ValueError('Duplicate or invalid reviewed direct attachment identity')
    if actual != required:
        raise ValueError('Direct attachment receipt differs from reviewed contract')
    workspace, helpers = Path(workspace).resolve(), Path(helpers).resolve()
    final_hashes = {}
    for entry in required:
        relative = entry['file']
        owned = [e for section, key in (('injections', 'injection_id'), ('states', 'state_id'))
                 for e in meta.get(section, []) if e.get(key) == entry['id']]
        if any(e.get('file') != relative for e in owned):
            raise ValueError('Direct attachment moved its reviewed owner file')
        source = (workspace / relative).resolve()
        if not source.is_relative_to(workspace):
            raise ValueError('Direct attachment source escapes workspace')
        for key in ('before', 'after'):
            value = entry.get(key)
            if not isinstance(value, str) or len(value) != 64 or any(c not in '0123456789abcdef' for c in value):
                raise ValueError('Missing direct attachment source fingerprint')
        if relative in final_hashes:
            if entry['before'] != final_hashes[relative]:
                raise ValueError('Direct attachment source chain differs')
        elif entry['before'] != meta['official_source_sha256'].get(relative):
            raise ValueError('Direct attachment does not start at official source: ' + relative)
        final_hashes[relative] = entry['after']
        if 'helper_sha256' in entry:
            helper_name = meta.get('direct_helpers', {}).get(entry['id'])
            if not isinstance(helper_name, str):
                raise ValueError('Direct attachment helper has no reviewed path')
            helper = (helpers / helper_name).resolve()
            if not helper.is_relative_to(helpers) or digest(helper.read_bytes()) != entry['helper_sha256']:
                raise ValueError('Direct attachment helper bytes differ')
    for relative, expected in final_hashes.items():
        if digest((workspace / relative).read_bytes()) != expected:
            raise ValueError('Direct attachment output bytes differ: ' + relative)
        if plan.get('inputs', {}).get(relative) != expected:
            raise ValueError('Plan does not bind direct attachment output: ' + relative)
    for section, key in (('injections', 'injection_id'), ('states', 'state_id')):
        if any(entry.get(key) in ids for entry in plan.get(section, [])):
            raise ValueError('Direct attachment is also owned by Mixins')
    return set(ids)


def check(plan, workspace, meta=None, helpers=None, direct_receipt=None):
    meta = metadata() if meta is None else meta
    helpers = ROOT / 'mod' if helpers is None else Path(helpers)
    workspace = Path(workspace).resolve()
    direct_files = set()
    if direct_receipt is not None or meta.get('direct_attachments'):
        if direct_receipt is None:
            raise ValueError('Reviewed direct attachments require their receipt')
        transferred = check_direct_inputs(plan, workspace, meta, direct_receipt, helpers)
        direct_files = {entry['file'] for entry in meta['direct_attachments']}
        # Only the exact, reviewed direct ownership above replaces these sites.
        # No synthetic AST node/fingerprint is fabricated for a source rewrite.
        meta = dict(meta)
        meta['injections'] = [e for e in meta['injections'] if e['injection_id'] not in transferred]
        meta['states'] = [e for e in meta.get('states', []) if e['state_id'] not in transferred]
    if plan.get('schema') != meta['schema']:
        raise ValueError('Unreviewed structure plan schema')
    for relative, expected in meta['official_source_sha256'].items():
        # Direct files already bind the official preimage, every rewrite edge,
        # the actual final bytes and the plan input. They must not equal the
        # original bytes after an intentional source rewrite.
        if relative in direct_files:
            continue
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
    reviewed_ids = [entry['injection_id'] for entry in meta['injections']]
    if len(reviewed_ids) != len(set(reviewed_ids)):
        raise ValueError('Duplicate reviewed structure hook identity')
    for required in meta['injections']:
        targets.add(required['target'])
        injection_ids.add(required['injection_id'])
        # One function can own a reviewed HEAD plus a distinct local-value
        # adapter. Match each exact identity, then reject every unreviewed
        # identity on these targets below; no foreign/extra hook is skipped.
        selected = [entry for entry in plan.get('injections', [])
                    if entry.get('injection_id') == required['injection_id']]
        if len(selected) != 1:
            raise ValueError('Missing, duplicated or competing structure hook: ' + required['target'])
        entry = selected[0]
        contract_keys = ('injection_id', 'target', 'file', 'function_fingerprint', 'kind', 'cancellable', 'bindings')
        contract_keys += tuple(key for key in ('selector', 'ordinal', 'require', 'site_fingerprint', 'variable', 'parameter') if key in required)
        for key in contract_keys:
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
    # Composite guards may also own states at an exclusive source boundary.
    # Accept only their exact contract, never merely a recognized state ID.
    reviewed_states = [dict(required, mod_id=MOD_ID) for required in meta.get('states', [])]
    def matches_state(entry, required):
        # PlannedState also carries packaging metadata (mod_version/priority),
        # not part of an owner/initializer/constructor contract.
        return all(entry.get(key) == value for key, value in required.items())
    for required in reviewed_states:
        selected = [entry for entry in plan.get('states', [])
                    if entry.get('state_id') == required.get('state_id')]
        if len(selected) != 1 or not matches_state(selected[0], required):
            raise ValueError('Reviewed structure state/constructor contract differs')
    for entry in plan.get('states', []):
        if entry.get('file') in exclusive or any(c.get('file') in exclusive for c in entry.get('constructors', [])):
            if not any(matches_state(entry, required) for required in reviewed_states):
                raise ValueError('Unreviewed structure state/constructor rewrite')
    for entry in plan.get('bridges', []):
        if entry.get('file') in exclusive:
            raise ValueError('Unreviewed bridge at a private structure boundary')


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    try:
        if len(args) not in (2, 3):
            raise ValueError('usage: check_structure_plan.py PLAN.json INPUT_WORKSPACE [DIRECT_RECEIPT.json]')
        receipt = json.loads(Path(args[2]).read_text(encoding='utf-8')) if len(args) == 3 else None
        check(json.loads(Path(args[0]).read_text(encoding='utf-8')), Path(args[1]), direct_receipt=receipt)
    except (ValueError, OSError, KeyError, TypeError) as error:
        print('Structure composition rejected: ' + str(error), file=sys.stderr)
        return 1
    print('Structure native inputs, package bytes, contracts and exclusive boundaries verified.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
