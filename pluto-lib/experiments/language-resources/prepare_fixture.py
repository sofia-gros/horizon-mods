#!/usr/bin/env python3
"""Prepare/verify a tiny actual-core fixture. Does not run Cargo or native code.

Run only in an approved bounded slot. Resolve with cargo metadata --offline,
then --verify, cargo test --locked --offline, and --verify again. The controller
must seal the full approved host tree and use the continuous capacity guard.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tomllib

ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent
NAME='pluto-language-resources-fixture'
VERSION='0.0.0'


def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()


def check_inputs(host):
    inputs=json.loads((HERE/'inputs.json').read_text())
    env={**os.environ,'GIT_OPTIONAL_LOCKS':'0'}
    head=subprocess.check_output(['git','-C',str(host),'rev-parse','HEAD'],text=True,env=env).strip()
    dirty=subprocess.check_output(['git','-C',str(host),'status','--porcelain','--untracked-files=no'],text=True,env=env)
    if head!=inputs['horizon'] or dirty: raise ValueError('host is not the approved clean official revision')
    for path,expected in inputs['module_inputs'].items():
        if digest(ROOT/path)!=expected: raise ValueError(f'changed module input: {path}')
    for path,expected in inputs['official_inputs'].items():
        if digest(host/path)!=expected: raise ValueError(f'changed official input: {path}')
    return inputs


def generated(host):
    source=ROOT/'pluto-lib/mixins/pluto_native_creative/language.rs'
    manifest=f'''[workspace]
[package]
name = "{NAME}"
version = "{VERSION}"
edition = "2021"
[dependencies]
horizon-client-core = {{ path = {json.dumps(str(host/'crates/horizon-client-core'))} }}
'''
    library='pub use horizon_client_core::{lang, text};\n'+f'#[path = {json.dumps(str(source))}]\npub mod pluto_language;\n'
    return {'Cargo.toml':manifest.encode(),'src/lib.rs':library.encode()}


def verify_lock(directory,host):
    def identity(p): return tuple(p.get(k) for k in ('name','version','source','checksum'))
    allowed={identity(p) for p in tomllib.loads((host/'Cargo.lock').read_text())['package']}
    fixture=(NAME,VERSION,None,None)
    allowed.add(fixture)
    resolved=tomllib.loads((directory/'Cargo.lock').read_text())['package']
    if any(identity(p) not in allowed for p in resolved): raise ValueError('unapproved dependency identity')
    if sum(identity(p)==fixture for p in resolved)!=1: raise ValueError('fixture lock has not been resolved')
    core=[p for p in resolved if p['name']=='horizon-client-core']
    if len(core)!=1 or core[0].get('source') is not None: raise ValueError('expected one actual local client-core')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--verify',action='store_true')
    args=parser.parse_args();host=args.host.resolve();out=args.output.resolve()
    check_inputs(host)
    files=generated(host)
    if args.verify:
        for path,data in files.items():
            if (out/path).read_bytes()!=data: raise ValueError(f'changed fixture source: {path}')
        verify_lock(out,host)
        print('Exact adapter/official inputs, fixture source and dependency identities verified; no execution claim.')
        return
    if out.exists(): raise FileExistsError(out)
    out.mkdir(parents=True);(out/'src').mkdir()
    for path,data in files.items(): (out/path).write_bytes(data)
    (out/'Cargo.lock').write_bytes((host/'Cargo.lock').read_bytes())
    print('Prepared only; metadata resolution and all 8 native cases remain unrun.')


if __name__=='__main__': main()
