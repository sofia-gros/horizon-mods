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
def check(plan, workspace):
    meta=entries();check_order(plan,meta);verify_source(plan,workspace,meta)
def main(argv=None):
    args=sys.argv[1:] if argv is None else argv
    try:
        if len(args)!=2: raise ValueError('usage: check_recipe_placement_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        check(json.loads(Path(args[0]).read_text()),Path(args[1]))
    except (ValueError,OSError,KeyError,TypeError,StopIteration) as error:
        print('Recipe placement composition rejected: '+str(error),file=sys.stderr);return 1
    print('Recipe placement veto ownership, native fingerprints and helper source verified.');return 0
if __name__=='__main__':raise SystemExit(main())
