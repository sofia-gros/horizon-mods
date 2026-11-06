"""Excluded 153 API + 16 finite-policy fixture preparation. No compile or fetch.

Execution needs a dedicated outer controller. Inner Git/metadata inherit its
PGID. No workflow/trigger or constructor activation is provided by this group.
"""
import argparse,hashlib,importlib.util,json,os,stat,subprocess,tomllib
from pathlib import Path
HERE=Path(__file__).resolve().parent
GROUP='pluto-lib/experiments/crafting-stored-facts'
API='pluto-lib/mixins/pluto_native_creative/inventory-api'
PIN='41f822b659789780f58d32c63ddfb6887ea1b8bf'
LOCK='0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f'
NAME='pluto-crafting-stored-tests'
API_NAME='pluto-inventory-api'
NATIVE=('horizon-core','horizon-client-core','horizon-items','horizon-world')
RECEIPT='fixture-sources.json'
def sha(data):return hashlib.sha256(data).hexdigest()

def run(args,cwd=None):
    if os.environ.get('PLUTO_FIXTURE_CONTROLLER')!='crafting-stored-v1' or os.getpgrp()!=os.getpid():
        raise ValueError('Dedicated outer controller PGID required')
    p=subprocess.Popen(args,cwd=cwd,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=False)
    try:
        out,err=p.communicate(timeout=60)
        if p.returncode:raise ValueError(f'Command failed {p.returncode}: {args!r}\n{err[-4000:]}')
        return out
    finally:
        if p.poll() is None:p.kill()
        p.wait(timeout=2)

def tree(root,exclude_git=False):
    result={}
    for directory,dirs,files in os.walk(root,followlinks=False):
        for name in sorted(dirs+files):
            p=Path(directory)/name;rel=p.relative_to(root).as_posix();mode=p.lstat().st_mode
            if exclude_git and rel=='.git':
                if name in dirs:dirs.remove(name)
                continue
            if stat.S_ISLNK(mode):
                result[rel]={'kind':'link','target':os.readlink(p)}
                if name in dirs:dirs.remove(name)
            elif stat.S_ISDIR(mode):result[rel]={'kind':'directory'}
            elif stat.S_ISREG(mode):result[rel]={'kind':'file','mode':stat.S_IMODE(mode),'sha256':sha(p.read_bytes())}
            else:raise ValueError('Special file: '+rel)
    return result

def parents(files):
    result=set()
    for name in files:
        p=Path(name).parent
        while p!=Path('.'):
            result.add(p.as_posix());p=p.parent
    return result

def exact(root,hashes):
    actual=tree(root);directories=parents(hashes)
    if set(actual)!=set(hashes)|directories:raise ValueError('Source file-set/extra directory differs: '+str(root))
    for name,digest in hashes.items():
        if actual[name]['kind']!='file' or actual[name]['sha256']!=digest:raise ValueError('Source bytes/kind differ: '+name)
    if any(actual[n]['kind']!='directory' for n in directories):raise ValueError('Parent directory is not regular')
    return actual

def inputs():
    meta=json.loads((HERE/'inputs.json').read_text())
    if (meta['official'],meta['official_lock_sha256'])!=(PIN,LOCK):raise ValueError('Cutoff drift')
    exact(HERE,dict(meta['helper_sha256'],**{'inputs.json':sha((HERE/'inputs.json').read_bytes())}))
    old,new=meta['api_existing_tests'],meta['api_added_tests']
    if len(old)!=147 or len(new)!=6 or len(set(old+new))!=153 or meta['api_expected_tests']!=sorted(old+new):raise ValueError('API inventory drift')
    if len(meta['fixture_expected_tests'])!=16 or len(set(meta['fixture_expected_tests']))!=16:raise ValueError('Fixture inventory drift')
    return meta

def clean_official(host):
    if run(['git','-C',str(host),'config','--get','remote.origin.url']).strip() not in {'https://github.com/horizon-rust/horizon.git','https://github.com/horizon-rust/horizon'}:raise ValueError('Wrong official origin')
    if run(['git','-C',str(host),'rev-parse','HEAD']).strip()!=PIN:raise ValueError('Wrong official revision')
    if run(['git','-C',str(host),'status','--porcelain','--untracked-files=all','--ignored=matching']).strip():raise ValueError('Official source is not fully clean')
    if sha((host/'Cargo.lock').read_bytes())!=LOCK:raise ValueError('Wrong official lock')

def compose(source,host,meta):
    # Check the whole package, including uncompiled examples, and every frozen
    # source/oracle file. No source-symlink traversal or hidden Cargo config.
    base=exact(source/API,meta['api_base_sha256'])
    candidate=exact(source/GROUP,meta['candidate_sha256'])
    spec=importlib.util.spec_from_file_location('stored_renderer',source/GROUP/'render.py')
    renderer=importlib.util.module_from_spec(spec);spec.loader.exec_module(renderer)
    contract=json.loads((source/GROUP/'inputs.json').read_text());oracles=json.loads((source/GROUP/'oracles.json').read_text())
    proposed={n:(source/GROUP/n).read_text() for n in contract['candidate_sha256']}
    renderer.verify_native(contract,oracles,(host/'crates/horizon-client/src/crafting.rs').read_text(),(host/'crates/horizon-client/src/advancements.rs').read_text())
    files,render_receipt=renderer.compose(contract,oracles,proposed)
    if {n:sha(s.encode()) for n,s in files.items()}!=meta['generated_sha256']:raise ValueError('Generated finite source drift')
    api={n:(source/API/n).read_bytes() for n in meta['api_base_sha256']}
    api['src/crafting/matching.rs']=proposed['matching.rs'].encode()
    if {n:sha(data) for n,data in api.items()}!=meta['api_candidate_sha256']:raise ValueError('API overlay drift')
    return api,{n:s.encode() for n,s in files.items()},{'api':base,'candidate':candidate,'render':render_receipt}

def manifests(host):
    workspace='[workspace]\nmembers=["inventory-api","fixture"]\ndefault-members=["inventory-api","fixture"]\nresolver="3"\n'
    fixture='[package]\nname="'+NAME+'"\nversion="0.0.0"\nedition="2024"\n[lib]\npath="lib.rs"\n[dependencies]\nserde_json="1"\n'
    fixture+='pluto-inventory-api={path="../inventory-api"}\n'
    for name in ('horizon-items','horizon-client-core','horizon-world'):
        fixture+=name+'={path='+json.dumps(str(host/'crates'/name))+'}\n'
    return workspace,fixture

def file_inventory(output):
    meta=inputs()
    names={'inventory-api/'+n for n in meta['api_candidate_sha256']}|{'fixture/'+n for n in meta['generated_sha256']}|{'Cargo.toml','Cargo.lock','fixture/Cargo.toml',RECEIPT}
    actual=tree(output)
    if set(actual)!=names|parents(names):raise ValueError('Output source set differs')
    if any(actual[n]['kind']!='file' for n in names) or any(actual[n]['kind']!='directory' for n in parents(names)):raise ValueError('Output kind/link differs')
    actual.pop(RECEIPT)
    return actual

def prepare(source,host,output):
    source=source.resolve();host=host.resolve();output=output.absolute()
    if output.exists() or output.is_symlink():raise ValueError('Output must be new')
    if output.resolve().is_relative_to(source) or output.resolve().is_relative_to(host):raise ValueError('Cannot write within input source')
    meta=inputs();clean_official(host);official_before=tree(host,True)
    api,fixture,source_seal=compose(source,host,meta);workspace,manifest=manifests(host)
    output.mkdir(parents=True,exist_ok=False)
    for prefix,files in [('inventory-api',api),('fixture',fixture)]:
        for name,data in files.items():
            p=output/prefix/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data)
    (output/'Cargo.toml').write_text(workspace);(output/'fixture/Cargo.toml').write_text(manifest)
    (output/'Cargo.lock').write_bytes((host/'Cargo.lock').read_bytes())
    receipt={'official':PIN,'host':str(host),'source':str(source),'source_seal':source_seal,'official_tree':official_before,'resolved':False,'api_tests':meta['api_expected_tests'],'fixture_tests':meta['fixture_expected_tests'],'scope':'API153 and finite stored-policy16 only; no constructor/bootstrap/Game/actor/ordinary hook activation'}
    receipt['output_tree']=tree(output)
    (output/RECEIPT).write_text(json.dumps(receipt,indent=2)+'\n');verify(output)
    return receipt

def verify(output,resolved=False,permit_adapted_lock=False):
    meta=inputs();receipt=json.loads((output/RECEIPT).read_text());source=Path(receipt['source']);host=Path(receipt['host'])
    if receipt['official']!=PIN or receipt['api_tests']!=meta['api_expected_tests'] or receipt['fixture_tests']!=meta['fixture_expected_tests']:raise ValueError('Receipt identity drift')
    clean_official(host)
    if tree(host,True)!=receipt['official_tree']:raise ValueError('Official source/link/mode drift')
    api,fixture,source_seal=compose(source,host,meta)
    if source_seal!=receipt['source_seal']:raise ValueError('Mod source drift')
    current=file_inventory(output);expected=dict(receipt['output_tree'])
    if permit_adapted_lock:current.pop('Cargo.lock');expected.pop('Cargo.lock')
    if current!=expected:raise ValueError('Output source/lock/mode drift')
    workspace,manifest=manifests(host)
    if (output/'Cargo.toml').read_text()!=workspace or (output/'fixture/Cargo.toml').read_text()!=manifest:raise ValueError('Manifest drift')
    for prefix,files in [('inventory-api',api),('fixture',fixture)]:
        for name,data in files.items():
            if (output/prefix/name).read_bytes()!=data:raise ValueError('Actual generated source drift')
    if resolved and not receipt['resolved']:raise ValueError('Metadata unresolved')
    return receipt

def validate_dependencies(host,output,metadata,original,adapted):
    identity=lambda p:(p['name'],p['version'],p.get('source'),p.get('checksum'))
    allowed={identity(p) for p in original['package']};actual={identity(p) for p in adapted['package']}
    local={(NAME,'0.0.0',None,None),(API_NAME,'0.1.0',None,None)}
    if actual-allowed-local or not local<=actual or len(actual)!=len(adapted['package']):raise ValueError('Lock identity escaped official plus two exact local packages')
    expected_local={NAME:output/'fixture/Cargo.toml',API_NAME:output/'inventory-api/Cargo.toml'}
    ids=[]
    for p in metadata['packages']:
        path=Path(p['manifest_path']).resolve();ids.append(p['id'])
        if (p['name'],p['version'],p.get('source')) not in {x[:3] for x in actual}:raise ValueError('Metadata is outside lock')
        if p['name'] in expected_local:
            if path!=expected_local[p['name']] or p['source'] is not None:raise ValueError('Wrong actual local API/fixture identity')
        elif p['source'] is None and not path.is_relative_to(host/'crates'):raise ValueError('Native path escaped official root')
    if len(ids)!=len(set(ids)):raise ValueError('Duplicate package IDs')
    for name in (*NATIVE,API_NAME,NAME):
        matches=[p for p in metadata['packages'] if p['name']==name]
        path=expected_local.get(name,host/'crates'/name/'Cargo.toml')
        if len(matches)!=1 or matches[0]['source'] is not None or Path(matches[0]['manifest_path']).resolve()!=path:raise ValueError('Expected one actual package: '+name)
    member_ids={p['id'] for p in metadata['packages'] if p['name'] in expected_local}
    if set(metadata['workspace_members'])!=member_ids or len(metadata['workspace_members'])!=2:raise ValueError('Wrong workspace members')
    if metadata.get('version')!=1:raise ValueError('Metadata schema differs')
    if Path(metadata['workspace_root']).resolve()!=output.resolve():raise ValueError('Wrong workspace root')
    defaults=metadata['workspace_default_members']
    if set(defaults)!=member_ids or len(defaults)!=2:raise ValueError('Wrong workspace default members')

def resolve(output):
    output=output.resolve();receipt=verify(output)
    if receipt['resolved']:raise ValueError('Already resolved')
    host=Path(receipt['host'])
    args=['cargo','metadata','--offline','--format-version=1','--manifest-path',str(output/'Cargo.toml')]
    metadata=json.loads(run(args,cwd=output));verify(output,permit_adapted_lock=True)
    validate_dependencies(host,output,metadata,tomllib.loads((host/'Cargo.lock').read_text()),tomllib.loads((output/'Cargo.lock').read_text()))
    lock_before=(output/'Cargo.lock').read_bytes()
    locked=json.loads(run(args[:2]+['--locked']+args[2:],cwd=output))
    if locked!=metadata or (output/'Cargo.lock').read_bytes()!=lock_before:raise ValueError('Locked metadata or lock drift')
    receipt['output_tree']=file_inventory(output);receipt['resolved']=True
    receipt['metadata_packages']=[{'name':p['name'],'id':p['id'],'manifest_path':p['manifest_path'],'source':p['source']} for p in metadata['packages']]
    (output/RECEIPT).write_text(json.dumps(receipt,indent=2)+'\n');verify(output,True)
    return receipt

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--source',type=Path);p.add_argument('--official',type=Path);p.add_argument('--output',type=Path,required=True)
    mode=p.add_mutually_exclusive_group(required=True);mode.add_argument('--prepare',action='store_true');mode.add_argument('--resolve',action='store_true');mode.add_argument('--verify',action='store_true')
    a=p.parse_args()
    if a.prepare:
        if a.source is None or a.official is None:p.error('source and official are required')
        r=prepare(a.source,a.official,a.output)
    elif a.resolve:r=resolve(a.output)
    else:r=verify(a.output,True)
    print(json.dumps({'prepared':True,'resolved':r['resolved'],'api_test_count':len(r['api_tests']),'fixture_test_count':len(r['fixture_tests']),'actual_rust_executed':False},sort_keys=True))
if __name__=='__main__':main()
