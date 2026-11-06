"""Exact, reviewed composition contracts; never infer authority from an ID.

This file does not discover a safe set by scanning names or positions. Its input
is an explicit source-reviewed contract. Missing candidate observation fails
closed until the newly composed plan has supplied the new placement identity.
"""
import hashlib
import json
import re


def canonical(value):
    # Unlike Python ==, this distinguishes true from 1 in IR contracts.
    return json.dumps(value, ensure_ascii=True, sort_keys=True,
                      separators=(',', ':'), allow_nan=False)


def projection(entry, kind):
    result = dict(entry)
    body = {'modules': 'source', 'injections': 'helper_source'}.get(kind)
    if body is not None:
        source = result.pop(body, None)
        if not isinstance(source, str) or body + '_sha256' in result:
            raise ValueError('Cloud contract needs actual source text, not an asserted hash')
        result[body + '_sha256'] = hashlib.sha256(source.encode()).hexdigest()
    if kind == 'modules':
        result.setdefault('visibility', 'pub(crate)')
    return result


def _slot(entry, kind):
    if kind == 'modules':
        return entry.get('file'), entry.get('name')
    if kind == 'states':
        return entry.get('target'), entry.get('name')
    return None


def validate_entries(plan, contract):
    """Validate reviewed entry bytes and exact semantic IR, not admission alone."""
    if type(contract.get('schema')) is not int or contract['schema'] != 1:
        raise ValueError('Unknown cloud composition contract schema')
    if type(plan.get('schema')) is not int or plan['schema'] != 6:
        raise ValueError('Unknown cloud composition plan schema')
    for kind, identity in [('modules', 'module_id'), ('injections', 'injection_id'),
                           ('states', 'state_id')]:
        rows = contract.get(kind)
        actual = plan.get(kind)
        if not isinstance(rows, list) or not rows or not isinstance(actual, list):
            raise ValueError('Missing cloud composition collection: ' + kind)
        ids = [row.get(identity) for row in rows]
        if any(not isinstance(i, str) or not i for i in ids) or len(set(ids)) != len(ids):
            raise ValueError('Duplicate or missing cloud contract identity')
        for required in rows:
            chosen = [entry for entry in actual
                      if entry.get(identity) == required[identity]
                      or (kind != 'injections' and _slot(entry, kind) == _slot(required, kind))]
            if len(chosen) != 1:
                raise ValueError('Missing/duplicated/relocated cloud dependency: ' + required[identity])
            if canonical(projection(chosen[0], kind)) != canonical(required):
                raise ValueError('Cloud dependency source/IR differs: ' + required[identity])


def validate_contracts(plan, contract):
    observation = contract.get('candidate_observation')
    # No fabricated fingerprint, sibling-copy, or wildcard candidate placement.
    if (not isinstance(observation, dict)
            or observation.get('runtime_commit') != contract.get('candidate_runtime_commit')
            or not isinstance(observation.get('plan_sha256'), str)
            or re.fullmatch('[0-9a-f]{64}', observation['plan_sha256']) is None
            or not isinstance(observation.get('evidence_commit'), str)
            or re.fullmatch('[0-9a-f]{40}', observation['evidence_commit']) is None):
        raise ValueError('Candidate cloud placement has no reviewed actual-plan observation')
    for row in contract.get('modules', []):
        fingerprint = row.get('fingerprint')
        if not isinstance(fingerprint, str) or re.fullmatch('[0-9a-f]{64}', fingerprint) is None:
            raise ValueError('Unobserved cloud module fingerprint: ' + str(row.get('module_id')))
    validate_entries(plan, contract)
    hook_records = {canonical(row) for row in contract['injections']}
    targets = set(contract['protected_targets'])
    prefixes = tuple(contract['protected_module_prefixes'])
    hook_sites = {(row['file'], tuple(row['node'])) for row in contract['injections']}

    def protects(entry):
        return (entry.get('target') in targets
                or entry.get('target', '').startswith(prefixes)
                or (entry.get('file'), tuple(entry.get('node', []))) in hook_sites)

    def allows(entry):
        return canonical(projection(entry, 'injections')) in hook_records

    # Inspect all kinds/phases on protected targets, not just HEAD, IDs, count,
    # or the observed node. Unknown RETURN/STORE/cancellable additions fail.
    for entry in plan['injections']:
        if protects(entry) and not allows(entry):
            raise ValueError('Unreviewed hook in cloud caller/commit composition')
    return {
        'module_slots': {_slot(row, 'modules') for row in contract['modules']},
        'protects_hook': protects,
        'allows_hook': allows,
    }
