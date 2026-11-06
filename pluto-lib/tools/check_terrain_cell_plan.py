#!/usr/bin/env python3
"""Fail closed on drift/composition of the bounded terrain cell adapter.

Requires the complete installed plan plus pristine official source. No GPU or
rendering-performance claim follows from this source/plan guard.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / 'mixins/pluto_native_creative'
def digest(text): return hashlib.sha256(text.encode()).hexdigest()
def metadata(): return json.loads((ROOT/'tools/terrain-cell-source.json').read_text())
def ids(raw): return {raw, 'pluto_native_creative-'+raw}
def formatted(text):
    formatter = os.environ.get('PLUTO_GUARD_RUSTFMT', 'rustfmt')
    if 'PLUTO_GUARD_RUSTFMT' in os.environ and not Path(formatter).is_absolute():
        raise ValueError('Explicit guard formatter must be an absolute path')
    return subprocess.run([formatter,'--edition','2024','--emit','stdout'],input=text,text=True,capture_output=True,check=True).stdout

def extract(text,start,end):
    if text.count(start)!=1: raise ValueError('native extraction start is missing or ambiguous')
    a=text.index(start)
    if end not in text[a:]: raise ValueError('native extraction end is missing')
    return text[a:text.index(end,a)].rstrip()

def verify_regeneration(workspace,meta):
    source=(workspace/meta['file']).read_text()
    if digest(source)!=meta['file_sha256']: raise ValueError('native mesher source drift')
    original=extract(source,meta['start'],meta['end'])
    if digest(original)!=meta['function_sha256']: raise ValueError('native mesher function drift')
    body=original.replace('    pub fn mesh_reusing(','fn original(',1).replace('&mut self,',"mesher: &mut Mesher<'_>,",1)
    body=re.sub(r'\bself\b','mesher',body)
    for change in meta['changes']:
        if body.count(change['old'])!=1: raise ValueError('reviewed cell dispatch hunk no longer matches exactly once')
        body=body.replace(change['old'],change['new'])
    host=(NATIVE/'terrain-cell-host.rs').read_text()
    actual=extract(host,'// BEGIN EXACT NATIVE BODY WITH REVIEWED DISPATCH HUNKS\n','// END EXACT NATIVE BODY WITH REVIEWED DISPATCH HUNKS').split('\n',1)[1]
    if formatted(body)!=formatted(actual): raise ValueError('copied mesher has changes outside reviewed hunks')
    face=extract(source,meta['face_start'],meta['face_end'])
    face=re.sub(r'\bself\b','mesher',face).replace('&mut tints,','tints,').replace('&mut out,','out,')
    actual=extract(host,'// BEGIN EXACT NATIVE FACE FILTER\n','// END EXACT NATIVE FACE FILTER').split('\n',1)[1]
    if formatted('fn filter() {\n'+face+'\n}')!=formatted('fn filter() {\n'+actual+'\n}'):
        raise ValueError('native face cull/pane/emission filter changed')

def check(plan,meta):
    if plan.get('schema')!=6: raise ValueError('unreviewed plan schema')
    # The copied body bypasses all interior hooks; source metadata has an empty
    # baked material name and is safe only through these pinned native methods.
    # Reserve the whole mesher file until broader composition is proved.
    selected=[h for h in plan.get('injections',[]) if h.get('file')==meta['file'] or h.get('target')==meta['target'] or h.get('function_fingerprint')==meta['function_fingerprint']]
    if len(selected)!=1: raise ValueError('terrain cell adapter requires exclusive mesher-file hooks')
    hook=selected[0]
    if hook.get('mod_id')!='pluto_native_creative' or hook.get('injection_id') not in ids(meta['injection_id']): raise ValueError('terrain cell hook owner changed')
    for key in ['target','function_fingerprint','bindings','priority','kind','cancellable']:
        if hook.get(key)!=meta[key]: raise ValueError('terrain cell hook contract changed: '+key)
    if any(hook.get(k)!=v for k,v in meta['location'].items()): raise ValueError('native target resolution changed')
    helper=(NATIVE/'terrain-cell-head.rs').read_text()
    if hook.get('helper_source')!=helper or digest(helper)!=meta['helper_sha256']: raise ValueError('terrain cell helper drift')
    for entry in meta['modules']:
        modules=[m for m in plan.get('modules',[]) if m.get('file')==entry['file'] and m.get('name')==entry['name']]
        if len(modules)!=1: raise ValueError('missing or duplicate terrain cell module')
        module=modules[0]
        if module.get('mod_id')!='pluto_native_creative' or module.get('module_id') not in ids(entry['module_id']): raise ValueError('terrain cell module owner drift')
        if module.get('fingerprint')!=entry['fingerprint']: raise ValueError('terrain cell module placement drift')
        text=(NATIVE/entry['source']).read_text()
        if digest(text)!=entry['sha256'] or module.get('source')!=text: raise ValueError('terrain cell module source drift')
    for path,sha in meta['source_sha256'].items():
        if plan.get('inputs',{}).get(path)!=sha: raise ValueError('terrain cell plan input drift: '+path)

def check_source(workspace,meta):
    for path,sha in meta['source_sha256'].items():
        if hashlib.sha256((workspace/path).read_bytes()).hexdigest()!=sha: raise ValueError('official source drift: '+path)
    verify_regeneration(workspace,meta)

if __name__=='__main__':
    try:
        if len(sys.argv)!=3: raise ValueError('usage: check_terrain_cell_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE')
        meta=metadata();check(json.loads(Path(sys.argv[1]).read_text()),meta);check_source(Path(sys.argv[2]),meta)
    except (ValueError,OSError,KeyError,subprocess.CalledProcessError) as error:
        print('Terrain cell adapter rejected: '+str(error),file=sys.stderr);raise SystemExit(1)
    print('Terrain cell official body, cull/emission filter and exclusive plan verified.')
