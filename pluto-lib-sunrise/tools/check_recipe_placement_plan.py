#!/usr/bin/env python3
"""Conservative complete-plan exclusivity for veto-only placement boundaries.

An earlier cancellable HEAD can bypass a veto, and RETURN can restore a denied
placement. Until another composition is proved, both targets have one owner.

Owner decision, 2026-11-22: the pluto_lib bundle is now the composition, so
check_native also admits the crafting-match module and veto when they match
the reviewed bundle_attachments in crafting-match-boundaries.json exactly
(pinned identity, phase, priority, bindings, function fingerprint, packaged
helper bytes and pristine file digest). The bundle's forwarders already
install into these hooks; without them the assembly path has nothing to
feed. The placement boundaries themselves remain exclusively native and
nothing else is exempted.
"""
import hashlib
import json
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[1]
MOD_ID = 'pluto_native_creative'

def digest(value): return hashlib.sha256(value).hexdigest()
def entries(): return json.loads((ROOT / 'tools/recipe-placement-boundaries.json').read_text())['boundaries']
def check_order(plan, metadata):
    if plan.get('schema') != 6: raise ValueError('unreviewed placement plan schema')
    for meta in metadata:
        selected = [hook for hook in plan.get('injections', []) if hook.get('target') == meta['target']]
        if len(selected) != 1: raise ValueError('placement veto requires exclusive native target: ' + meta['target'])
        hook = selected[0]
        if hook.get('mod_id') != MOD_ID or hook.get('injection_id') not in (meta['injection_id'], MOD_ID+'-'+meta['injection_id']):
            raise ValueError('placement veto owner changed')
        for key in ('file','kind','cancellable','priority','bindings','function_fingerprint'):
            if hook.get(key) != meta[key]: raise ValueError('placement target/phase fingerprint drift: '+key)
        if meta['kind'] != 'HEAD' or meta['cancellable'] is not True:
            raise ValueError('placement veto must remain a cancellable HEAD')
def verify_source(plan, workspace, metadata, helpers=None):
    helpers = ROOT / 'mixins/pluto_native_creative' if helpers is None else Path(helpers)
    for meta in metadata:
        if digest((Path(workspace)/meta['file']).read_bytes()) != meta['file_sha256']:
            raise ValueError('pristine placement source drift')
        if plan.get('inputs',{}).get(meta['file']) != meta['file_sha256']:
            raise ValueError('placement plan does not bind pristine source')
        source = (helpers/meta['helper']).read_bytes()
        if digest(source) != meta['helper_sha256']: raise ValueError('placement helper drift')
        hook = next(h for h in plan['injections'] if h['target'] == meta['target'])
        if hook.get('helper_source','').encode() != source: raise ValueError('placement plan helper differs')
# The trait bundle has no placement veto provider. Preserve the fixed native
# definition and UI entry, not the retired copied bundle's optional policy.
# (The reviewed crafting-match attachments in crafting-match-boundaries.json
# are admitted by exact identity in check_native below; they are not
# placement hooks and do not change this preservation.)
NATIVE_SOURCES = {
    'crates/horizon-client/src/crafting.rs': '298e65ea0417a37ccc73c29c23e25b396c369faeb9b17f0fef7f32b097c2243e',
    'crates/horizon-client/src/game/recipe_book.rs': '5e672aaa709de25a66c657c57925043e198b00a825ba42e7b7cfbcd4bb77c018',
}


def native_bundle(plan):
    return any(entry.get('mod_id') == 'pluto_lib'
               for section in ('injections', 'modules', 'states') for entry in plan.get(section, []))


BUNDLE_MOD_ID = 'pluto_lib'


def bundle_entries():
    data = json.loads((ROOT / 'tools/crafting-match-boundaries.json').read_text())
    return data.get('bundle_attachments', [])


def _bundle_match(entry, section, meta, helpers):
    """Exact reviewed bundle attachment: pinned identity and phase fields,
    and the plan-carried bytes bound to the packaged bundle file."""
    if entry.get('mod_id') != BUNDLE_MOD_ID:
        return False
    if 'injection_id' in meta:
        if section != 'injections':
            return False
        for key in ('injection_id', 'target', 'file', 'kind', 'cancellable',
                    'priority', 'bindings', 'function_fingerprint'):
            if entry.get(key) != meta[key]:
                return False
        packaged = (Path(helpers) / meta['helper']).read_bytes()
        return (digest(packaged) == meta['helper_sha256']
                and entry.get('helper_source', '').encode() == packaged)
    if section != 'modules':
        return False
    for key in ('module_id', 'name', 'file', 'fingerprint'):
        if entry.get(key) != meta[key]:
            return False
    if entry.get('visibility', 'pub(crate)') != meta['visibility']:
        return False
    packaged = (Path(helpers) / meta['source_file']).read_bytes()
    return (digest(packaged) == meta['source_sha256']
            and entry.get('source', '').encode() == packaged)


def check_native(plan, workspace, sources, methods, reviewed=None, helpers=None):
    """Shared strict preservation check for unattached recipe boundaries.

    An entry that trips the preservation test is admitted only when it
    matches a reviewed bundle attachment exactly (see bundle_entries); the
    return value counts those admissions per attachment identity so the
    owning guard can require them once the composition is present.
    """
    if plan.get('schema') != 6:
        raise ValueError('unreviewed native recipe plan schema')
    reviewed = bundle_entries() if reviewed is None else reviewed
    helpers = ROOT / 'mod' if helpers is None else Path(helpers)
    matched = {}
    for section in ('injections', 'modules', 'states'):
        for entry in plan.get(section, []):
            if any(c.get('file') in sources for c in entry.get('constructors', [])):
                raise ValueError('state constructor changes a preserved native recipe source')
            if (entry.get('mod_id') == MOD_ID or entry.get('file') in sources
                    or entry.get('target', '').rsplit('::', 1)[-1] in methods):
                hit = next((meta for meta in reviewed
                            if _bundle_match(entry, section, meta, helpers)), None)
                if hit is None:
                    raise ValueError('native recipe boundary must remain unattached; no mixed legacy adapter')
                key = hit.get('injection_id', hit.get('module_id'))
                matched[key] = matched.get(key, 0) + 1
    root = Path(workspace)
    for name, expected in sources.items():
        source = root / name
        if source.is_symlink() or not source.is_file():
            raise ValueError('native recipe source must be a regular file')
        source.resolve().relative_to(root.resolve())
        if plan.get('inputs', {}).get(name) != expected:
            raise ValueError('native recipe plan source differs')
        if digest(source.read_bytes()) != expected:
            raise ValueError('native recipe workspace source differs')
    return matched


def check(plan, workspace):
    if native_bundle(plan):
        check_native(plan, workspace, NATIVE_SOURCES, {'placement', 'place_recipe'})
        return
    meta=entries();check_order(plan,meta);verify_source(plan,workspace,meta)
def main(argv=None):
    args=sys.argv[1:] if argv is None else argv
    try:
        if len(args)!=2: raise ValueError('usage: check_recipe_placement_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        check(json.loads(Path(args[0]).read_text()),Path(args[1]))
    except (ValueError,OSError,KeyError,TypeError,StopIteration) as error:
        print('Recipe placement composition rejected: '+str(error),file=sys.stderr);return 1
    print('Recipe placement boundary verified; native preservation supplies no custom veto API.');return 0
if __name__=='__main__':raise SystemExit(main())
