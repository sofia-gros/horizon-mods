"""Fail-closed composition check for the isolated item-name removal candidate.

This is not an execution grant. A passing plan is not a native/codec test.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent
OWNER = 'pluto_item_name_removal_candidate'
GUEST = 'crates/horizon-client/src/lan/guest.rs'
SYNC = 'crates/horizon-client/src/item_sync.rs'

def sha(data):
    return hashlib.sha256(data).hexdigest()

def metadata():
    return json.loads((ROOT / 'inputs.json').read_text())

def check_structure(plan, meta, candidate):
    if plan.get('schema') != 6:
        raise ValueError('Unreviewed plan schema')
    expected = {entry['expected']['target']: entry for entry in meta['injections']}
    hooks = plan.get('injections', [])
    for target, entry in expected.items():
        selected = [hook for hook in hooks if hook.get('target') == target]
        if len(selected) != 1:
            raise ValueError('Exclusive target must have exactly one hook: ' + target)
        actual = dict(selected[0])
        source = actual.pop('helper_source', None)
        if actual != entry['expected']:
            raise ValueError('Hook owner/order/site/contract drift: ' + target)
        if source != (candidate / entry['helper']).read_text():
            raise ValueError('Hook source drift')
    # A guest-local helper could arm the pub(super) gate before an unrelated
    # native call. This isolated contract therefore reserves guest.rs, not just
    # one target. Receive-side producers likewise remain native and exclusive.
    for hook in hooks:
        if hook.get('file') in (GUEST, SYNC) and hook.get('target') not in expected:
            raise ValueError('Unreviewed guest/receive injection')
        if hook.get('mod_id') == OWNER and hook.get('target') not in expected:
            raise ValueError('Unreviewed candidate injection')
    modules = plan.get('modules', [])
    allowed = {(entry['file'], entry['name']): entry for entry in meta['modules']}
    for identity, entry in allowed.items():
        selected = [m for m in modules if (m.get('file'), m.get('name')) == identity]
        if len(selected) != 1:
            raise ValueError('Candidate module missing or duplicated')
        module = selected[0]
        if (module.get('mod_id') != OWNER or module.get('mod_version') != '0.1.0'
                or module.get('module_id') != entry['module_id']
                or module.get('visibility', 'pub(crate)') != 'pub(crate)'
                or module.get('source') != (candidate / entry['helper']).read_text()):
            raise ValueError('Candidate module owner/source/visibility drift')
        if not isinstance(module.get('fingerprint'), str) or len(module['fingerprint']) != 64:
            raise ValueError('Missing native module destination fingerprint')
    for module in modules:
        identity = (module.get('file'), module.get('name'))
        if (module.get('file') in (GUEST, SYNC) or module.get('mod_id') == OWNER) and identity not in allowed:
            raise ValueError('Unreviewed guest/receive module')
    for kind in ('bridges', 'states'):
        for entry in plan.get(kind, []):
            files = [entry.get('file')] + [c.get('file') for c in entry.get('constructors', [])]
            if entry.get('mod_id') == OWNER or any(f in (GUEST, SYNC) for f in files):
                raise ValueError('Unreviewed state/bridge near the entry gate')
    if any(plan.get(kind) for kind in ('post_build',)):
        raise ValueError('Post-build source mutation is outside this isolated proof')

def check_sources(plan, meta, workspace, candidate):
    actual_files = set()
    for path in candidate.iterdir():
        if path.is_symlink():
            raise ValueError('Candidate symlink')
        if path.is_file() and path.suffix in ('.rs', '.toml'):
            actual_files.add(path.name)
    if actual_files != set(meta['candidate_sha256']):
        raise ValueError('Candidate source file-set drift')
    for relative, expected in meta['candidate_sha256'].items():
        if sha((candidate / relative).read_bytes()) != expected:
            raise ValueError('Candidate source bytes changed: ' + relative)
    for relative, expected in meta['native_sha256'].items():
        path = workspace / relative
        if path.is_symlink() or sha(path.read_bytes()) != expected:
            raise ValueError('Official source bytes changed: ' + relative)
        if plan.get('inputs', {}).get(relative) != expected:
            raise ValueError('Plan is not bound to the official source: ' + relative)

def check(plan, workspace, candidate=None):
    candidate = ROOT.parent / 'candidate' if candidate is None else candidate
    meta = metadata()
    check_sources(plan, meta, workspace, candidate)
    check_structure(plan, meta, candidate)

def main():
    try:
        if len(sys.argv) != 3:
            raise ValueError('usage: check_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        check(json.loads(Path(sys.argv[1]).read_text()), Path(sys.argv[2]))
    except (ValueError, OSError, KeyError, TypeError) as error:
        print('Item-name candidate composition rejected: ' + str(error), file=sys.stderr)
        return 1
    print('Item-name candidate sources and exclusive composition verified; execution is separate.')
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
