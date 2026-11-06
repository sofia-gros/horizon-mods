#!/usr/bin/env python3
"""Prepare a bounded binding fixture: real official registry + modeled renderer.

No native rendering or gameplay is exercised. Production binding/cache/upper
projection sources and the actual Pluto API are included without edits.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib
from prepare_render_fixture import check_identities, clean_official
from prepare_fluid_query_fixture import api_inputs

IDENTITY=('pluto-terrain-bindings','0.0.0',None,None)
API=('pluto-api','0.1.0',None,None)
def digest(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def check_lock(host,output):
    locked=tomllib.loads((host/'Cargo.lock').read_text())
    locked['package'].append({'name':API[0],'version':API[1]})
    check_identities(locked,tomllib.loads((output/'Cargo.lock').read_text()),IDENTITY)
def verify(output):
    evidence=json.loads((output/'fixture-sources.json').read_text());host=Path(evidence['horizon_source'])
    clean_official(host,evidence['horizon_revision'])
    if digest(host/'Cargo.lock')!=evidence['official_lock_sha256']: raise ValueError('Official lock changed')
    if api_inputs(Path(evidence['api_source']))!=evidence['api_sources_sha256']: raise ValueError('Actual API changed')
    for path,expected in evidence['sources_sha256'].items():
        if digest(output/path)!=expected: raise ValueError(f'Fixture source changed: {path}')
    check_lock(host,output)
    return evidence

def prepare(host,output):
    root=Path(__file__).resolve().parents[1];native=root/'mixins/pluto_native_creative'
    if 'prepare_with_materials' in (native/'terrain-binding-host.rs').read_text():
        raise ValueError('The legacy modeled renderer shell cannot validate native material templates; use an actual renderer fixture. No output was created.')
    pin=tomllib.loads((native/'horizon-mixin.toml').read_text())['compatibility']['horizon_revision']
    host=host.resolve();output=output.resolve();clean_official(host,pin)
    api=(native/'api').resolve();api_hashes=api_inputs(api)
    if output.exists(): raise ValueError('Output must not exist')
    output.mkdir(parents=True)
    sources={name:native/name for name in ['terrain-bindings.rs','terrain-binding-host.rs','fluid-render-prepare.rs']}
    sources.update({'lib.rs':root/'tests/native/terrain-bindings/shell.rs','tests.rs':root/'tests/native/terrain-bindings/tests.rs'})
    for name,source in sources.items():shutil.copyfile(source,output/name)
    locked=tomllib.loads((host/'Cargo.lock').read_text());logs=[p['version'] for p in locked['package'] if p['name']=='log']
    if len(logs)!=1:raise ValueError('Require exactly one official log package')
    manifest='[package]\nname="pluto-terrain-bindings"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lib]\npath="lib.rs"\n[dependencies]\n'
    manifest+='horizon-world={path='+json.dumps(str(host/'crates/horizon-world'))+'}\n'
    manifest+='pluto-api={path='+json.dumps(str(api))+'}\nlog="='+logs[0]+'"\n[patch.crates-io]\n'
    for name,entry in tomllib.loads((host/'Cargo.toml').read_text()).get('patch',{}).get('crates-io',{}).items():
        if set(entry)!={'path'}:raise ValueError('Unsupported official patch')
        path=(host/entry['path']).resolve()
        if not path.is_relative_to(host):raise ValueError('Official patch escapes source')
        manifest+=json.dumps(name)+'={path='+json.dumps(str(path))+'}\n'
    (output/'Cargo.toml').write_text(manifest);shutil.copyfile(host/'Cargo.lock',output/'Cargo.lock')
    subprocess.run(['cargo','metadata','--offline','--manifest-path',str(output/'Cargo.toml'),'--format-version','1'],check=True,stdout=subprocess.DEVNULL)
    evidence={'horizon_source':str(host),'horizon_revision':pin,'official_lock_sha256':digest(host/'Cargo.lock'),'api_source':str(api),'api_sources_sha256':api_hashes,
        'sources_sha256':{name:digest(output/name) for name in [*sources,'Cargo.toml']},
        'scope':'Actual official BlockRegistry, BlockState and FluidState; production adapters/cache/API. Assets, RenderAssets, MeshTables and Mesher are explicitly modeled shells. No full renderer types, native assembly, tessellation, graphics or gameplay proof.'}
    (output/'fixture-sources.json').write_text(json.dumps(evidence,indent=2)+'\n');verify(output);print(output)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--horizon-source',type=Path);p.add_argument('--output',type=Path,required=True);p.add_argument('--verify-lock',action='store_true');a=p.parse_args()
    if a.verify_lock:verify(a.output);print('Official lock and exact source inputs verified')
    elif a.horizon_source is None:p.error('--horizon-source is required')
    else:prepare(a.horizon_source,a.output)
