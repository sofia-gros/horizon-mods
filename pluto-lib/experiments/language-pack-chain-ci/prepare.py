"""Prepare/resolve/verify only the exact actual public packs/mod_content::build/Language baseline fixture.
Execution requires the separately granted CI controller; no host sources change.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tomllib

ROOT=Path(__file__).resolve().parents[3]
SOURCE=ROOT/'pluto-lib/experiments/language-pack-chain'
NAME='pluto-language-pack-chain-fixture'
VERSION='0.0.0'
OFFICIAL='https://github.com/horizon-rust/horizon.git'
PATCH_PATHS={'wgpu-hal':'third_party/wgpu-hal','naga':'third_party/naga','libmimalloc-sys':'third_party/libmimalloc-sys'}


def sha(data):return hashlib.sha256(data).hexdigest()
def git(root,*args):
    return subprocess.check_output(['git','-C',str(root),*args],text=True,
        env={**os.environ,'GIT_OPTIONAL_LOCKS':'0'}).strip()
def inputs(host):
    config=json.loads((SOURCE/'inputs.json').read_text())
    if (git(host,'rev-parse','HEAD')!=config['horizon'] or
        git(host,'remote','get-url','origin')!=OFFICIAL or
        git(host,'status','--porcelain','--untracked-files=all','--ignored=matching')):
        raise ValueError('Require approved clean official checkout')
    for entry in config['official_inputs']:
        if sha((host/entry['path']).read_bytes())!=entry['sha256']:
            raise ValueError('Official source mismatch: '+entry['path'])
    for path,digest in config['fixture_inputs'].items():
        if sha((SOURCE/path).read_bytes())!=digest:raise ValueError('Fixture input mismatch: '+path)
    official_patch_paths(host)
    return config

def official_patch_paths(host):
    actual=tomllib.loads((host/'Cargo.toml').read_text()).get('patch',{}).get('crates-io',{})
    expected={name:{'path':path} for name,path in PATCH_PATHS.items()}
    if actual!=expected:raise ValueError('Official patch declarations changed')
    paths={name:(host/path/'Cargo.toml').resolve() for name,path in PATCH_PATHS.items()}
    for name,path in paths.items():
        if path!=host/PATCH_PATHS[name]/'Cargo.toml':raise ValueError('Official patch path redirected')
        package=tomllib.loads(path.read_text())['package']
        if package['name']!=name:raise ValueError('Wrong official patched package name')
    return paths

def generated(host):
    # JSON escaping is valid for TOML basic string contents; do not interpolate
    # arbitrary unescaped filesystem strings into the dependency table.
    escaped=json.dumps(str(host),ensure_ascii=False)[1:-1]
    template=(SOURCE/'Cargo.toml.in').read_text()
    patches=official_patch_paths(host)
    patch_text='\n[patch.crates-io]\n'+''.join(name+' = { path = '+json.dumps(str(path.parent),ensure_ascii=False)+' }\n' for name,path in sorted(patches.items()))
    return {'Cargo.toml':(template.replace('@OFFICIAL@',escaped)+patch_text).encode(),
            'fixture.rs':(SOURCE/'fixture.rs').read_bytes()}

def identity(p):return tuple(p.get(k) for k in ['name','version','source','checksum'])
def verify_lock(out,host):
    official=tomllib.loads((host/'Cargo.lock').read_text())['package']
    actual=tomllib.loads((out/'Cargo.lock').read_text())['package']
    allowed={identity(p) for p in official};local=(NAME,VERSION,None,None)
    if any(identity(p) not in allowed|{local} for p in actual):
        raise ValueError('Unexpected dependency identity')
    if sum(identity(p)==local for p in actual)!=1:raise ValueError('Expected one resolved fixture')
    for name in ['horizon-mods','horizon-client-core','horizon-client-lib','horizon-client-render']:
        matches=[p for p in actual if p['name']==name]
        if len(matches)!=1 or matches[0].get('source') is not None:
            raise ValueError('Expected one actual official package: '+name)

def verify_metadata(metadata,out,host):
    patches=official_patch_paths(host)
    for package in metadata['packages']:
        if package['name'] in patches and package.get('source') is not None:
            raise ValueError('Registry substitute for official local patch: '+package['name'])
    locals_=[p for p in metadata['packages'] if p.get('source') is None]
    if len({p['name'] for p in locals_})!=len(locals_):raise ValueError('Duplicate local package name')
    fixture=[p for p in locals_ if p['name']==NAME]
    if len(fixture)!=1 or Path(fixture[0]['manifest_path']).resolve()!=out/'Cargo.toml':
        raise ValueError('Expected exact fixture package')
    for p in locals_:
        if p['name']==NAME:continue
        path=Path(p['manifest_path']).resolve()
        expected=patches.get(p['name'],(host/'crates'/p['name']/'Cargo.toml').resolve())
        if path!=expected or not (path.is_relative_to(host/'crates') or p['name'] in patches):
            raise ValueError('Substituted local dependency: '+p['name'])
    for name in ['horizon-mods','horizon-client-core','horizon-client-lib','horizon-client-render']:
        if sum(p['name']==name for p in locals_)!=1:raise ValueError('Missing actual official package: '+name)

def verify_files(out,host):
    config=inputs(host)
    for path,data in generated(host).items():
        if (out/path).read_bytes()!=data:raise ValueError('Changed generated fixture: '+path)
    verify_lock(out,host)
    evidence=json.loads((out/'fixture-sources.json').read_text())
    if evidence['expected_names']!=config['expected_test_names'] or evidence['official']!=str(host):
        raise ValueError('Fixture evidence scope drift')
    if sha((out/'Cargo.lock').read_bytes())!=evidence['lock_sha256']:raise ValueError('Resolved lock drift')
    metadata=json.loads((out/'metadata.json').read_text());verify_metadata(metadata,out,host)
    if sha((out/'metadata.json').read_bytes())!=evidence['metadata_sha256']:raise ValueError('Metadata drift')
    return evidence

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--official',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--resolve',action='store_true')
    parser.add_argument('--verify',action='store_true')
    args=parser.parse_args();host=args.official.resolve();out=args.output.resolve()
    if args.resolve and args.verify:raise ValueError('Choose resolve or verify')
    config=inputs(host)
    if args.verify:
        verify_files(out,host);print('Exact fixture, official inputs, dependency lock and metadata verified.');return
    if args.resolve:
        for path,data in generated(host).items():
            if (out/path).read_bytes()!=data:raise ValueError('Changed prepared source: '+path)
        result=subprocess.check_output(['cargo','metadata','--offline','--format-version','1','--manifest-path',str(out/'Cargo.toml')])
        metadata=json.loads(result);verify_lock(out,host);verify_metadata(metadata,out,host)
        (out/'metadata.json').write_bytes(result)
        evidence={'official':str(host),'revision':config['horizon'],'expected_names':config['expected_test_names'],
                  'lock_sha256':sha((out/'Cargo.lock').read_bytes()),'metadata_sha256':sha(result),
                  'dependency_packages':[{'name':p['name'],'manifest_path':p['manifest_path'],'source':p.get('source')} for p in metadata['packages']]}
        (out/'fixture-sources.json').write_text(json.dumps(evidence,sort_keys=True,indent=2)+'\n')
        verify_files(out,host);return
    if out.exists():raise FileExistsError(out)
    out.mkdir()
    for path,data in generated(host).items():(out/path).write_bytes(data)
    (out/'Cargo.lock').write_bytes((host/'Cargo.lock').read_bytes())
    print('Prepared exact source and official lock; native tests have not run.')

if __name__=='__main__':main()
