#!/usr/bin/env python3
"""Isolated owned storage tests/type experiment. No ordinary UI/LAN activation."""
import importlib.util
import json
import os
from pathlib import Path
import re
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SOURCE = ROOT/'pluto-lib/experiments/guide-owned-opening'
STORAGE = ROOT/'pluto-lib/experiments/guide-storage-ci'
BRANCH = 'checkpoint/guide-owned-opening-ci-20261106'
REPOSITORY = 'sofia-gros/horizon-mods'
HELPER = 'crates/horizon-client/src/game/retained_book.rs'
RUNNER_TESTS = 10
REQUIRED = {'runner', 'prior_runner', 'source', 'toolchain', 'compile', 'inventory', 'ownership10',
            'official_source', 'prototype_source', 'experiment_delta', 'metadata', 'host_type'}

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    return module

# Reuse reviewed bounded process/log/disk/metadata/export utilities unchanged.
old = load('prior_storage_ci', STORAGE/'run.py')
base, Runner, sha, seal, FLOOR = old.base, old.Runner, old.sha, old.seal, old.FLOOR

def config():
    value = json.loads((HERE/'inputs.json').read_text())
    if value['branch'] != BRANCH or value['repository'] != REPOSITORY:
        raise ValueError('Wrong authorized branch/repository')
    for path, digest in value['source_sha256'].items():
        p = ROOT/path
        if p.is_symlink() or sha(p.read_bytes()) != digest:
            raise ValueError('Pinned source differs: '+path)
    original = old.config()
    if value['official'] != original['official']:
        raise ValueError('Different official source/lock')
    expected = dict(original['expected_delta'])
    expected[HELPER] = sha(composed_helper())
    if value['expected_delta'] != expected:
        raise ValueError('Only the exact helper extension may differ from the old eleven-file patch')
    source = json.loads((SOURCE/'inputs.json').read_text())
    names = ['retained_book::ownership_tests::'+name for name in source['ownership_tests']]
    previous = re.findall(r'#\[test\]\s*fn (\w+)\(', (old.PROTOTYPE/'retained_book.rs').read_text())
    inventory = ['retained_book::storage_tests::'+name for name in previous] + names
    if len(names)!=10 or len(inventory)!=17 or len(set(inventory))!=17:
        raise ValueError('Wrong bounded test set')
    if value['ownership_names']!=names or value['inventory_names']!=inventory:
        raise ValueError('Frozen test inventory changed')
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

def prepare(pristine, experiment):
    value = config(); old.independent_roots(pristine,experiment)
    before = seal(pristine)
    # This first verifies the exact old proposal, then appends only the two
    # separately pinned fragments. The previous successful output is not reused
    # as evidence for this changed helper.
    old.prepare(pristine,experiment)
    target = experiment/HELPER
    if target.is_symlink() or sha(target.read_bytes())!=old.config()['expected_delta'][HELPER]:
        raise ValueError('Base helper differs before extension')
    target.write_bytes(composed_helper())
    old.verify_delta(pristine,experiment,value['expected_delta'])
    if seal(pristine)!=before: raise ValueError('Pristine changed during preparation')

def execute(report):
    value=config(); head=base.git('rev-parse','HEAD')
    validate_event(os.environ,json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text()),head)
    base.clean(); source=seal(ROOT)
    work=Path(os.environ['RUNNER_TEMP']).resolve()/('guide-owned-opening-'+head)
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
        runner.run('compile-ownership',['rustc','--edition','2021','--test',str(SOURCE/'fixture.rs'),'-C','debuginfo=0','-o',str(binary)],timeout=300)
        compiled=base.binary_seal(binary);report['gates']['compile']=compiled
        inventory(runner.run('inventory',[str(binary),'--list','--format','terse']),value['inventory_names'])
        report['gates']['inventory']=value['inventory_names']
        tested=runner.run('ownership10',[str(binary),'retained_book::ownership_tests::','--test-threads=1'])
        tests(tested,value['ownership_names'])
        if base.binary_seal(binary)!=compiled:raise ValueError('Compiled executable changed')
        report['gates']['ownership10']={'names':value['ownership_names'],'passed':10,'filtered':7,'log_sha256':sha(tested.encode())}
        runner.run('official-init',['git','init',str(checkout)])
        runner.run('official-origin',['git','-C',str(checkout),'remote','add','origin',old.HORIZON_URL])
        runner.run('official-fetch',['git','-C',str(checkout),'fetch','--depth','1','origin',old.HORIZON],timeout=600)
        runner.run('official-checkout',['git','-C',str(checkout),'checkout','--detach',old.HORIZON])
        exact=runner.run('official-head',['git','-C',str(checkout),'rev-parse','HEAD']).strip()
        if exact!=old.HORIZON:raise ValueError('Wrong official commit')
        seals[str(checkout)]=seal(checkout)
        runner.run('official-export',[sys.executable,str(HERE/'run.py'),'--export',str(checkout),str(pristine)])
        if sha((pristine/'Cargo.lock').read_bytes())!=old.LOCK:raise ValueError('Official lock drift')
        if 'sofia-gros/horizon' in (pristine/'Cargo.lock').read_text():raise ValueError('Forbidden dependency destination')
        seals[str(pristine)]=seal(pristine);report['gates']['official_source']={'revision':exact,'lock_sha256':old.LOCK}
        env['PLUTO_OFFICIAL_SOURCE']=str(pristine)
        old.python_pass(runner.run('prototype-source',[sys.executable,str(old.PROTOTYPE/'test_patch.py'),'-v']),7)
        report['gates']['prototype_source']=7
        runner.run('prepare-experiment',[sys.executable,str(HERE/'run.py'),'--prepare',str(pristine),str(experiment)])
        seals[str(experiment)]=old.verify_delta(pristine,experiment,value['expected_delta'])
        report['gates']['experiment_delta']={'paths':value['expected_delta'],'count':11,'helper_appended_fragments':2}
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
            'scope':'Ten generic owned-storage tests plus excluded official Game/BookScreen lib+bin typing. Not ordinary Pluto composition, permitted Game opening, actual Client/UI, catalog reload or LAN reply execution.'}
    try:
        execute(report)
        if set(report['gates'])!=REQUIRED:raise ValueError('Missing required gate')
    except BaseException as exc:base.error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:
            f.write('## Isolated owned Guide storage\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:base.error(report,exc);report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)

if __name__=='__main__':
    if len(sys.argv)==4 and sys.argv[1]=='--export':config();old.export(Path(sys.argv[2]),Path(sys.argv[3]))
    elif len(sys.argv)==4 and sys.argv[1]=='--prepare':prepare(Path(sys.argv[2]),Path(sys.argv[3]))
    elif len(sys.argv)==1:main()
    else:raise SystemExit('Invalid runner mode')
