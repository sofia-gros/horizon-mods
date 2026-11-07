"""Thin bounded wrapper around the existing ordinary Windows BuildOnly entry.

Source candidate. No operation is admitted until exact inputs are frozen/released.
"""
from contextlib import contextmanager
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import sys
import time
import tomllib

HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'tconstruct/tools'))
import prepare
import play_windows_latest as launch

def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
base=load('build_owned_phase',HERE.parent/'windows-source-ci/run.py')
status=load('build_child_status',HERE.parent/'windows-formatter-ci/child_status.py')
resource=load('build_resource_observer',HERE.parent/'windows-formatter-ci/rustc_runtime.py')
validation=load('build_strict_tests',HERE.parent/'windows-source-ci/validate.py')
BRANCH='checkpoint/windows-default-build-ci-20261107'
RESERVE=4*1024**3;INITIAL_WORK_RESERVE=64*1024**3;RAM_FLOOR=1024**3
PREPARATION_DEADLINE=None
BUDGETS={'pure':(120,128*1024),'prepare':(1200,32*1024**2),'build':(5400,32*1024**2),'inspect':(60,1024**2)}


@contextmanager
def bounded_resources(receipt,paths):
    old=base.disk_guard;stats=receipt.setdefault('resources',{'filesystems':{},'memory_samples':0})
    def check(extra):
        for p in {*paths,*extra}:
            u=shutil.disk_usage(p);key=str(Path(p).anchor)
            row=stats['filesystems'].setdefault(key,{'total_bytes':u.total,'minimum_free_bytes':u.free})
            row['minimum_free_bytes']=min(row['minimum_free_bytes'],u.free)
            if u.free<base.FLOOR+RESERVE:raise RuntimeError('Disk floor plus stop reserve reached')
        v=resource.memory();stats['memory_samples']+=1
        stats.setdefault('memory_first',v);stats['memory_last']=v
        stats['minimum_available_physical_bytes']=min(stats.get('minimum_available_physical_bytes',v['available_physical']),v['available_physical'])
        if v['available_physical']<RAM_FLOOR:raise RuntimeError('Available physical RAM floor reached')
    base.disk_guard=check
    try:check([]);yield
    finally:base.disk_guard=old


def phase(name,argv,kind,work,env,receipt):
    old=(base.TIMEOUT,base.LOG_LIMIT)
    base.TIMEOUT,base.LOG_LIMIT=BUDGETS[kind]
    if PREPARATION_DEADLINE is not None:
        base.TIMEOUT=min(base.TIMEOUT,max(0,PREPARATION_DEADLINE-time.monotonic()))
        if base.TIMEOUT<=0:
            base.TIMEOUT,base.LOG_LIMIT=old
            raise RuntimeError('Total preparation deadline')
    try:return status.phase(base,name,list(map(str,argv)),work,env,receipt)
    finally:base.TIMEOUT,base.LOG_LIMIT=old


def decode_json(raw):
    def pairs(items):
        d={}
        for k,v in items:
            if k in d:raise ValueError('Duplicate receipt field')
            d[k]=v
        return d
    return json.loads(raw.decode('utf-8'),object_pairs_hook=pairs,
                      parse_constant=lambda s:(_ for _ in ()).throw(ValueError('Nonfinite JSON')))


def check_config(c):
    if c.get('ready') is not True or not re.fullmatch('[0-9a-f]{40}',c.get('integration_head') or ''):
        raise ValueError('Build source is not frozen for execution')
    if c.get('official')!={'horizon':launch.HORIZON,'horizon-mixins':launch.MIXINS}:raise ValueError('Official pins differ')
    if c.get('branch')!=BRANCH or c.get('toolchain')!=prepare.TOOLCHAIN:raise ValueError('Fixed build contract differs')


def verify_source(c):
    before=base.snapshot(ROOT)
    for p,h in c['source_sha256'].items():
        if before.get(p)!=h:raise ValueError('Build source pin differs: '+p)
    return before


def clean_env(work):
    original={k:v for k,v in os.environ.items() if k.upper() not in {'PYTHONPATH','PYTHONHOME'} and not k.upper().startswith('HORIZON_')}
    env=prepare.environment(original,work)
    # Explicit bootstrap Git configuration is process-only; no user settings change.
    env.update(GIT_CONFIG_COUNT='3',GIT_CONFIG_KEY_0='core.autocrlf',GIT_CONFIG_VALUE_0='false',
               GIT_CONFIG_KEY_1='core.hooksPath',GIT_CONFIG_VALUE_1=str(work/'no-hooks'),
               GIT_CONFIG_KEY_2='core.attributesFile',GIT_CONFIG_VALUE_2=os.devnull)
    return env


def git_checkout(name,revision,work,git,env,receipt):
    urls={'horizon':'https://github.com/horizon-rust/horizon.git','horizon-mixins':'https://github.com/horizon-rust/horizon-mixins.git','ordinary':'https://github.com/sofia-gros/horizon-mods.git'}
    if name not in urls:raise ValueError('Unknown source checkout')
    destination=work/name;destination.mkdir()
    commands=[['init',destination],['-C',destination,'remote','add','origin',urls[name]],
              ['-C',destination,'fetch','--depth','1','origin',revision],['-C',destination,'checkout','--detach',revision]]
    for i,args in enumerate(commands):phase(name+'-git-'+str(i),[git,*args],'prepare',work,env,receipt)
    return destination


def verify_lock_diff(original,updated,plan):
    def rows(p):
        value=tomllib.loads(launch.read(p).decode('utf-8'))
        result={}
        for row in value.get('package',[]):
            key=(row['name'],row['version'],row.get('source'))
            if key in result:raise ValueError('Duplicate lock package')
            result[key]=row
        return result
    before,after=rows(original),rows(updated)
    # Official apply performs its authoritative native::refuse_new_packages check.
    # Independently retain and require all original package identities/checksums.
    if not set(before)<=set(after):raise ValueError('Applied lock lost or changed an official package')
    for key,row in before.items():
        if row.get('checksum')!=after[key].get('checksum'):raise ValueError('Applied lock checksum changed')
    allowed=decode_json(launch.guard_format.bounded_file(plan,16*1024**2)).get('crates',[])
    identities={(x['package'],x['version'],None) for x in allowed}
    if len(identities)!=len(allowed) or set(after)-set(before)!=identities-set(before):raise ValueError('Applied local packages differ from accepted plan')
    added=[{'name':k[0],'version':k[1],'source':k[2]} for k in sorted(set(after)-set(before),key=str)]
    if any(x['source'] is not None for x in added):raise ValueError('Applied lock introduced a nonlocal package')
    return {'before':prepare.record(original,8*1024**2),'after':prepare.record(updated,8*1024**2),
            'added_local_packages':added,'official_apply_lock_validation':True}


def linked_result(raw,home):
    lines=raw.decode('utf-8').splitlines()
    if not lines:raise ValueError('Missing ordinary link result')
    result=decode_json(lines[-1].encode('utf-8'))
    if set(result)!={'scope','game_started','resources_prepared','receipt','executable','sha256'}:
        raise ValueError('Ordinary result schema differs')
    if result['scope']!='guarded-windows-link-only' or result['game_started'] is not False or result['resources_prepared'] is not False:
        raise ValueError('Ordinary link scope differs')
    ready=Path(result['receipt']);exe=Path(result['executable'])
    root=home/'builds-latest-gnu'
    if ready.name!='.linked.json' or ready.parent!=exe.parent or exe.name!='horizon-client.exe' or ready.parent.parent!=root:
        raise ValueError('Ordinary output path differs')
    launch.directory(ready.parent)
    value=decode_json(launch.guard_format.bounded_file(ready,4*1024**2))
    if value.get('outputs')!={'game':result['sha256']} or prepare.record(exe)['sha256']!=result['sha256']:
        raise ValueError('Ordinary executable receipt mismatch')
    if value.get('inputs',{}).get('mode')!='link-only' or value['inputs'].get('resources') is not None:
        raise ValueError('Ordinary input mode differs')
    return result,value


def observe_float_repair(home, receipt, formatter=None):
    """Retain the one small stage receipt even when the same build phase fails."""
    observation={'status':'absent','scope':'Generated-file receipt and byte inverse; not Rust AST or build proof'}
    receipt['float_range_repair']=observation
    receipt['rust_syntax']={'status':'absent','scope':'Syntax receipt only; no type/link proof'}
    try:
        builds=Path(home)/'builds-latest-gnu'
        if not builds.exists():
            if builds.is_symlink():raise ValueError('Linked build root')
            return
        launch.directory(builds)
        # A fresh driver home can contain only the one ordinary attempt.
        entries=[]
        for entry in builds.iterdir():
            entries.append(entry)
            if len(entries)>1:raise ValueError('Multiple ordinary build roots')
        if not entries:return
        entry=entries[0]
        if not re.fullmatch('[0-9a-f]{64}',entry.name):raise ValueError('Unexpected ordinary build key')
        launch.directory(entry)
        stage=entry/'stage';path=stage/'float-range-repair-receipt.json'
        if not stage.exists():
            if stage.is_symlink():raise ValueError('Linked stage')
            return
        launch.directory(stage)
        if not path.exists():
            if path.is_symlink():raise ValueError('Linked repair receipt')
            return
        raw=launch.guard_format.bounded_file(path,8192);value=decode_json(raw)
        json.dumps(value,allow_nan=False) # reject nested exponent overflow too
        fixed={'schema':1,'kind':'known-float-range-token-boundary',
          'target':'crates/horizon-world/src/worldgen/structure/start.rs',
          'horizon':launch.HORIZON,'mixins':launch.MIXINS,'inserted_bytes':1,
          'inverse_verified':True,'other_stage_entries_unchanged':True,
          'original_and_locks_unchanged':True,'ast_verified':False,
          'semantic_change':False,'repair_applied':True}
        if not isinstance(value,dict) or any(type(value.get(k)) is not type(v) or value[k]!=v for k,v in fixed.items()):
            raise ValueError('Repair receipt identity/status differs')
        for name in ('before_sha256','after_sha256','plan_sha256','stage_before_seal_sha256','stage_after_seal_sha256'):
            if not isinstance(value.get(name),str) or not re.fullmatch('[0-9a-f]{64}',value[name]):raise ValueError('Repair digest differs')
        for name in ('before_bytes','after_bytes','offset'):
            if type(value.get(name)) is not int or value[name]<0:raise ValueError('Repair length/offset differs')
        data=launch.guard_format.bounded_file(stage/'modded'/fixed['target'],64*1024**2)
        at=value['offset']
        if (len(data)!=value['after_bytes'] or value['after_bytes']!=value['before_bytes']+1
            or at>=len(data) or data[at:at+1]!=b' ' or base.sha(data)!=value['after_sha256']
            or base.sha(data[:at]+data[at+1:])!=value['before_sha256']):raise ValueError('Repaired file/inverse differs')
        if prepare.record(stage/'complete-plan.json',16*1024**2)['sha256']!=value['plan_sha256']:
            raise ValueError('Repair plan differs')
        observation.update(status='observed',path=str(path),bytes=len(raw),sha256=base.sha(raw),result=value)
        observe_rust_syntax(stage,value,receipt,formatter)
    except Exception as error:
        # Preserve the original failed build exception; this is separate evidence.
        observation.update(status='rejected',error=(type(error).__name__+': '+str(error))[:512])


def observe_rust_syntax(stage, repair, receipt, formatter):
    observation=receipt['rust_syntax']
    try:
        path=stage/'rust-syntax-receipt.json'
        if not path.exists():
            if path.is_symlink():raise ValueError('Linked syntax receipt')
            return
        raw=launch.guard_format.bounded_file(path,16384);value=decode_json(raw)
        json.dumps(value,allow_nan=False)
        fixed={'schema':1,'kind':'rustfmt-stdin-syntax','syntax_only':True,'type_checked':False,
          'target':repair['target'],'input_sha256':repair['after_sha256'],'input_bytes':repair['after_bytes'],
          'edition':'2024','stdin_nonrecursive':True,'emit':'stdout','writes_source':False,'timeout_seconds':30}
        if not isinstance(value,dict) or any(type(value.get(k)) is not type(v) or value[k]!=v for k,v in fixed.items()):
            raise ValueError('Syntax receipt identity/input differs')
        if type(value.get('passed')) is not bool or value.get('formatter_sha256')!=receipt['gates']['formatter']['executable_sha256']:
            raise ValueError('Syntax formatter/status differs')
        if formatter is None or Path(value.get('formatter',''))!=Path(formatter):raise ValueError('Syntax formatter path differs')
        if value['passed']:
            if (type(value.get('returncode')) is not int or value['returncode']!=0 or type(value.get('stderr_bytes')) is not int or value['stderr_bytes']!=0
                or value.get('source_unchanged') is not True or value.get('formatter_unchanged') is not True or 'error' in value):
                raise ValueError('Syntax success evidence differs')
        elif not isinstance(value.get('error'),str):raise ValueError('Missing syntax failure reason')
        observation.update(status='observed',bytes=len(raw),sha256=base.sha(raw),result=value)
    except Exception as error:
        observation.update(status='rejected',error=(type(error).__name__+': '+str(error))[:512])


def execute(receipt):
    global PREPARATION_DEADLINE
    c=decode_json((HERE/'inputs.json').read_bytes());check_config(c)
    if os.name!='nt' or sys.version_info[:2]!=(3,12):raise ValueError('Windows x64 Python3.12 only')
    event=decode_json(Path(os.environ['GITHUB_EVENT_PATH']).read_bytes())
    if (os.environ.get('GITHUB_REPOSITORY')!='sofia-gros/horizon-mods' or os.environ.get('GITHUB_EVENT_NAME')!='push'
        or os.environ.get('GITHUB_REF')!='refs/heads/'+BRANCH or event.get('repository',{}).get('private') is not False):
        raise ValueError('Exact public dedicated branch only')
    PREPARATION_DEADLINE=time.monotonic()+1200
    before=verify_source(c);receipt['source_before_sha256']=base.sha(json.dumps(before,sort_keys=True).encode())
    receipt['head']=os.environ['GITHUB_SHA'];receipt['integration_head']=c['integration_head']
    work=Path(os.environ['RUNNER_TEMP'])/('windows-build-'+receipt['head']);work.mkdir()
    if shutil.disk_usage(work).free<base.FLOOR+INITIAL_WORK_RESERVE:raise ValueError('Initial work reserve unavailable')
    for rel in ('home','home/tmp-latest-gnu','home/cargo-home-latest-gnu','no-hooks'): (work/rel).mkdir(parents=True,exist_ok=True)
    env=clean_env(work);git=shutil.which('git');rustup=shutil.which('rustup')
    if not git or not rustup:raise ValueError('Existing runner Git/rustup required; no bootstrap installer')
    tools_before={k:prepare.record(Path(v)) for k,v in {'git':git,'rustup':rustup}.items()}
    source_roots=None;toolchain_seal=None;linked=None;ordinary=None;ordinary_before=None
    read_count=0
    def read_command(argv,*,cwd,env,capture=False):
        nonlocal read_count
        if str(argv[0])!=git or capture is not True:raise ValueError('Unexpected identity subprocess')
        read_count+=1
        return phase('identity-'+str(read_count),argv,'pure',work,env,receipt).decode('utf-8').strip()
    launch.run=read_command
    with bounded_resources(receipt,[ROOT,work,Path(sys.executable).parent,Path(rustup).parent]):
      try:
        raw=phase('pure',[sys.executable,HERE/'test_runner.py','-v'],'pure',work,env,receipt)
        receipt['gates']['pure']=validation.suite(raw,c['test_names'])
        actual=phase('checkout-identity',[git,'-C',ROOT,'rev-parse','HEAD'],'pure',work,env,receipt).decode().strip()
        if actual!=receipt['head']:raise ValueError('Checkout differs from event head')
        dirty=phase('checkout-clean',[git,'-C',ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching'],'pure',work,env,receipt)
        if dirty.strip():raise ValueError('Project checkout is not clean')
        rv=prepare.rustup_version(phase('rustup-version',[rustup,'--version'],'pure',work,env,receipt),receipt)
        receipt['gates']['rustup']={'version':rv,'executable':tools_before['rustup']}
        phase('rustup-install',prepare.install_argv(rustup),'prepare',work,env,receipt)
        toolchain=work/'rustup/toolchains'/prepare.TOOLCHAIN
        installed=phase('components',[rustup,'component','list','--toolchain',prepare.TOOLCHAIN,'--installed'],'pure',work,env,receipt).decode().strip()
        receipt['gates']['components']=prepare.admit_manifest(toolchain,installed)
        receipt['gates']['archives']=decode_json(phase('archives',[sys.executable,HERE/'prepare.py','archives',work],'prepare',work,env,receipt))
        receipt['gates']['winlibs']=decode_json(phase('winlibs',[sys.executable,HERE/'prepare.py','winlibs',work],'prepare',work,env,receipt))
        receipt['gates']['formatter']=launch.guard_format.admit(toolchain,work/'rustfmt.tar.gz')
        toolchain_seal=launch.tree_digest(toolchain)
        rust=phase('rust-version',[toolchain/'bin/rustc.exe','-vV'],'pure',work,env,receipt).decode('utf-8').strip()
        launch.validate_rust(rust)
        if 'commit-hash: '+launch.guard_format.COMMIT not in rust.splitlines():raise ValueError('Rust compiler commit differs')
        cargo=phase('cargo-version',[toolchain/'bin/cargo.exe','-vV'],'pure',work,env,receipt).decode('utf-8').strip()
        if not cargo.startswith('cargo 1.99.1 '):raise ValueError('Cargo version differs')
        receipt['gates']['selected_tools']={'rust':rust,'cargo':cargo,'tree_sha256':toolchain_seal}
        ordinary=git_checkout('ordinary',c['integration_head'],work,git,env,receipt)
        ordinary_identity=read_command([git,'-C',ordinary,'rev-parse','HEAD','HEAD^{tree}'],cwd=work,env=env,capture=True).splitlines()
        if ordinary_identity!=[c['integration_head'],c['integration_tree']]:raise ValueError('Selected ordinary checkout differs')
        if read_command([git,'-C',ordinary,'status','--porcelain','--untracked-files=all','--ignored=matching'],cwd=work,env=env,capture=True):raise ValueError('Selected ordinary checkout dirty')
        ordinary_before=base.snapshot(ordinary)
        for rel,digest in c['ordinary_tools'].items():
            if ordinary_before.get(rel)!=digest:raise ValueError('Selected ordinary tool differs: '+rel)
        receipt['gates']['ordinary_source']={'head':c['integration_head'],'tree':c['integration_tree'],'files':len(ordinary_before),'seal':base.sha(json.dumps(ordinary_before,sort_keys=True).encode())}
        horizon=git_checkout('horizon',launch.HORIZON,work,git,env,receipt)
        mixins=git_checkout('horizon-mixins',launch.MIXINS,work,git,env,receipt)
        source_seals=[launch.source_identity(p,n,r,l,git,env) for p,n,r,l in [
          (horizon,'horizon',launch.HORIZON,launch.HORIZON_LOCK),(mixins,'horizon-mixins',launch.MIXINS,launch.MIXINS_LOCK)]]
        receipt['gates']['official_sources']=source_seals;source_roots=(horizon,mixins)
        online=dict(env);online['CARGO_NET_OFFLINE']='false'
        for name,path in [('horizon',horizon),('mixins',mixins)]:
            phase('fetch-'+name,[toolchain/'bin/cargo.exe','fetch','--locked','--manifest-path',path/'Cargo.toml','--target',launch.TARGET],'prepare',work,online,receipt)
        PREPARATION_DEADLINE=None
        try:
            raw=phase('build-only',[sys.executable,ordinary/'tconstruct/tools/play_windows_latest.py',
              '--horizon',horizon,'--mixins',mixins,'--toolchain',toolchain,'--home',work/'home',
              '--rustfmt-archive',work/'rustfmt.tar.gz','--build-only'],'build',work,env,receipt)
        finally:
            observe_float_repair(work/'home',receipt,toolchain/'bin/rustfmt.exe')
        if receipt['float_range_repair']['status']!='observed':raise ValueError('Successful link lacks valid repair evidence')
        if receipt['rust_syntax']['status']!='observed' or receipt['rust_syntax']['result']['passed'] is not True:raise ValueError('Successful link lacks accepted syntax evidence')
        linked,detail=linked_result(raw,work/'home');stage=Path(linked['receipt']).parent/'stage'
        if Path(receipt['float_range_repair']['path']).parent!=stage:raise ValueError('Repair and linked stage differ')
        receipt['gates']['ordinary_link']={**linked,'plan':prepare.record(stage/'complete-plan.json',16*1024**2),
          'applied_lock':verify_lock_diff(horizon/'Cargo.lock',stage/'modded/Cargo.lock',stage/'complete-plan.json')}
        receipt['gates']['imports']=decode_json(phase('imports',[sys.executable,HERE/'inspect_pe.py',linked['executable']],'inspect',work,env,receipt))
        help_env=dict(env);help_home=work/'help-home';help_home.mkdir();help_env['HORIZON_HOME']=str(help_home)
        raw=phase('help',[linked['executable'],'--home',help_home,'--game-dir',help_home,'--help'],'inspect',work,help_env,receipt)
        text=raw.decode('utf-8')
        if 'Horizon' not in text or '--resources PATH' not in text or 'Controls:' not in text:raise ValueError('Client help output differs')
        if any((help_home/p).exists() for p in ('saves','resources','options.txt')):raise ValueError('Unexpected game/asset output during help')
        receipt['gates']['help']={'bytes':len(raw),'sha256':base.sha(raw),'scope':'DLL loader and CLI help; not a rendered game'}
        if prepare.record(linked['executable'])['sha256']!=linked['sha256']:raise ValueError('Built executable changed')
        required={'pure','rustup','components','archives','winlibs','formatter','selected_tools','official_sources','ordinary_source','ordinary_link','imports','help'}
        if set(receipt['gates'])!=required or not all(p.get('job_empty') is True and p.get('process_succeeded') is True for p in receipt['phases']):raise ValueError('Missing completed build gate')
        receipt['completed']=True
      finally:
        PREPARATION_DEADLINE=None
        if base.snapshot(ROOT)!=before:raise ValueError('Project source changed')
        if ordinary_before is not None:
            if base.snapshot(ordinary)!=ordinary_before:raise ValueError('Selected ordinary source changed')
            identity=read_command([git,'-C',ordinary,'rev-parse','HEAD','HEAD^{tree}'],cwd=work,env=env,capture=True).splitlines()
            dirty=read_command([git,'-C',ordinary,'status','--porcelain','--untracked-files=all','--ignored=matching'],cwd=work,env=env,capture=True)
            if identity!=[c['integration_head'],c['integration_tree']] or dirty:raise ValueError('Selected ordinary final Git state changed')
        if {k:prepare.record(Path(v)) for k,v in {'git':git,'rustup':rustup}.items()}!=tools_before:raise ValueError('Bootstrap tools changed')
        if toolchain_seal is not None and launch.tree_digest(toolchain)!=toolchain_seal:raise ValueError('Toolchain changed')
        if source_roots is not None:
            after=[launch.source_identity(p,n,r,l,git,env) for p,n,r,l in [
              (source_roots[0],'horizon',launch.HORIZON,launch.HORIZON_LOCK),(source_roots[1],'horizon-mixins',launch.MIXINS,launch.MIXINS_LOCK)]]
            if after!=receipt['gates']['official_sources']:raise ValueError('Official source/lock changed')
        head_after=phase('checkout-final-head',[git,'-C',ROOT,'rev-parse','HEAD'],'pure',work,env,receipt).decode().strip()
        clean_after=phase('checkout-final-clean',[git,'-C',ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching'],'pure',work,env,receipt)
        if head_after!=receipt['head'] or clean_after.strip():raise ValueError('Project final Git state changed')
        base.disk_guard([work]);receipt['verified']=True


def main():
    receipt={'schema':1,'passed':False,'verified':False,'completed':False,'gates':{},'phases':[],
             'scope':'fresh ordinary guarded stage, Windows default-feature link and CLI help; no GUI/assets/game test'}
    original_run=launch.run
    try:execute(receipt)
    except Exception as error:receipt['error']=type(error).__name__+': '+str(error)
    finally:launch.run=original_run
    receipt['passed']=receipt['completed'] and receipt['verified'] and 'error' not in receipt
    print('FINAL_RECEIPT '+json.dumps(receipt,sort_keys=True),flush=True)
    return 0 if receipt['passed'] else 1

if __name__=='__main__':sys.exit(main())
