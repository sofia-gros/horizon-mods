"""Exact helper14 serde fixture. No host editing or planner invocation."""
import argparse,hashlib,json,os,subprocess,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
REPAIR=ROOT/'pluto-lib/experiments/language-overlay-repair'
PLACEMENT=ROOT/'pluto-lib/experiments/language-overlay-placement'
NAME='pluto-language-repair-helper-fixture'
PIN='f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
URL='https://github.com/horizon-rust/horizon.git'
LOCK='0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f'
def sha(b):return hashlib.sha256(b).hexdigest()
def require(ok,message):
    if not ok:raise ValueError(message)
def git(host,*args):
    require(os.environ.get('PLUTO_FIXTURE_CONTROLLER')=='language-repair-helper-v1' and os.getpgrp()==os.getpid(),'Require outer controller process group')
    return subprocess.check_output(['git','-C',str(host),*args],text=True,env={**os.environ,'GIT_OPTIONAL_LOCKS':'0'}).strip()
def inputs(host):
    require(git(host,'remote','get-url','origin')==URL and git(host,'rev-parse','HEAD')==PIN,'Wrong official identity')
    require(not git(host,'status','--porcelain','--untracked-files=all','--ignored=matching'),'Dirty official checkout')
    require(sha((host/'Cargo.lock').read_bytes())==LOCK,'Wrong official lock')
    contract=json.loads((REPAIR/'inputs.json').read_bytes())
    for p,h in contract['official_sources'].items():require(sha((host/p).read_bytes())==h,'Changed official input: '+p)
    require(sha((REPAIR/'merge.rs').read_bytes())==contract['helper_sha256'],'Changed helper')
    config=json.loads((PLACEMENT/'inputs.json').read_bytes())
    for p,h in config['fixture_sha256'].items():require(sha((PLACEMENT/p).read_bytes())==h,'Changed fixture: '+p)
    require(config['merge_sha256']==contract['helper_sha256'],'Two helper owners differ')
    hostdeps=tomllib.loads((host/'crates/horizon-client-lib/Cargo.toml').read_text())['dependencies']
    require(tomllib.loads((PLACEMENT/'Cargo.toml.in').read_text())['dependencies']=={k:hostdeps[k] for k in ['serde','serde_json']},'Official serde requirements changed')
    return config
def generated():
    return {'Cargo.toml':(PLACEMENT/'Cargo.toml.in').read_bytes(),'fixture.rs':(PLACEMENT/'fixture.rs').read_bytes(),'merge.rs':(REPAIR/'merge.rs').read_bytes()}
def identity(p):return tuple(p.get(k) for k in ['name','version','source','checksum'])
def validate_dependencies(host,out,metadata,lock):
    allowed={identity(p) for p in tomllib.loads((host/'Cargo.lock').read_text())['package']}
    actual=[identity(p) for p in lock['package']];own=(NAME,'0.0.0',None,None)
    require(len(actual)==len(set(actual)) and actual.count(own)==1 and set(actual)<=allowed|{own},'Dependency lock identity escaped official lock')
    locals_=[p for p in metadata['packages'] if p.get('source') is None]
    require(len(locals_)==1 and locals_[0]['name']==NAME and Path(locals_[0]['manifest_path']).resolve()==out/'Cargo.toml','Unexpected local/native substitute')
    require(metadata['workspace_members']==[locals_[0]['id']],'Wrong fixture workspace')
    packages=metadata['packages'];require(len({p['id'] for p in packages})==len(packages),'Duplicate metadata ID')
    for p in packages:require((p['name'],p['version'],p.get('source')) in {x[:3] for x in actual},'Metadata escaped lock')
    for name in ['serde','serde_json']:
        matches=[p for p in packages if p['name']==name]
        require(len(matches)==1 and matches[0].get('source')=='registry+https://github.com/rust-lang/crates.io-index','Expected actual locked serde package')
    require(not any(p['name'].startswith(('horizon-','pluto-')) and p['name']!=NAME for p in packages),'Helper proof cannot claim native dependencies')
def files(out):
    require(out.is_dir() and not out.is_symlink(),'Invalid fixture directory')
    rows={}
    for p in out.iterdir():
        require(p.is_file() and not p.is_symlink(),'Unexpected fixture entry kind')
        rows[p.name]=sha(p.read_bytes())
    return rows
def verify(host,out):
    config=inputs(host)
    require(set(files(out))=={'Cargo.toml','fixture.rs','merge.rs','Cargo.lock','metadata.json','fixture-sources.json'},'Fixture file-set changed')
    for p,b in generated().items():require((out/p).read_bytes()==b,'Generated source changed')
    evidence=json.loads((out/'fixture-sources.json').read_bytes())
    require(evidence['expected_names']==config['expected_tests'] and evidence['official']==str(host),'Fixture scope changed')
    require(sha((out/'Cargo.lock').read_bytes())==evidence['lock_sha256'] and sha((out/'metadata.json').read_bytes())==evidence['metadata_sha256'],'Resolved lock/metadata changed')
    validate_dependencies(host,out,json.loads((out/'metadata.json').read_bytes()),tomllib.loads((out/'Cargo.lock').read_text()))
    return evidence
def main():
    parser=argparse.ArgumentParser();parser.add_argument('--official',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    group=parser.add_mutually_exclusive_group();group.add_argument('--resolve',action='store_true');group.add_argument('--verify',action='store_true')
    args=parser.parse_args();host=args.official.resolve();out=args.output.absolute();config=inputs(host)
    require(not out.resolve().is_relative_to(host) and not out.resolve().is_relative_to(ROOT),'Output may not modify source')
    if args.verify:verify(host,out);print('Exact helper fixture verified.');return
    if args.resolve:
        require(set(files(out))=={'Cargo.toml','fixture.rs','merge.rs','Cargo.lock'},'Prepared file-set changed')
        for p,b in generated().items():require((out/p).read_bytes()==b,'Prepared source changed')
        require((out/'Cargo.lock').read_bytes()==(host/'Cargo.lock').read_bytes(),'Unseeded official lock')
        raw=subprocess.check_output(['cargo','metadata','--offline','--format-version','1','--manifest-path',str(out/'Cargo.toml')])
        metadata=json.loads(raw);validate_dependencies(host,out,metadata,tomllib.loads((out/'Cargo.lock').read_text()))
        (out/'metadata.json').write_bytes(raw)
        evidence={'official':str(host),'revision':PIN,'expected_names':config['expected_tests'],'lock_sha256':sha((out/'Cargo.lock').read_bytes()),'metadata_sha256':sha(raw),
                  'packages':[{'name':p['name'],'version':p['version'],'manifest_path':p['manifest_path'],'source':p.get('source')} for p in metadata['packages']]}
        (out/'fixture-sources.json').write_text(json.dumps(evidence,sort_keys=True)+'\n');verify(host,out);return
    require(not out.exists() and not out.is_symlink(),'Output already exists');out.mkdir()
    for p,b in generated().items():(out/p).write_bytes(b)
    (out/'Cargo.lock').write_bytes((host/'Cargo.lock').read_bytes());print('Exact helper and official lock prepared; Rust unexecuted.')
if __name__=='__main__':main()
