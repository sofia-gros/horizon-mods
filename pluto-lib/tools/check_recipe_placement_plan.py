#!/usr/bin/env python3
"""Conservative complete-plan exclusivity for veto-only placement boundaries.

An earlier cancellable HEAD can bypass a veto, and RETURN can restore a denied
placement. Until another composition is proved, both targets have one owner.
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
NATIVE_SOURCES = {
    'crates/horizon-client/src/crafting.rs': '298e65ea0417a37ccc73c29c23e25b396c369faeb9b17f0fef7f32b097c2243e',
    'crates/horizon-client/src/game/recipe_book.rs': '5e672aaa709de25a66c657c57925043e198b00a825ba42e7b7cfbcd4bb77c018',
}


def native_bundle(plan):
    return any(entry.get('mod_id') == 'pluto_lib'
               for section in ('injections', 'modules', 'states') for entry in plan.get(section, []))


def check_native(plan, workspace, sources, methods):
    """Shared strict preservation check for unattached recipe boundaries."""
    if plan.get('schema') != 6:
        raise ValueError('unreviewed native recipe plan schema')
    for section in ('injections', 'modules', 'states'):
        for entry in plan.get(section, []):
            if any(c.get('file') in sources for c in entry.get('constructors', [])):
                raise ValueError('state constructor changes a preserved native recipe source')
            if (entry.get('mod_id') == MOD_ID or entry.get('file') in sources
                    or entry.get('target', '').rsplit('::', 1)[-1] in methods):
                raise ValueError('native recipe boundary must remain unattached; no mixed legacy adapter')
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
