#!/usr/bin/env python3
"""Six latest native Font generation contracts; no Game, GPU or LAN proof."""
import importlib.util
import json
import os
from pathlib import Path
import re
import shlex
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
BRANCH='checkpoint/guide-generation-latest-ci-20261106'
REPOSITORY='sofia-gros/horizon-mods'
BASE='a11121ab76d4a783244d13c131261019d5b2a8c8'
PREP='pluto-lib/experiments/guide-generation-latest-fixture'
UTILITY=['pluto-lib/experiments/guide-storage-ci/run.py','tconstruct/tests/guide-identity-ci/run.py']
NATIVE=['horizon-client-core','horizon-client-render']
NAME='pluto-guide-generation-tests'
RUNNER_TESTS=20
REQUIRED={'runner','preparer15','source6','toolchain','official','metadata','compile','inventory21','generation6'}

def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module

old=load('reviewed_guide_runner',ROOT/UTILITY[0])
prep=load('frozen_generation_preparer',ROOT/PREP/'prepare.py')
base,Runner,sha,seal=old.base,old.Runner,old.sha,old.seal

def config():
    value=json.loads((HERE/'inputs.json').read_text())
    if (value['repository'],value['branch'],value['base'])!=(REPOSITORY,BRANCH,BASE):raise ValueError('Wrong immutable source scope')
    expected=set(UTILITY)|{PREP+'/'+n for n in ['prepare.py','test_prepare.py','inputs.json','README.md']}|{
        'pluto-lib/experiments/guide-generation-latest-ci/'+n for n in ['run.py','test_runner.py','README.md']}
    expected.add('.github/workflows/guide-generation-latest-wip.yml')
    if set(value['source_sha256'])!=expected:raise ValueError('Wrong controller source set')
    for name,digest in value['source_sha256'].items():
        if sha(prep.read(ROOT,name))!=digest:raise ValueError('Changed controller input: '+name)
    p=prep.config()
    if value['official']!={'url':old.HORIZON_URL,'revision':prep.HORIZON,'lock_sha256':p['official_sha256']['Cargo.lock']}:raise ValueError('Wrong latest official input')
    if (len(p['inventory_names']),len(set(p['inventory_names'])),len(p['generation_names']),len(set(p['generation_names'])))!=(21,21,6,6):raise ValueError('Wrong fixture inventory')
    if set(p['generation_names'])!={n for n in p['inventory_names'] if n.startswith('consumer_tests::generation_tests::')}:raise ValueError('Wrong bounded generation filter')
    return value,p

def event(env,event,head):
    if (env.get('GITHUB_ACTIONS')!='true' or env.get('GITHUB_REPOSITORY')!=REPOSITORY or
        env.get('GITHUB_EVENT_NAME')!='push' or env.get('GITHUB_REF')!='refs/heads/'+BRANCH or
        env.get('GITHUB_SHA')!=head or event.get('repository',{}).get('private') is not False or
        event.get('repository',{}).get('full_name')!=REPOSITORY):raise ValueError('Require exact public dedicated push')

def inventory(log,names):
    if len(names)!=21 or len(set(names))!=21:raise ValueError('Wrong expected inventory')
    if log!=''.join(n+': test\n' for n in sorted(names)):raise ValueError('Wrong entire terse inventory')

def tests(log,names):
    rows=re.findall(r'^test (\S+) \.\.\. (\w+)\s*$',log,re.M)
    if len(names)!=6 or len(set(names))!=6 or len(rows)!=6 or {n for n,_ in rows}!=set(names) or any(s!='ok' for _,s in rows):raise ValueError('Wrong six passing names')
    if re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',log,re.M)!=[('6','0','0','0','15')]:raise ValueError('Wrong exact selected result')

def compiler_arguments(line):
    # Cargo's leading shell-style environment values are not Rust inputs.
    # In particular CARGO_PKG_HOMEPAGE=https://serde.rs and RUSTC=/x/rustc
    # must not be confused with a source path or the executable token.
    tokens=shlex.split(line.split('Running `',1)[1].rsplit('`',1)[0])
    at=0
    while at<len(tokens) and re.match(r'^[A-Za-z_][A-Za-z0-9_]*=',tokens[at]):at+=1
    command=tokens[at:]
    if not command:return None
    if Path(command[0]).name!='rustc':
        if '--crate-name' in command:raise ValueError('Unexpected compiler command')
        return None
    if sum(Path(token).name=='rustc' for token in command)!=1:
        raise ValueError('Ambiguous rustc executable tokens')
    return command[1:]

def compilers(log,metadata,fixture):
    wanted={}
    for name in NATIVE+[NAME]:
        packages=[p for p in metadata['packages'] if p['name']==name]
        if len(packages)!=1:raise ValueError('Wrong compiler package')
        targets=[t for t in packages[0]['targets'] if t['kind']==['lib']]
        if len(targets)!=1:raise ValueError('Wrong native/fixture target')
        wanted[name.replace('-','_')]=Path(targets[0]['src_path']).resolve()
    result=[]
    for line in log.splitlines():
        if 'Running `' not in line or 'rustc' not in line:continue
        args=compiler_arguments(line)
        if args is None or '--crate-name' not in args:continue
        if args.count('--crate-name')!=1 or args.index('--crate-name')+1==len(args):raise ValueError('Malformed crate-name argument')
        name=args[args.index('--crate-name')+1]
        if name not in wanted:continue
        paths=[(fixture/a).resolve() for a in args if a.endswith('.rs')]
        if paths!=[wanted[name]] or ('--test' in args)!=(name==NAME.replace('-','_')):
            detail={'crate':name,'source_count':len(paths),'sources':[str(p)[:240] for p in paths[:4]],
                    'expected':str(wanted[name])[:240],'test':'--test' in args,'command_sha256':sha(line.encode())}
            print('COMPILER_SOURCE_MISMATCH',json.dumps(detail,sort_keys=True),flush=True)
            raise ValueError('Wrong cold compiler source/profile')
        result.append({'crate':name,'source':str(paths[0]),'test':'--test' in args,'command_sha256':sha(line.encode())})
    if len(result)!=3 or {r['crate'] for r in result}!=set(wanted):raise ValueError('Missing/duplicate cold compiler')
    return result

def artifact(log,metadata,target):
    rows=[json.loads(line) for line in log.splitlines()]
    packages=[p for p in metadata['packages'] if p['name']==NAME]
    if len(packages)!=1:raise ValueError('Wrong artifact owner')
    candidates=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('package_id')==packages[0]['id'] and r.get('profile',{}).get('test') is True and r.get('executable')]
    if len(candidates)!=1:raise ValueError('Missing/duplicate fixture executable')
    row=candidates[0];path=Path(row['executable'])
    if row.get('fresh') is not False or row['target']['kind']!=['lib'] or Path(row['target']['src_path']).resolve()!=Path(packages[0]['manifest_path']).parent/'lib.rs' or not path.is_absolute() or not path.resolve().is_relative_to(target.resolve()):raise ValueError('Wrong executable path/target')
    return path

def outputs(target):
    return {p.relative_to(target).as_posix():seal(p) for p in sorted(target.glob('debug/build/*/out'))}

def prepare_command(host,fixture):
    if os.environ.get('PLUTO_FONT_CONTROLLER')!='generation-latest-v1' or os.getpid()!=os.getpgrp():raise ValueError('Require guarded outer process group')
    event(os.environ,json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text()),base.git('rev-parse','HEAD'))
    prep.prepare(host,fixture)
    print('Prepared exact selected inputs; no metadata or native invocation.',flush=True)

def execute(report):
    value,p=config();head=base.git('rev-parse','HEAD')
    event(os.environ,json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text()),head);base.clean()
    original=seal(ROOT);work=Path(os.environ['RUNNER_TEMP']).resolve()/('guide-generation-latest-'+head)
    old.independent_roots(ROOT,work)
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_NET_OFFLINE','CARGO','RUSTC','GIT_DIR','GIT_INDEX_FILE','GIT_WORK_TREE']:env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),
        CARGO_BUILD_JOBS='1',CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',
        TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1',CARGO_TERM_COLOR='never',PLUTO_FONT_CONTROLLER='generation-latest-v1')
    runner=Runner(work,env);runner.guard(reserve=2*1024**3)
    work.mkdir();(work/'cargo-home').mkdir();(work/'tmp').mkdir()
    host=work/'official';fixture=work/'fixture';target=work/'target'
    host_before=None;fixture_before=None;generated=None;binary=None;compiled=None;metadata=None;g=report['gates']
    try:
        for gate,path,count in [('runner',HERE/'test_runner.py',RUNNER_TESTS),('preparer15',ROOT/PREP/'test_prepare.py',15),('source6',ROOT/'tconstruct/tests/guide-generation-query/test_source.py',6)]:
            old.python_pass(runner.run(gate,[sys.executable,str(path),'-v']),count);g[gate]={'passed':True,'tests':count}
        runner.run('apt-update',['sudo','apt-get','update'],timeout=600)
        runner.run('apt-install',['sudo','apt-get','install','-y','libasound2-dev','libudev-dev'],timeout=600)
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal','--no-self-update'],timeout=600)
        g['toolchain']=runner.run('compiler-identity',['rustc','--version','--verbose']).strip()
        runner.run('official-init',['git','init',str(host)])
        runner.run('official-origin',['git','-C',str(host),'remote','add','origin',value['official']['url']])
        runner.run('official-fetch',['git','-C',str(host),'fetch','--depth','1','origin',prep.HORIZON],timeout=600)
        runner.run('official-checkout',['git','-C',str(host),'checkout','--detach',prep.HORIZON])
        actual=runner.run('official-head',['git','-C',str(host),'rev-parse','HEAD']).strip()
        origin=runner.run('official-url',['git','-C',str(host),'remote','get-url','origin']).strip()
        if actual!=prep.HORIZON or origin!=old.HORIZON_URL:raise ValueError('Wrong official Git identity')
        if runner.run('official-clean',['git','-C',str(host),'status','--porcelain','--untracked-files=all','--ignored=matching']).strip():raise ValueError('Dirty official checkout')
        prep.check_official_inputs(host,p);host_before=seal(host)
        g['official']={'revision':actual,'selected_files':len(p['official_sha256']),'lock_sha256':p['official_sha256']['Cargo.lock']}
        runner.run('locked-fetch',['cargo','fetch','--locked','--manifest-path',str(host/'Cargo.toml')],cwd=host,timeout=900)
        runner.run('prepare',[sys.executable,str(HERE/'run.py'),'--prepare',str(host),str(fixture)])
        raw=runner.run('fixture-metadata',['cargo','metadata','--offline','--format-version','1','--manifest-path',str(fixture/'Cargo.toml')],cwd=fixture,separate_stdout=True)
        metadata=json.loads(raw);prep.verify(fixture,metadata)
        locked=runner.run('locked-metadata',['cargo','metadata','--locked','--offline','--format-version','1','--manifest-path',str(fixture/'Cargo.toml')],cwd=fixture,separate_stdout=True)
        if json.loads(locked)!=metadata:raise ValueError('Resolved metadata changed')
        fixture_before=seal(fixture);g['metadata']={'sha256':sha(raw.encode()),'packages':len(metadata['packages']),'lock_sha256':sha((fixture/'Cargo.lock').read_bytes())}
        raw=runner.run('native-compile',['cargo','test','--lib','--no-run','--locked','--offline','--manifest-path',str(fixture/'Cargo.toml'),'-vv','--message-format=json'],cwd=fixture,timeout=1800,separate_stdout=True)
        witnesses=compilers((work/'native-compile.stderr.log').read_text(),metadata,fixture)
        binary=artifact(raw,metadata,target);compiled=base.binary_seal(binary);generated=outputs(target)
        g['compile']={'passed':True,'actual_compilers':witnesses,'binary':compiled}
        if seal(fixture)!=fixture_before or seal(host)!=host_before or seal(ROOT)!=original:raise ValueError('Source changed during compile')
        inventory(runner.run('inventory',[str(binary),'--list','--format','terse']),p['inventory_names'])
        if base.binary_seal(binary)!=compiled or outputs(target)!=generated:raise ValueError('Binary/generated drift after inventory')
        g['inventory21']={'names':sorted(p['inventory_names']),'tests':21}
        tests(runner.run('generation6',[str(binary),'consumer_tests::generation_tests::','--test-threads=1']),p['generation_names'])
        g['generation6']={'names':sorted(p['generation_names']),'tests':6,'ignored':0,'filtered':15}
    except BaseException as exc:base.error(report,exc)
    finally:
        try:
            base.clean();config()
            if seal(ROOT)!=original:raise ValueError('Mod source drift')
            if host_before is not None and seal(host)!=host_before:raise ValueError('Official source drift')
            if fixture_before is not None and seal(fixture)!=fixture_before:raise ValueError('Fixture/lock drift')
            if metadata is not None:prep.verify(fixture,metadata)
            if generated is not None and outputs(target)!=generated:raise ValueError('OUT_DIR drift')
            if compiled is not None and base.binary_seal(binary)!=compiled:raise ValueError('Executable drift')
            runner.guard();report['verified']=True
        except BaseException as exc:base.error(report,exc)
        report.update(minimum_free_bytes=runner.minimum,source_seal_sha256=sha(json.dumps(original,sort_keys=True).encode()),
            official_seal_sha256=sha(json.dumps(host_before,sort_keys=True).encode()) if host_before else None,
            fixture_seal_sha256=sha(json.dumps(fixture_before,sort_keys=True).encode()) if fixture_before else None,
            output_seal_sha256=sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None)

def main():
    if len(sys.argv)>1:
        if len(sys.argv)!=4 or sys.argv[1]!='--prepare':raise ValueError('Unknown child command')
        prepare_command(Path(sys.argv[2]),Path(sys.argv[3]));return
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},
        'scope':'Six exact production generation cases using actual latest core/render Font and synthetic fixture data. Old twelve and codec three are compiled/inventoried only. No Game, GPU, live drawing, UI/lifetime, language refresh, LAN or installed package proof.'}
    try:
        execute(report)
        if set(report['gates'])!=REQUIRED:raise ValueError('Missing required gates')
    except BaseException as exc:base.error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:f.write('## Latest native Font generation six\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:base.error(report,exc);report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)
if __name__=='__main__':main()
