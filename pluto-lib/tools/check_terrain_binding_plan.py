#!/usr/bin/env python3
"""Require exact preparation source and exclusive resource-assembly hook.

This guard is for the COMPLETE installed plan, not a standalone proof that the
native renderer, loader, tessellator or textures were executed.
"""
import hashlib
import json
from pathlib import Path
import sys
ROOT=Path(__file__).resolve().parents[1]
NATIVE=ROOT/'mixins/pluto_native_creative'
def metadata():return json.loads((ROOT/'tools/terrain-binding-source.json').read_text())

def reviewed_ids(identifier):
    # Accept only the component spelling and its exact deterministic bundle ID.
    return {identifier, 'pluto_native_creative-' + identifier}

def check(plan,meta):
    hooks=plan.get('injections',[])
    selected=[h for h in hooks if h.get('target')==meta['target']
        or (h.get('file')==meta['location']['file'] and h.get('node')==meta['location']['node'])
        or h.get('function_fingerprint')==meta['function_fingerprint']]
    if len(selected)!=1:raise ValueError('Resource assembly requires exactly the reviewed preparation HEAD')
    hook=selected[0]
    if hook.get('mod_id')!='pluto_native_creative' or hook.get('injection_id') not in reviewed_ids(meta['injection_id']):
        raise ValueError('Unexpected assembly hook owner')
    if hook.get('kind')!='HEAD' or hook.get('cancellable'):raise ValueError('Preparation must not replace or cancel native assembly')
    if hook.get('function_fingerprint')!=meta['function_fingerprint'] or any(hook.get(k)!=v for k,v in meta['location'].items()):
        raise ValueError('Assembly function changed')
    if hook.get('bindings')!=meta['bindings']:raise ValueError('Prepared resource owners changed')
    raw=(NATIVE/meta['helper']).read_bytes()
    if hashlib.sha256(raw).hexdigest()!=meta['helper_sha256'] or hook.get('helper_source')!=raw.decode():
        raise ValueError('Assembly helper drifted')
    for expected in meta['modules']:
        actual=[m for m in plan.get('modules',[]) if m.get('file')==expected['file'] and m.get('name')==expected['name']]
        if (len(actual)!=1 or actual[0].get('mod_id')!='pluto_native_creative'
                or actual[0].get('module_id') not in reviewed_ids(expected['module_id'])
                or any(actual[0].get(k)!=expected[k] for k in ['file','name','fingerprint'])):
            raise ValueError('Derived catalog module placement changed')
        raw=(NATIVE/expected['source']).read_bytes()
        if hashlib.sha256(raw).hexdigest()!=expected['sha256'] or actual[0].get('source')!=raw.decode():
            raise ValueError('Derived catalog module source drifted')
    for path,digest in meta['source_sha256'].items():
        if plan.get('inputs',{}).get(path)!=digest:raise ValueError(f'Plan uses unreviewed official source: {path}')
def check_source(workspace,meta):
    for path,digest in meta['source_sha256'].items():
        if hashlib.sha256((workspace/path).read_bytes()).hexdigest()!=digest:raise ValueError(f'Official source drifted: {path}')
if __name__=='__main__':
    try:
        if len(sys.argv)!=3:raise ValueError('usage: check_terrain_binding_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        meta=metadata();check(json.loads(Path(sys.argv[1]).read_text()),meta);check_source(Path(sys.argv[2]),meta)
    except (ValueError,OSError,KeyError) as e:
        print(f'Terrain binding preparation rejected: {e}',file=sys.stderr);raise SystemExit(1)
    print('Terrain binding preparation source, owners and exclusive assembly phase verified.')
