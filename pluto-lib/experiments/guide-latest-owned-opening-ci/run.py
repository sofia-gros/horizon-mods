#!/usr/bin/env python3
"""Latest isolated owned storage tests/type experiment. No ordinary UI/LAN activation."""
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SOURCE = ROOT/'pluto-lib/experiments/guide-owned-opening'
STORAGE = ROOT/'pluto-lib/experiments/guide-storage-ci'
BRANCH = 'checkpoint/guide-latest-owned-opening-ci-20261106'
REPOSITORY = 'sofia-gros/horizon-mods'
HELPER = 'crates/horizon-client/src/game/retained_book.rs'
LATEST = ROOT/'pluto-lib/experiments/guide-latest-owned-opening'
PRIOR = ROOT/'pluto-lib/experiments/guide-owned-opening-ci'
HORIZON = 'f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
LOCK = '0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f'
RUNNER_TESTS = 13
REQUIRED = {'runner', 'prior_runner', 'source', 'toolchain', 'compile', 'inventory', 'ownership10',
            'official_source', 'latest_inventory', 'prototype_identity', 'latest_source', 'experiment_delta', 'metadata', 'host_type'}

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    return module

# Reuse reviewed bounded process/log/disk/metadata/export utilities unchanged.
prior = load('prior_owned_ci', PRIOR/'run.py')
old = prior.old
latest = load('latest_owned_source', LATEST/'generate.py')
base, Runner, sha, seal, FLOOR = old.base, old.Runner, old.sha, old.seal, old.FLOOR

def config():
    value = json.loads((HERE/'inputs.json').read_text())
    if value['branch'] != BRANCH or value['repository'] != REPOSITORY:
        raise ValueError('Wrong authorized branch/repository')
    for path, digest in value['source_sha256'].items():
        p = ROOT/path
        if p.is_symlink() or sha(p.read_bytes()) != digest:
            raise ValueError('Pinned source differs: '+path)
    previous = prior.config()
    pins = latest.config()
    if value['official'] != {'url': old.HORIZON_URL, 'revision': HORIZON, 'lock_sha256': LOCK}:
        raise ValueError('Different latest official source/lock')
    if (pins['official_horizon'],pins['official_lock_sha256']) != (HORIZON,LOCK):
        raise ValueError('Latest source candidate uses another target')
    result = json.loads((LATEST/'generation.json').read_text())
    expected = {p:result['output_sha256'][p] for p in result['changed_files']}
    if value['expected_delta'] != expected or len(expected)!=11:
        raise ValueError('Different latest eleven-file experiment')
    if expected[HELPER] != previous['expected_delta'][HELPER] or expected[HELPER] != sha(composed_helper()):
        raise ValueError('Original storage and owned fragments changed')
    for field in ['ownership_names','inventory_names']:
        if value[field] != previous[field]:
            raise ValueError('Old ownership fixture inventory changed')
    return value

def composed_helper():
    return b'\n'.join([(old.PROTOTYPE/'retained_book.rs').read_bytes(),
                       (SOURCE/'owned.rs').read_bytes(), (SOURCE/'book-view.rs').read_bytes()])

def validate_event(env, event, head):
    if (env.get('GITHUB_ACTIONS')!='true' or env.get('GITHUB_REPOSITORY')!=REPOSITORY or
        env.get('GITHUB_EVENT_NAME')!='push' or env.get('GITHUB_REF')!='refs/heads/'+BRANCH or
        env.get('GITHUB_SHA')!=head or event.get('repository',{}).get('private') is not False or
        event.get('repository',{}).get('full_name')!=REPOSITORY):
        raise ValueError('Require exact authorized public push')

def inventory(log, names):
    rows = re.findall(r'^(\S+): test$', log, re.M)
    if len(rows)!=17 or set(rows)!=set(names): raise ValueError('Wrong complete test inventory')
    if re.findall(r'^(\d+) tests, (\d+) benchmarks$',log,re.M)!=[('17','0')]:
        raise ValueError('Wrong inventory summary')

def tests(log, names):
    rows = re.findall(r'^test (\S+) \.\.\. (\w+)\s*$',log,re.M)
    if len(rows)!=10 or {n for n,_ in rows}!=set(names) or any(r!='ok' for _,r in rows):
        raise ValueError('Missing, extra, duplicate, ignored or failed selected case')
    if re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',log,re.M)!=[('10','0','0','0','7')]:
        raise ValueError('Wrong exact test summary')

def complete_inventory(log):
    complete=json.loads(log)
    partial=json.loads((LATEST/'generation.json').read_text())
    expected=dict(partial,full_inventory_verified=True,status='Full current game direct-site inventory and exact source transform checked; not applied or typed.')
    if complete!=expected:raise ValueError('Wrong complete latest inventory receipt')
    return complete

def export(checkout, destination):
    old.independent_roots(checkout,destination)
    portable=load('latest_portable_git', ROOT/'tconstruct/tools/portable_archive.py')
    destination.mkdir()
    proc=subprocess.Popen(['git','-C',str(checkout),'archive','--format=tar',HORIZON],stdout=subprocess.PIPE)
    try:
        portable.extract_git_tar(proc.stdout,destination)
        if proc.wait(timeout=60):raise ValueError('Latest official export failed')
    finally:
        proc.stdout.close()
        if proc.poll() is None:proc.terminate();proc.wait(timeout=2)

def prepare(pristine, experiment):
    value=config();old.independent_roots(pristine,experiment)
    before=seal(pristine);pins=latest.config();legacy=latest.legacy()
    # Rescan the entire exported subtree as well as the earlier clean Git gate.
    inputs=legacy.read_inputs(pristine,pins)
    patch,result=latest.build_known_inputs(inputs,pins)
    if patch!=(LATEST/'prototype.patch').read_text() or result!=json.loads((LATEST/'generation.json').read_text()):
        raise ValueError('Latest patch/partial receipt differs')
    transformed=legacy.apply_in_memory(patch,inputs)
    if {p:sha(transformed[p].encode()) for p in result['changed_files']}!=value['expected_delta']:
        raise ValueError('Latest output differs')
    shutil.copytree(pristine,experiment)
    for name in result['changed_files']:
        target=experiment/name
        if target.is_symlink() or not target.resolve().is_relative_to(experiment.resolve()):
            raise ValueError('Unsafe experiment target')
        target.write_text(transformed[name])
    old.verify_delta(pristine,experiment,value['expected_delta'])
    if seal(pristine)!=before:raise ValueError('Pristine changed during preparation')

def execute(report):
    value=config(); head=base.git('rev-parse','HEAD')
    validate_event(os.environ,json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text()),head)
    base.clean(); source=seal(ROOT)
    work=Path(os.environ['RUNNER_TEMP']).resolve()/('guide-latest-owned-opening-'+head)
    old.independent_roots(ROOT,work)
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),
               CARGO_BUILD_JOBS='1',CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',
               TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1',CARGO_TERM_COLOR='never')
    runner=Runner(work,env);runner.guard(reserve=2*1024**3)
    work.mkdir();(work/'cargo-home').mkdir();(work/'tmp').mkdir()
    checkout=work/'official-git';pristine=work/'pristine';experiment=work/'experiment';binary=work/'ownership-tests'
    seals={};generated=None;compiled=None
    try:
        old.python_pass(runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v']),RUNNER_TESTS)
        report['gates']['runner']=RUNNER_TESTS
        old.python_pass(runner.run('prior-runner-checks',[sys.executable,str(STORAGE/'test_runner.py'),'-v']),15)
        report['gates']['prior_runner']=15
        checked=json.loads(runner.run('source-check',[sys.executable,str(SOURCE/'check_source.py')]))
        if checked!={'source_files':12,'ownership_tests':10,'rust_executed':False,'native_type_executed':False}:
            raise ValueError('Different source-only guard result')
        report['gates']['source']=checked
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal','--no-self-update'],timeout=600)
        report['gates']['toolchain']=runner.run('rustc-identity',['rustc','--version','--verbose']).strip()
        runner.run('official-init',['git','init',str(checkout)])
        runner.run('official-origin',['git','-C',str(checkout),'remote','add','origin',old.HORIZON_URL])
        runner.run('official-fetch',['git','-C',str(checkout),'fetch','--depth','1','origin',HORIZON],timeout=600)
        runner.run('official-checkout',['git','-C',str(checkout),'checkout','--detach',HORIZON])
        exact=runner.run('official-head',['git','-C',str(checkout),'rev-parse','HEAD']).strip()
        if exact!=HORIZON:raise ValueError('Wrong official commit')
        seals[str(checkout)]=seal(checkout)
        complete=complete_inventory(runner.run('latest-inventory',[sys.executable,str(LATEST/'generate.py'),'--official',str(checkout)],timeout=180))
        report['gates']['latest_inventory']={'full_inventory_verified':True,'known_source_files':19,
            'book_sites':sum(len(x['accesses']) for x in latest.config()['book_field_inventory']),
            'attachment_sites':sum(len(x['sites']) for x in latest.config()['attachment_inventory']),
            'patch_sha256':complete['patch_sha256'],'receipt_sha256':sha(json.dumps(complete,sort_keys=True).encode())}
        report['gates']['prototype_identity']={'original_fragments_unchanged':True,'helper_sha256':sha(composed_helper())}
        runner.run('official-export',[sys.executable,str(HERE/'run.py'),'--export',str(checkout),str(pristine)])
        if sha((pristine/'Cargo.lock').read_bytes())!=LOCK:raise ValueError('Official lock drift')
        if 'sofia-gros/horizon' in (pristine/'Cargo.lock').read_text():raise ValueError('Forbidden dependency destination')
        seals[str(pristine)]=seal(pristine);report['gates']['official_source']={'revision':exact,'lock_sha256':LOCK}
        old.python_pass(runner.run('latest-source',[sys.executable,str(LATEST/'test_source.py'),'-v']),10)
        report['gates']['latest_source']=10
        runner.run('compile-ownership',['rustc','--edition','2021','--test',str(SOURCE/'fixture.rs'),'-C','debuginfo=0','-o',str(binary)],timeout=300)
        compiled=base.binary_seal(binary);report['gates']['compile']=compiled
        inventory(runner.run('inventory',[str(binary),'--list','--format','terse']),value['inventory_names'])
        report['gates']['inventory']=value['inventory_names']
        tested=runner.run('ownership10',[str(binary),'retained_book::ownership_tests::','--test-threads=1'])
        tests(tested,value['ownership_names'])
        if base.binary_seal(binary)!=compiled:raise ValueError('Compiled executable changed')
        report['gates']['ownership10']={'names':value['ownership_names'],'passed':10,'filtered':7,'log_sha256':sha(tested.encode())}
        runner.run('prepare-experiment',[sys.executable,str(HERE/'run.py'),'--prepare',str(pristine),str(experiment)])
        seals[str(experiment)]=old.verify_delta(pristine,experiment,value['expected_delta'])
        report['gates']['experiment_delta']={'paths':value['expected_delta'],'count':11,'helper_fragments_unchanged':3}
        runner.run('locked-fetch',['cargo','fetch','--locked','--manifest-path',str(pristine/'Cargo.toml')],cwd=pristine,timeout=900)
        arguments=['cargo','metadata','--locked','--offline','--no-default-features','--format-version','1','--manifest-path']
        a=old.metadata(runner.run('pristine-metadata',arguments+[str(pristine/'Cargo.toml')],cwd=pristine,separate_stdout=True),pristine)
        b=old.metadata(runner.run('experiment-metadata',arguments+[str(experiment/'Cargo.toml')],cwd=experiment,separate_stdout=True),experiment)
        if a!=b:raise ValueError('Dependency/feature graph changed beyond source-root relocation')
        report['gates']['metadata']={'normalized_sha256':sha(a.encode()),'full_dependencies':True}
        typed=runner.run('host-type',['cargo','check','--offline','--locked','--manifest-path',str(experiment/'Cargo.toml'),
                       '-p','horizon-client','--no-default-features','--lib','--bin','horizon-client','-vv'],cwd=experiment,timeout=2400)
        report['gates']['host_type']={'actual_compiler_invocations':old.host_invocations(typed),'log_sha256':sha(typed.encode())}
        generated=old.out_dirs(work/'target')
    except BaseException as exc:base.error(report,exc)
    finally:
        try:
            base.clean();config()
            if seal(ROOT)!=source:raise ValueError('Mod source drift')
            for name,before in seals.items():
                if seal(Path(name))!=before:raise ValueError('Official/experiment source drift: '+name)
            if compiled is not None and base.binary_seal(binary)!=compiled:raise ValueError('Executable drift')
            if generated is not None and old.out_dirs(work/'target')!=generated:raise ValueError('Build output drift')
            runner.guard();report['verified']=True
        except BaseException as exc:base.error(report,exc)
        report.update(minimum_free_bytes=runner.minimum,mod_source_seal=sha(json.dumps(source,sort_keys=True).encode()),
                      source_seals={p:sha(json.dumps(s,sort_keys=True).encode()) for p,s in seals.items()},
                      build_outputs_seal=sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None)

def main():
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},'floor_bytes':FLOOR,
            'scope':'Latest f4dc target: ten generic owned-storage tests plus excluded official Game/BookScreen lib+bin typing. Not ordinary Pluto composition, permitted Game opening, actual Client/UI, catalog reload or LAN reply execution.'}
    try:
        execute(report)
        if set(report['gates'])!=REQUIRED:raise ValueError('Missing required gate')
    except BaseException as exc:base.error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:
            f.write('## Latest isolated owned Guide storage\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:base.error(report,exc);report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)

if __name__=='__main__':
    if len(sys.argv)==4 and sys.argv[1]=='--export':config();export(Path(sys.argv[2]),Path(sys.argv[3]))
    elif len(sys.argv)==4 and sys.argv[1]=='--prepare':prepare(Path(sys.argv[2]),Path(sys.argv[3]))
    elif len(sys.argv)==1:main()
    else:raise SystemExit('Invalid runner mode')
