#!/usr/bin/env python3
"""Locked helper14 only. No native writer/stamp, planner or Game execution."""
import importlib.util,json,os,re,shlex,shutil,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
base=load('successful_pack8_helpers',HERE.parent/'language-pack-chain-latest-ci/run.py')
prepare=load('repair_fixture_prepare',HERE/'prepare.py')
REPOSITORY='sofia-gros/horizon-mods'
BRANCH='checkpoint/plutolib-language-repair-helper-ci-20261106'
SOURCE='489f63c54861225874efe99f11dbb0399fe8e1c7'
OFFICIAL={'url':prepare.URL,'revision':prepare.PIN,'lock_sha256':prepare.LOCK}
REQUIRED={'runner','source13','placement12','metadata','actual_helper_compile','helper14'}
PREFIX='pluto-lib/experiments/'
FILES={PREFIX+'language-repair-helper-ci/'+p for p in ['run.py','prepare.py','test_runner.py','README.md']}
FILES|={PREFIX+'language-overlay-repair/'+p for p in ['merge.rs','source.py','inputs.json','test_source.py','README.md','source-validation.json']}
FILES|={PREFIX+'language-overlay-placement/'+p for p in ['horizon-mixin.toml','write-head.rs','stamp-store.rs','prepare.py','test_source.py','inputs.json','fixture.rs','Cargo.toml.in','README.md','source-validation.json']}
FILES|={PREFIX+'language-pack-chain-latest-ci/run.py','.github/workflows/language-repair-helper-wip.yml'}
def config():return json.loads((HERE/'inputs.json').read_bytes())
def validate(value):
    if (value['repository'],value['branch'],value['source'])!=(REPOSITORY,BRANCH,SOURCE) or value['official']!=OFFICIAL:
        raise ValueError('Wrong fixed trigger/source/official target')
    if set(value['source_sha256'])!=FILES:raise ValueError('Wrong frozen file-set')
    for p,digest in value['source_sha256'].items():
        path=ROOT/p
        if path.is_symlink() or base.sha(path.read_bytes())!=digest:raise ValueError('Changed source: '+p)
    expected=json.loads((prepare.PLACEMENT/'inputs.json').read_bytes())['expected_tests']
    if value['tests']!=expected or len(expected)!=len(set(expected)) or len(expected)!=14:raise ValueError('Wrong complete helper inventory')
def event_allowed(event,env):
    return (env.get('GITHUB_ACTIONS')=='true' and env.get('GITHUB_REPOSITORY')==REPOSITORY and
      env.get('GITHUB_EVENT_NAME')=='push' and env.get('GITHUB_REF')=='refs/heads/'+BRANCH and event.get('repository',{}).get('private') is False)
def compilers(log,metadata,fixture):
    expected={}
    for name in ['serde','serde_json',prepare.NAME]:
        package=[p for p in metadata['packages'] if p['name']==name]
        if len(package)!=1:raise ValueError('Missing compiler package')
        targets=[t for t in package[0]['targets'] if t['kind']==['lib']]
        if len(targets)!=1:raise ValueError('Wrong lib target')
        expected[name.replace('-','_')]=Path(targets[0]['src_path']).resolve()
    result=[]
    for line in log.splitlines():
        if 'Running `' not in line or 'rustc' not in line:continue
        args=shlex.split(line.split('Running `',1)[1].rsplit('`',1)[0])
        if '--crate-name' not in args:continue
        name=args[args.index('--crate-name')+1]
        if name.startswith('horizon_'):raise ValueError('Unexpected host compilation in helper-only proof')
        if name not in expected:continue
        paths=[(fixture/arg).resolve() for arg in args if arg.endswith('.rs')]
        if paths!=[expected[name]] or ('--test' in args)!=(name==prepare.NAME.replace('-','_')):
            raise ValueError('Wrong actual compiler source/profile: '+name)
        result.append({'crate':name,'source':str(paths[0]),'test':'--test' in args,'invocation_sha256':base.sha(line.encode())})
    if len(result)!=3 or {r['crate'] for r in result}!=set(expected):raise ValueError('Missing/duplicate actual compiler witnesses')
    return result
def execute(report):
    value=config();validate(value)
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if not event_allowed(event,os.environ):raise ValueError('Require authorized public dedicated CI; no local execution')
    if base.git(ROOT,'rev-parse','HEAD')!=os.environ['GITHUB_SHA'] or base.git(ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching'):
        raise ValueError('Wrong/dirty Mod checkout')
    original=base.seal(ROOT);work=Path(os.environ['RUNNER_TEMP'])/('language-repair-helper-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for k in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_NET_OFFLINE','CARGO','RUSTC','GIT_DIR','GIT_INDEX_FILE','GIT_WORK_TREE']:env.pop(k,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),CARGO_BUILD_JOBS='1',
               CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',
               TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1',PLUTO_FIXTURE_CONTROLLER='language-repair-helper-v1')
    runner=base.Runner(work,env);runner.guard(reserve=2*1024**3)
    work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
    host=work/'official';fixture=work/'fixture';host_seal=None;fixture_seal=None;generated=None;gates=report['gates']
    try:
        out=runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v'])
        base.validate_python_checks(out,value['runner_tests']);gates['runner']={'passed':True,'tests':value['runner_tests']}
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal'],timeout=600)
        runner.run('official-init',['git','init',str(host)])
        runner.run('official-origin',['git','-C',str(host),'remote','add','origin',OFFICIAL['url']])
        runner.run('official-fetch',['git','-C',str(host),'fetch','--depth','1','origin',OFFICIAL['revision']],timeout=600)
        runner.run('official-checkout',['git','-C',str(host),'checkout','--detach','FETCH_HEAD'])
        if base.git(host,'rev-parse','HEAD')!=OFFICIAL['revision'] or base.git(host,'remote','get-url','origin')!=OFFICIAL['url'] or base.git(host,'status','--porcelain','--untracked-files=all','--ignored=matching'):
            raise ValueError('Wrong/dirty official checkout')
        if base.sha((host/'Cargo.lock').read_bytes())!=OFFICIAL['lock_sha256']:raise ValueError('Wrong official lock')
        host_seal=base.seal(host);env['PLUTO_SELECTED_SOURCE']=str(host)
        for gate,path,count in [('source13',prepare.REPAIR,13),('placement12',prepare.PLACEMENT,12)]:
            out=runner.run(gate,[sys.executable,str(path/'test_source.py'),'-v']);base.validate_python_checks(out,count);gates[gate]={'passed':True,'tests':count,'scope':'source/synthetic inspection, not accepted native plan'}
        runner.run('official-locked-fetch',['cargo','fetch','--locked','--manifest-path',str(host/'Cargo.toml')],timeout=900)
        env['CARGO_NET_OFFLINE']='true'
        args=[sys.executable,str(HERE/'prepare.py'),'--official',str(host),'--output',str(fixture)]
        runner.run('prepare',args);runner.run('metadata',args+['--resolve']);runner.run('verify-before',args+['--verify'])
        evidence=json.loads((fixture/'fixture-sources.json').read_bytes());metadata=json.loads((fixture/'metadata.json').read_bytes())
        if evidence['expected_names']!=value['tests']:raise ValueError('Prepared test scope changed')
        fixture_seal=base.regular_seal(fixture)
        gates['metadata']={'passed':True,'packages':evidence['packages'],'lock_sha256':evidence['lock_sha256'],'metadata_sha256':evidence['metadata_sha256']}
        command=['cargo','test','--lib','--locked','--offline','--manifest-path',str(fixture/'Cargo.toml'),'-vv']
        out=runner.run('helper-compile',command+['--no-run'],cwd=fixture,timeout=600)
        witnesses=compilers(out,metadata,fixture);print('ACTUAL_HELPER_COMPILERS',json.dumps(witnesses),flush=True)
        gates['actual_helper_compile']={'passed':True,'witnesses':witnesses,'native_host':False}
        generated=base.out_dirs(work/'target')
        names=base.test_after_compile(runner,'helper14',command+['--','--test-threads=1'],14,value['tests'])
        gates['helper14']={'passed':True,'names':names,'tests':14,'ignored':0,'filtered':0}
        runner.run('verify-after',args+['--verify'])
        if set(gates)!=REQUIRED:raise ValueError('Missing required gate')
    except BaseException as exc:base.report_error(report,exc)
    finally:
        try:
            base.assert_seal(ROOT,original)
            if base.git(ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching'):raise ValueError('Dirty source checkout')
            if host_seal is not None:
                base.assert_seal(host,host_seal)
                if base.git(host,'status','--porcelain','--untracked-files=all','--ignored=matching'):raise ValueError('Dirty official checkout')
            if fixture_seal is not None and base.regular_seal(fixture)!=fixture_seal:raise ValueError('Fixture/lock/metadata drift')
            if generated is not None and base.out_dirs(work/'target')!=generated:raise ValueError('Generated inputs drift')
            runner.guard();report['verified']=True
        except BaseException as exc:base.report_error(report,exc,'verification: ')
        report.update(minimum_free_bytes=runner.minimum_free,source_seal_sha256=base.sha(json.dumps(original,sort_keys=True).encode()),
                      official_seal_sha256=base.sha(json.dumps(host_seal,sort_keys=True).encode()) if host_seal else None,
                      fixture_seal_sha256=base.sha(json.dumps(fixture_seal,sort_keys=True).encode()) if fixture_seal else None,
                      generated_seal_sha256=base.sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None)
def main():
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},
            'scope':'Exact frozen merge helper14 against actual official-locked serde/std only. No host writer/stamp, Mixins placement/apply, actual build/cache, pack-priority repair, Game or UI proof.'}
    try:
        execute(report)
        if set(report['gates'])!=REQUIRED:raise ValueError('Missing required gates')
    except BaseException as exc:base.report_error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as out:out.write('## Language helper14 only\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:base.report_error(report,exc,'summary: ');report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)
if __name__=='__main__':main()
