#!/usr/bin/env python3
"""Bounded, read-only terrain plan witness. Never updates production guards."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

PLAN_BYTES = 32 * 1024 * 1024
SOURCE_BYTES = 2 * 1024 * 1024
OUTPUT_BYTES = 128 * 1024
MAX_ROWS = 4096
MAX_INPUTS = 20000
MAX_COMPETITORS = 64
HEX = re.compile(r'[0-9a-f]{64}\Z')
MESHER = 'crates/horizon-client-render/src/mesher.rs'


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def pairs(values):
    result = {}
    for key, value in values:
        require(key not in result, 'duplicate JSON key')
        result[key] = value
    return result


def decode(raw):
    return json.loads(raw.decode('utf-8'), object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'nonfinite JSON number'))


def regular(path, limit):
    """Reject symlinks/nonregular inputs and bound reads, including growth."""
    path = Path(os.path.abspath(path))
    for part in (path, *path.parents):
        require(not part.is_symlink(), 'symlink input')
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and before.st_size <= limit, 'file type/size limit')
        chunks, remaining = [], limit + 1
        while remaining:
            block = os.read(fd, min(65536, remaining))
            if not block:
                break
            chunks.append(block)
            remaining -= len(block)
        raw = b''.join(chunks)
        after = os.fstat(fd)
    finally:
        os.close(fd)
    require(len(raw) <= limit, 'file grew past limit')
    require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) ==
            (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns), 'input changed during read')
    return raw


def relative(value):
    require(type(value) is str and 0 < len(value) <= 512, 'invalid relative path')
    require(not value.startswith('/') and '\\' not in value and
            all(x not in ('', '.', '..') for x in value.split('/')), 'unsafe relative path')
    return value


def text(value, limit=1024):
    require(type(value) is str and 0 < len(value.encode('utf-8')) <= limit, 'invalid bounded text')
    return value


def fingerprint(value):
    require(type(value) is str and HEX.fullmatch(value), 'invalid fingerprint')
    return value


def node(value):
    require(type(value) is list and 0 < len(value) <= 32 and
            all(type(x) is int and 0 <= x <= 2**32-1 for x in value), 'invalid node')
    return value


def rows(plan, name):
    value = plan.get(name)
    require(type(value) is list and len(value) <= MAX_ROWS and
            all(type(x) is dict for x in value), 'missing/oversized ' + name)
    return value


def payload(value):
    require(type(value) is str, 'invalid payload')
    raw = value.encode('utf-8')
    require(len(raw) <= SOURCE_BYTES, 'oversized payload')
    return {'sha256': sha(raw), 'bytes': len(raw)}


def hook_record(row):
    for key in ('mod_id', 'mod_version', 'injection_id', 'target', 'kind'):
        text(row.get(key))
    relative(row.get('file'))
    node(row.get('node'))
    fingerprint(row.get('function_fingerprint'))
    require(type(row.get('cancellable')) is bool, 'invalid cancellation flag')
    require(type(row.get('priority')) is int and -2**31 <= row['priority'] < 2**31,
            'invalid priority')
    bindings = row.get('bindings')
    require(type(bindings) is list and len(bindings) <= 64, 'invalid bindings')
    for binding in bindings:
        require(type(binding) is dict and set(binding) == {'name', 'ty', 'value'}, 'invalid binding')
        for value in binding.values():
            text(value, 4096)
    contract = row.get('contract')
    require(type(contract) is dict and len(json.dumps(contract).encode()) <= 8192, 'invalid contract')
    result = {k: row[k] for k in ('mod_id', 'mod_version', 'injection_id', 'target', 'file',
              'node', 'function_fingerprint', 'kind', 'cancellable', 'priority', 'bindings', 'contract')}
    site_keys = {'hook', 'hook_version', 'argument', 'constant_site', 'call_site',
                 'site_fingerprint', 'selector', 'ordinal', 'require', 'captures',
                 'expression_site', 'variable_site', 'variable', 'slice'}
    require(not (set(row) - set(result) - site_keys - {'helper_source'}), 'unknown hook field')
    result['site'] = {k: row[k] for k in sorted(site_keys) if k in row}
    require(len(json.dumps(result['site']).encode()) <= 8192, 'site detail byte limit')
    result['payload'] = payload(row.get('helper_source'))
    return result


def module_record(row):
    for key in ('mod_id', 'mod_version', 'module_id', 'name'):
        text(row.get(key))
    relative(row.get('file'))
    fingerprint(row.get('fingerprint'))
    visibility = row.get('visibility', 'pub(crate)')
    require(visibility in ('pub(crate)', 'pub'), 'unknown module visibility')
    result = {k: row[k] for k in ('mod_id', 'mod_version', 'module_id', 'file', 'name', 'fingerprint')}
    require(not (set(row) - set(result) - {'visibility', 'source'}), 'unknown module field')
    result['visibility'] = visibility
    result['payload'] = payload(row.get('source'))
    return result


def verify_sources(contract, official_root, mod_root):
    require(contract.get('schema') == 1 and len(contract.get('hooks', [])) == 2 and
            len(contract.get('modules', [])) == 6, 'unexpected witness contract')
    expected = contract.get('official_source_sha256')
    require(type(expected) is dict and len(expected) == 13, 'unexpected official source scope')
    for path, digest in expected.items():
        require(sha(regular(Path(official_root) / relative(path), SOURCE_BYTES)) == fingerprint(digest),
                'official source mismatch: ' + path)
    native = Path(mod_root) / 'pluto-lib/mixins/pluto_native_creative'
    result = {}
    for row in contract['hooks'] + contract['modules']:
        path = relative(row.get('helper', row.get('source')))
        raw = regular(native / path, SOURCE_BYTES)
        require(sha(raw) == fingerprint(row['payload_sha256']), 'source payload mismatch: ' + path)
        require(path not in result, 'duplicate source payload')
        result[path] = raw.decode('utf-8')
    return result


def collect(raw, contract, sources):
    require(len(raw) <= PLAN_BYTES, 'plan byte limit')
    plan = decode(raw)
    require(type(plan) is dict and plan.get('schema') == 6, 'unreviewed plan schema')
    text(plan.get('transformer'))
    text(plan.get('rustc'))
    inputs = plan.get('inputs')
    require(type(inputs) is dict and len(inputs) <= MAX_INPUTS, 'invalid input inventory')
    for path, digest in contract['official_source_sha256'].items():
        require(inputs.get(path) == digest, 'plan official source mismatch: ' + path)
    hooks, modules = rows(plan, 'injections'), rows(plan, 'modules')
    # An observation of a malformed duplicate plan is not a usable witness.
    seen = set()
    for row in hooks:
        key = (text(row.get('mod_id')), text(row.get('injection_id')))
        require(key not in seen, 'duplicate hook ID')
        seen.add(key)
    seen = set()
    for row in modules:
        key = (relative(row.get('file')), text(row.get('name')))
        require(key not in seen, 'duplicate module destination')
        seen.add(key)
    selected_hooks = []
    selected_positions = set()
    for expected in contract['hooks']:
        matches = [(i, row) for i, row in enumerate(hooks)
                   if row.get('injection_id') == expected['injection_id']]
        require(len(matches) == 1, 'missing or ambiguous reviewed hook')
        index, row = matches[0]
        record = hook_record(row)
        for key, value in expected.items():
            if key not in ('helper', 'payload_sha256'):
                require(record.get(key) == value, 'reviewed hook contract mismatch: ' + key)
        source = sources[expected['helper']]
        require(row['helper_source'] == source and sha(source.encode()) == expected['payload_sha256'],
                'reviewed hook payload mismatch')
        selected_hooks.append(record)
        selected_positions.add(index)
    selected_modules = []
    for expected in contract['modules']:
        matches = [row for row in modules if row.get('module_id') == expected['module_id']]
        require(len(matches) == 1, 'missing or ambiguous reviewed module')
        row = matches[0]
        record = module_record(row)
        for key, value in expected.items():
            if key not in ('source', 'payload_sha256'):
                require(record.get(key) == value, 'reviewed module contract mismatch: ' + key)
        source = sources[expected['source']]
        require(row['source'] == source and sha(source.encode()) == expected['payload_sha256'],
                'reviewed module payload mismatch')
        selected_modules.append(record)
    competitors = []
    for index, row in enumerate(hooks):
        if index in selected_positions:
            continue
        reasons = []
        if row.get('file') == MESHER:
            reasons.append('reserved_mesher_file')
        for chosen in selected_hooks:
            same_target = row.get('target') == chosen['target']
            same_node = row.get('file') == chosen['file'] and row.get('node') == chosen['node']
            same_fingerprint = row.get('function_fingerprint') == chosen['function_fingerprint']
            if same_target or same_node or same_fingerprint:
                reasons.append('target_or_resolved_identity:' + chosen['injection_id'])
        if reasons:
            require(len(competitors) < MAX_COMPETITORS, 'competitor count limit')
            competitors.append({'plan_index': index, 'reasons': reasons, **hook_record(row)})
    result = {'schema': 1, 'status': 'observed_only', 'compatibility': None,
              'native_executed': False, 'plan': {'sha256': sha(raw), 'bytes': len(raw),
              'schema': 6, 'transformer': plan['transformer'], 'rustc': plan['rustc'],
              'hook_count': len(hooks), 'module_count': len(modules), 'input_count': len(inputs)},
              'selected_official_source_sha256': contract['official_source_sha256'],
              'hooks': selected_hooks, 'modules': selected_modules, 'competitors': competitors,
              'module_node_note': 'Schema 6 modules have file/fingerprint, not an AST node field.'}
    require(len(json.dumps(result, sort_keys=True).encode()) <= OUTPUT_BYTES, 'witness byte limit')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--official-root', type=Path, required=True)
    parser.add_argument('--mod-root', type=Path, required=True)
    args = parser.parse_args()
    raw = b''
    try:
        contract_raw = regular(Path(__file__).with_name('contract.json'), OUTPUT_BYTES)
        contract = decode(contract_raw)
        raw = regular(args.plan, PLAN_BYTES)
        sources = verify_sources(contract, args.official_root, args.mod_root)
        witness = collect(raw, contract, sources)
        # Repeat exact bytes after collection. No metadata or input file is written.
        require(raw == regular(args.plan, PLAN_BYTES), 'plan changed during collection')
        require(sources == verify_sources(contract, args.official_root, args.mod_root), 'source changed')
        require(contract_raw == regular(Path(__file__).with_name('contract.json'), OUTPUT_BYTES),
                'contract changed during collection')
        witness['contract_sha256'] = sha(contract_raw)
        encoded = json.dumps(witness, sort_keys=True, separators=(',', ':'))
        require(len(encoded.encode()) <= OUTPUT_BYTES, 'witness byte limit')
        print(encoded)
        return 0
    except (ValueError, OSError, KeyError, TypeError, RecursionError, UnicodeError) as error:
        print(json.dumps({'status': 'rejected', 'compatibility': None,
              'plan_sha256': sha(raw) if raw else None, 'error': str(error)[:512]}))
        return 1


if __name__ == '__main__':
    sys.exit(main())
