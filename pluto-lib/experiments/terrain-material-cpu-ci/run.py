#!/usr/bin/env python3
"""Actual official CPU mesh proof only; no GPU, real assets or consumer activation."""
import contextlib
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import re
import shlex
import shutil
import sys
import time
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
FIXTURE=HERE.parent/'terrain-material-cpu'
spec=importlib.util.spec_from_file_location('frozen_terrain_author',HERE.parent/'terrain-author-ci/run.py')
A=importlib.util.module_from_spec(spec);spec.loader.exec_module(A)
FLOOR=A.FLOOR
TEST='mesher::pluto_terrain_material_cpu::tests::real_native_mesh_differential_and_timing'
PROFILE='test; opt-level=0; debug=0; incremental=0'
RUNNER_TEST_COUNT=15
REQUIRED_GATES={'runner15','inherited_runner20','source23','fixture_source12','official_sources',
    'production_package','prepare','plan','complete_plan_guards','fixture_plan_guard','apply',
    'host_lib_bin_type','renderer_compile','native_material2','cpu_no_provider','cpu_registered'}
TARGETS={'pluto_api','pluto_inventory','horizon_client_render','horizon_client_lib','horizon_client'}
def config():return json.loads((HERE/'inputs.json').read_text())
def validate_config(c):
    expected={'horizon':{'url':'https://github.com/horizon-rust/horizon.git',
        'revision':'624c4f6265c02296fbe8a80f029dd0d987fb94c0'},
        'mixins':{'url':'https://github.com/horizon-rust/horizon-mixins.git',
        'revision':'e8cd0aa38cffef49cb2e088ef716baa107148c02'}}
    if (c['repository']!='sofia-gros/horizon-mods' or
        c['branch']!='checkpoint/plutolib-terrain-material-cpu-ci-20261106' or c['official']!=expected or
        c['official_lock_sha256']!='463913123985e344f2889e5beae65982773e6dbfacdcac7a916bcec522a5bd31' or
        c['production_head']!='43f174273cd5f1b9cf3acd1d26abab8838025510' or
        c['bundle_sha256']!='cd6de18cacbe7c40218ec817671bf3842aef3be0d586c613bb788babafbd3a76'):
        raise ValueError('Require exact official repositories, revisions and production cutoff')

def strict_seal(root):
    # Include ignored/untracked input files. Do not follow source symlinks.
    import stat
    result={}
    for path in sorted(root.rglob('*')):
        relative=path.relative_to(root)
        if relative.parts[0]=='.git':continue
        info=path.lstat();mode=info.st_mode
        if stat.S_ISDIR(mode):continue
        if stat.S_ISREG(mode):result[str(relative)]={'kind':'file','sha256':A.sha(path.read_bytes()),'mode':stat.S_IMODE(mode)}
        elif stat.S_ISLNK(mode):result[str(relative)]={'kind':'symlink','target':os.readlink(path)}
        else:raise ValueError('Special source node: '+str(path))
    return result

def assert_strict(root,expected):
    if strict_seal(root)!=expected:raise ValueError('Complete source/file/link drift: '+str(root))
def assert_checkout():
    if A.git(ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching'):
        raise ValueError('Dirty, untracked or ignored checkout')

def compiler_records(log):
    records=[]
    for line in log.splitlines():
        if 'Running `' not in line or 'rustc' not in line:continue
        command=line.split('Running `',1)[1].rsplit('`',1)[0]
        args=shlex.split(command)
        if '--crate-name' not in args:continue
        name=args[args.index('--crate-name')+1]
        if name not in TARGETS:continue
        records.append({'crate':name,'source':[x for x in args if x.endswith('.rs')],
            'test':'--test' in args,'codegen':[args[i+1] for i,x in enumerate(args[:-1]) if x=='-C'],
            'full_invocation_sha256':A.sha(line.encode()),'argument_count':len(args)})
    return records

class SilentOutput:
    # The inner frozen runner still verifies raw logs and owns teardown. Suppress
    # its full verbose print, not the subprocess log or subsequent validation.
    def write(self,value):return len(value)
    def flush(self):pass
class Runner(A.Runner):
    def __init__(self,work,env,report):super().__init__(work,env);self.report=report
    def run(self,name,command,cwd=None,timeout=2400):
        print('PHASE_START',name,json.dumps(command),flush=True)
        started=time.monotonic();error=None
        try:
            with contextlib.redirect_stdout(SilentOutput()):
                return super().run(name,command,cwd=cwd,timeout=timeout)
        except BaseException as exc:error=exc;raise
        finally:
            path=self.work/(name+'.log')
            phase={'name':name,'elapsed_seconds':round(time.monotonic()-started,3),
                'raw_sha256':None,'raw_bytes':None,'terminal':'error' if error else 'success',
                'error':None if error is None else type(error).__name__+': '+str(error),
                'required_compilers':[]}
            self.report['phases'].append(phase)
            try:
                raw=path.read_bytes() if path.exists() else b''
                text=raw.decode(errors='replace')
                phase.update(raw_sha256=A.sha(raw),raw_bytes=len(raw),required_compilers=compiler_records(text))
                selected=[line for line in text.splitlines() if line.startswith(('test ','test result:',
                    'PLUTO_TERRAIN_','Ran ','OK','Terrain ','One test-only','Exact production'))]
                rendered='\n'.join(selected)
                if len(rendered.encode())>128*1024:raise ValueError('Structured phase output exceeds bound')
                if rendered:print(rendered,flush=True)
                if error:
                    diagnostics=[]
                    for line in text.splitlines():
                        if not line.startswith('{'):continue
                        try:value=json.loads(line)
                        except json.JSONDecodeError:continue
                        message=value.get('message',{})
                        if value.get('reason')=='compiler-message' and message.get('level')=='error':
                            diagnostics.append(str(message.get('rendered') or message.get('message',''))[:4096])
                    print('FAILURE_DIAGNOSTICS','\n'.join(diagnostics)[:16384],flush=True)
                    print('FAILURE_TAIL',text[-16384:],flush=True)
            except BaseException as exc:
                phase['summary_error']=type(exc).__name__+': '+str(exc)
                phase['terminal']='error'
                raise
            finally:
                print('PHASE_RECEIPT',json.dumps(phase,sort_keys=True),flush=True)

def artifact(log,stage,target):
    found=[]
    for line in log.splitlines():
        if not line.startswith('{'):continue
        try:value=json.loads(line)
        except json.JSONDecodeError:continue
        if value.get('reason')!='compiler-artifact':continue
        t=value.get('target',{});profile=value.get('profile',{})
        if t.get('name')!='horizon_client_render' or profile.get('test') is not True:continue
        if t.get('kind')!=['lib'] or t.get('src_path')!=str(stage/'crates/horizon-client-render/src/lib.rs'):
            raise ValueError('Unexpected actual renderer artifact source')
        if profile.get('opt_level')!='0' or profile.get('debug_assertions') is not True:
            raise ValueError('Unexpected timing profile')
        p=Path(value.get('executable') or '').resolve()
        if not p.is_relative_to((target/'debug/deps').resolve()) or not p.is_file() or not os.access(p,os.X_OK):
            raise ValueError('Artifact is not the actual target executable')
        found.append(p)
    if len(found)!=1:raise ValueError('Missing or duplicate exact renderer test artifact')
    return found[0]

def validate_cpu(log,mode):
    A.validate_tests(log,[TEST])
    markers=re.findall(r'^PLUTO_TERRAIN_MODE (\S+) complete; actual native CPU mesh, original synthetic resources only$',log,re.M)
    if markers!=[mode]:raise ValueError('Missing, foreign or duplicate completed mode')
    rows=[json.loads(m) for m in re.findall(r'^PLUTO_TERRAIN_CPU (.+)$',log,re.M)]
    expected={'no-provider'} if mode=='no-provider' else {'registered-ordinary','provider-no-claim','claimed-mixed','secondary-material'}
    if len(rows)!=len(expected) or {r.get('case') for r in rows}!=expected:raise ValueError('Missing/duplicate timing cases')
    for row in rows:
        if (row.get('profile_label')!=PROFILE or row.get('debug_assertions') is not True or
            row.get('iterations_per_sample')!=64 or row.get('warmup')!=8 or row.get('samples')!=7):
            raise ValueError('Timing configuration mismatch')
        for key in ['baseline_ns_per_section','adapter_ns_per_section']:
            data=row.get(key)
            if not isinstance(data,list) or len(data)!=7 or any(type(x)is not int or x<=0 for x in data):
                raise ValueError('Invalid timing sample')
        ratio=sorted(row['adapter_ns_per_section'])[3]/sorted(row['baseline_ns_per_section'])[3]
        if not math.isfinite(row.get('median_ratio',float('nan'))) or row['median_ratio']!=ratio:
            raise ValueError('Timing median ratio mismatch')
    replays=[json.loads(m) for m in re.findall(r'^PLUTO_TERRAIN_REPLAY (.+)$',log,re.M)]
    if not replays:raise ValueError('No scratch/output replay evidence')
    if mode=='registered':
        cases=re.findall(r'^PLUTO_TERRAIN_CASE (.+)$',log,re.M)
        required={'no-claim passed 4 lighting combinations','claimed passed 4 lighting combinations',
                  'invalid-cell passed 4 lighting combinations','post-stale passed 4 lighting combinations',
                  'materials passed: layers, mixed faces, tint, Top, UV, fallback replay, Fusion full-layout smoke'}
        if len(cases)!=len(required) or set(cases)!=required:raise ValueError('Missing/duplicate required CPU cases')
        if not any(r.get('valid_claim_resumed') is True for r in replays):raise ValueError('Budget fallback claim did not resume')
    return {'timings':rows,'replays':replays,'mode':mode,'exact_tests':1}

def verify_sources(c):
    for path,digest in c['source_sha256'].items():
        if A.sha((ROOT/path).read_bytes())!=digest:raise ValueError('Frozen input changed: '+path)

def execute(report):
    c=config();validate_config(c);verify_sources(c)
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if (os.environ.get('GITHUB_ACTIONS')!='true' or os.environ.get('GITHUB_REPOSITORY')!=c['repository'] or
        os.environ.get('GITHUB_EVENT_NAME')!='push' or os.environ.get('GITHUB_REF')!='refs/heads/'+c['branch'] or
        event.get('repository',{}).get('private') is not False or A.git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA')):
        raise ValueError('Only the exact authorized public push branch is supported')
    assert_checkout();before=strict_seal(ROOT)
    work=Path(os.environ['RUNNER_TEMP'])/('terrain-material-cpu-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for k in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(k,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'host-target'),
        CARGO_BUILD_JOBS='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_TEST_OPT_LEVEL='0',
        CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1')
    if min(shutil.disk_usage(p).free for p in A.write_filesystems(work,env))<FLOOR:raise RuntimeError('Preflight disk floor')
    work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
    runner=Runner(work,env,report);g=report['gates'];seals={};generated=None;binary=None;binary_hash=None
    try:
        out=runner.run('runner15',[sys.executable,str(HERE/'test_runner.py'),'-v']);A.validate_python_checks(out,15);g['runner15']='passed'
        out=runner.run('inherited-runner20',[sys.executable,str(HERE.parent/'terrain-author-ci/test_runner.py'),'-v']);A.validate_python_checks(out,20);g['inherited_runner20']='passed'
        runner.guard(reserve=2*1024**3)
        runner.run('apt-update',['sudo','apt-get','update'],timeout=600)
        runner.run('apt-install',['sudo','apt-get','install','-y','--no-install-recommends','libasound2-dev','libudev-dev'],timeout=600)
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal','--component','rustfmt'],timeout=600)
        out=runner.run('source23',['python3','-m','unittest','discover','-s','pluto-lib/tests','-p','test_terrain*.py','-v']);A.validate_python_checks(out,23);g['source23']='passed'
        for name,info in c['official'].items():
            checkout=work/(name+'-git');original=work/name
            runner.run(name+'-init',['git','init',str(checkout)])
            runner.run(name+'-origin',['git','-C',str(checkout),'remote','add','origin',info['url']])
            runner.run(name+'-fetch',['git','-C',str(checkout),'fetch','--depth','1','origin',info['revision']])
            runner.run(name+'-checkout',['git','-C',str(checkout),'checkout','--detach','FETCH_HEAD'])
            if A.git(checkout,'rev-parse','HEAD')!=info['revision'] or A.git(checkout,'remote','get-url','origin')!=info['url'] or A.git(checkout,'status','--porcelain','--untracked-files=all'):
                raise ValueError('Official source identity mismatch')
            runner.run(name+'-export',[sys.executable,str(HERE/'run.py'),'--export',str(checkout),info['revision'],str(original)])
            seals[str(original)]=strict_seal(original)
        official=work/'horizon';mixins=work/'mixins'
        if A.sha((official/'Cargo.lock').read_bytes())!=c['official_lock_sha256']:raise ValueError('Official lock changed')
        g['official_sources']='passed';env['PLUTO_OFFICIAL_SOURCE']=str(official)
        out=runner.run('fixture-source12',[sys.executable,str(FIXTURE/'test_source.py'),'-v']);A.validate_python_checks(out,12);g['fixture_source12']='passed'
        runner.run('mixins-build',['cargo','build','--release','--locked','-p','horizon-mixins-cli','--target-dir',str(work/'mixins-target')],cwd=mixins)
        cli=work/'mixins-target/release/horizon-mixins';bundle=work/'production.hnmod';overlay=work/'overlay'
        runner.run('package',[sys.executable,str(ROOT/'pluto-lib/tools/package_native.py'),str(bundle)])
        if A.sha(bundle.read_bytes())!=c['bundle_sha256']:raise ValueError('Exact 43f production archive changed')
        g['production_package']='passed'
        runner.run('prepare',[sys.executable,str(FIXTURE/'prepare.py'),'--official',str(official),'--bundle',str(bundle),'--output',str(overlay)])
        seals[str(overlay)]=strict_seal(overlay);g['prepare']='passed'
        plan=work/'plan.json';stage=work/'stage'
        runner.run('official-plan',[str(cli),'plan','--workspace',str(official),'--mods',str(overlay/'mods'),'--out',str(plan)]);g['plan']='passed'
        runner.run('all12guards',[sys.executable,str(ROOT/'pluto-lib/tools/check_native_plan.py'),str(plan),str(official)]);g['complete_plan_guards']='passed'
        runner.run('fixture-plan-guard',[sys.executable,str(FIXTURE/'check_plan.py'),str(overlay),str(plan)]);g['fixture_plan_guard']='passed'
        runner.run('official-apply',[str(cli),'apply','--workspace',str(official),'--mods',str(overlay/'mods'),'--out',str(stage)])
        seals[str(stage)]=strict_seal(stage);g['apply']='passed'
        runner.run('locked-fetch',['cargo','fetch','--locked','--manifest-path',str(stage/'Cargo.toml')])
        checked=runner.run('host-type',['cargo','check','--offline','--locked','--manifest-path',str(stage/'Cargo.toml'),'-p','horizon-client','--no-default-features','--lib','--bin','horizon-client','-vv'],cwd=stage)
        A.native_host_checks(checked);assert_strict(stage,seals[str(stage)]);g['host_lib_bin_type']='passed'
        built=runner.run('renderer-compile',['cargo','test','--offline','--locked','--manifest-path',str(stage/'Cargo.toml'),'-p','horizon-client-render','--no-default-features','--lib','--no-run','--message-format=json-render-diagnostics','-vv'],cwd=stage)
        A.native_test_compilation(built,'horizon_client_render','src/lib.rs')
        binary=artifact(built,stage,work/'host-target');binary_hash=A.sha(binary.read_bytes());g['renderer_compile']='passed'
        generated=A.out_dirs(work/'host-target')
        out=runner.run('native-material2',[str(binary),c['native_filter'],'--show-output','--test-threads=1'],cwd=stage)
        A.validate_tests(out,c['native_tests']);g['native_material2']='passed'
        for mode in ['no-provider','registered']:
            for path,seal in seals.items():assert_strict(Path(path),seal)
            if A.out_dirs(work/'host-target')!=generated or A.sha(binary.read_bytes())!=binary_hash:raise ValueError('Pre-execution generated/binary drift')
            env.update(PLUTO_TERRAIN_FIXTURE_RESOURCES=str(overlay/'resources'),PLUTO_TERRAIN_FIXTURE_MODE=mode,
                PLUTO_TERRAIN_PROFILE_LABEL=PROFILE,PLUTO_TERRAIN_TIMING_ITERS='64')
            out=runner.run('cpu-'+mode,[str(binary),TEST,'--exact','--show-output','--test-threads=1'],cwd=stage,timeout=900)
            report['cpu'][mode]=validate_cpu(out,mode)
            runner.run('verify-'+mode,[sys.executable,str(FIXTURE/'prepare.py'),'--output',str(overlay),'--verify'])
            for path,seal in seals.items():assert_strict(Path(path),seal)
            if A.out_dirs(work/'host-target')!=generated or A.sha(binary.read_bytes())!=binary_hash:raise ValueError('Post-execution generated/binary drift')
            g['cpu_'+mode.replace('-','_')]='passed'
    except BaseException as exc:A.record_error(report,exc)
    finally:
        try:
            assert_strict(ROOT,before);assert_checkout();verify_sources(c)
            for path,seal in seals.items():assert_strict(Path(path),seal)
            if generated is not None and A.out_dirs(work/'host-target')!=generated:raise ValueError('Final generated-input drift')
            if binary is not None and A.sha(binary.read_bytes())!=binary_hash:raise ValueError('Final executable drift')
            runner.guard();report['verified']=True
        except BaseException as exc:A.record_error(report,exc,'verification: ')
        report['minimum_free_bytes']=runner.minimum_free
        report['seals']={path:A.sha(json.dumps(seal,sort_keys=True).encode()) for path,seal in seals.items()}
        report['generated_seal']=A.sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None
        report['binary_sha256']=binary_hash

def main():
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},
        'phases':[],'cpu':{},'floor_bytes':FLOOR,'raw_logs':'validated inside runner, not uploaded',
        'scope':'Actual CPU mesh on original synthetic resources; unoptimized timing only. No GPU, production asset or TConstruct activation proof.'}
    try:
        execute(report)
        if report['error'] is None and set(report['gates'])!=REQUIRED_GATES:raise ValueError('Missing/unexpected required gate')
    except BaseException as exc:A.record_error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:f.write('## Terrain CPU proof\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:A.record_error(report,exc,'summary: ');report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)
if __name__=='__main__':
    if len(sys.argv)==5 and sys.argv[1]=='--export':A.export(Path(sys.argv[2]),sys.argv[3],Path(sys.argv[4]))
    elif len(sys.argv)==1:main()
    else:raise SystemExit('Invalid runner arguments')
