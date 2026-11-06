#!/usr/bin/env python3
"""Unchanged main + exact successful common: public plan-only diagnostic, never apply."""
import importlib.util,json,os,re,shlex,shutil,sys
from pathlib import Path
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
common=load('verified_common_helpers',ROOT/'pluto-lib/experiments/common-composite-ci/run.py')
prepare=load('baseline_prepare',HERE/'prepare.py')
safety=load('baseline_safety',HERE/'safety.py')
sha=common.sha
REPOSITORY='sofia-gros/horizon-mods'
BRANCH='checkpoint/plutolib-main-common-plan-ci-20261106'
MAIN=prepare.MAIN
OFFICIAL={'horizon':{'url':'https://github.com/horizon-rust/horizon.git','revision':prepare.HORIZON,
 'lock':'463913123985e344f2889e5beae65982773e6dbfacdcac7a916bcec522a5bd31'},
 'mixins':{'url':'https://github.com/horizon-rust/horizon-mixins.git','revision':prepare.MIXINS,
 'lock':'35846f4a621cf8e1e54f6f0f7e13ac848db590d4ec401620501c107954eae6ca'}}
RUNNER_TESTS=20
PREFIX='pluto-lib/experiments/main-common-ci/'
INPUT_PATHS={PREFIX+p for p in ['run.py','prepare.py','safety.py','test_runner.py','README.md']}
INPUT_PATHS|={'.github/workflows/main-common-plan-wip.yml','pluto-lib/experiments/common-composite-ci/run.py',
 'pluto-lib/experiments/common-composite-ci/inputs.json','tconstruct/tools/portable_archive.py'}
REQUIRED={'runner','memory_admission','fixed_checkouts','cli_compile','main_bundle','common_bundle','combined_payload','plan_diagnostic'}

class Runner(common.Runner):
    def guard(self,reserve=0):
        super().guard(reserve)
        available=safety.check_memory(reserve=2*1024**3 if reserve else 512*1024**2)
        self.minimum_memory=min(getattr(self,'minimum_memory',available),available)
        if self.work.exists():
            if any(p.stat().st_size>128*1024**2 for p in self.work.glob('*.log')):
                raise RuntimeError('Bounded phase log exceeded 128 MiB')

def inputs():return json.loads((HERE/'inputs.json').read_text())
def validate(config):
    if (config['repository'],config['branch'],config['main'],config['common_bundle_sha256'])!=(REPOSITORY,BRANCH,MAIN,prepare.BUNDLE):
        raise ValueError('Wrong fixed baseline or trigger')
    if config['official']!=OFFICIAL:raise ValueError('Wrong official source or lock')
    if set(config['source_sha256'])!=INPUT_PATHS:raise ValueError('Input file-set changed')
    for p,digest in config['source_sha256'].items():
        path=ROOT/p
        if path.is_symlink() or sha(path.read_bytes())!=digest:raise ValueError('Changed source: '+p)
    common.validate_inputs(common.read_inputs())

def event_allowed(env,event):
    return (env.get('GITHUB_ACTIONS')=='true' and env.get('GITHUB_REPOSITORY')==REPOSITORY and
      env.get('GITHUB_EVENT_NAME')=='push' and env.get('GITHUB_REF')=='refs/heads/'+BRANCH and
      event.get('repository',{}).get('private') is False)

def cli_witness(log):
    found=[]
    for line in log.splitlines():
        if 'Running `' not in line or 'rustc' not in line:continue
        args=shlex.split(line.split('Running `',1)[1].rsplit('`',1)[0])
        if '--crate-name' not in args:continue
        crate=args[args.index('--crate-name')+1]
        if crate.startswith(('horizon_client','horizon_items','horizon_world','pluto_')):
            raise ValueError('Unexpected Game/consumer compilation')
        if crate=='horizon_mixins' and '--test' not in args:
            sources=[a for a in args if a.endswith('.rs')]
            if not any(a.endswith('horizon-mixins-cli/src/main.rs') for a in sources):
                raise ValueError('Wrong actual CLI source')
            found.append({'crate':crate,'source':sources,'invocation_sha256':sha(line.encode())})
    if len(found)!=1:raise ValueError('Expected one actual official CLI compilation')
    return found[0]

def attempt_plan(runner,cli,official,mods,plan):
    # One invocation only. Expected planner rejection is evidence, never a retry.
    try:
        output=runner.run('official-plan',[str(cli),'plan','--workspace',str(official),'--mods',str(mods),'--out',str(plan)],timeout=180)
    except RuntimeError:
        phase=runner.report['phases'][-1] if runner.report['phases'] else {}
        if phase.get('name')!='official-plan' or phase.get('exit_code')!=1 or phase.get('error')!='RuntimeError: official-plan exited 1':raise
        if plan.exists():raise ValueError('Rejected planner unexpectedly wrote a plan')
        return {'accepted':False,'diagnostic':prepare.diagnostic((runner.work/'official-plan.log').read_text())}
    value=json.loads(output)
    if value.get('status')!='ok' or not plan.is_file():raise ValueError('Planner success lacks actual plan')
    return {'accepted':True,'summary':value,'plan_sha256':sha(plan.read_bytes())}

def execute(report):
    config=inputs();validate(config)
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if not event_allowed(os.environ,event):raise ValueError('Require exact public dedicated push branch; no local execution')
    if common.git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA'):raise ValueError('Wrong CI head')
    common.assert_git_clean(ROOT)
    original=common.checkout_seal(ROOT);original_git=prepare.git_state(ROOT)
    report['memory_observation']=safety.memory_observation()
    work=Path(os.environ['RUNNER_TEMP'])/('main-common-plan-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for k in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO','RUSTC','CARGO_NET_OFFLINE','GIT_DIR','GIT_INDEX_FILE','GIT_WORK_TREE']:
        env.pop(k,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'cli-target'),
      CARGO_BUILD_JOBS='1',CARGO_PROFILE_RELEASE_DEBUG='0',CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',
      TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1',GIT_OPTIONAL_LOCKS='0')
    runner=Runner(work,env,report);snapshots={};git_snapshots={};artifact_seals={};cli=None;cli_hash=None;plan=None
    try:
        runner.guard(reserve=4*1024**3)
        report['gates']['memory_admission']={'passed':True,'available_bytes':runner.minimum_memory,'capacity_minimum':6*1024**3,'admission_available_minimum':2*1024**3}
        work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
        out=runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v'])
        common.validate_python_checks(out,RUNNER_TESTS);report['gates']['runner']={'passed':True,'tests':RUNNER_TESTS}
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal','--component','rustfmt'],timeout=600)
        roots={}
        pins={**OFFICIAL,'main':{'url':'https://github.com/sofia-gros/horizon-mods.git','revision':MAIN}}
        for name,pin in pins.items():
            root=work/(name+'-git');roots[name]=root
            runner.run(name+'-init',['git','init',str(root)])
            runner.run(name+'-origin',['git','-C',str(root),'remote','add','origin',pin['url']])
            runner.run(name+'-fetch',['git','-C',str(root),'fetch','--depth','1','origin',pin['revision']],timeout=600)
            runner.run(name+'-checkout',['git','-C',str(root),'checkout','--detach','FETCH_HEAD'])
            common.assert_git_clean(root)
            if common.git(root,'rev-parse','HEAD')!=pin['revision'] or common.git(root,'remote','get-url','origin')!=pin['url']:
                raise ValueError('Fixed checkout substituted')
            if 'lock' in pin and sha((root/'Cargo.lock').read_bytes())!=pin['lock']:raise ValueError('Wrong official lock')
            if name=='main' and common.git(root,'rev-parse','HEAD^{tree}')!='0c766bc524fc42580c5c7cd04ab71374a4eaf481':
                raise ValueError('Wrong fixed main tree')
            snapshots[str(root)]=common.checkout_seal(root);git_snapshots[str(root)]=prepare.git_state(root)
        report['gates']['fixed_checkouts']={'passed':True,'main':MAIN,'official':OFFICIAL}
        official=work/'horizon'
        runner.run('horizon-export',[sys.executable,str(HERE/'run.py'),'--export',str(roots['horizon']),prepare.HORIZON,str(official)])
        snapshots[str(official)]=common.checkout_seal(official)
        runner.run('cli-locked-fetch',['cargo','fetch','--locked','--manifest-path',str(roots['mixins']/'Cargo.toml')],timeout=900)
        env['CARGO_NET_OFFLINE']='true'
        output=runner.run('cli-cold-build',['cargo','build','--release','--offline','--locked','-p','horizon-mixins-cli','--bin','horizon-mixins','--target-dir',str(work/'cli-target'),'-vv'],cwd=roots['mixins'],timeout=1800)
        witness=cli_witness(output);cli=work/'cli-target/release/horizon-mixins';cli_hash=sha(cli.read_bytes())
        report['gates']['cli_compile']={'passed':True,'witness':witness,'binary_sha256':cli_hash,'scope':'official tool only; no Game or consumer build'}
        main_bundle=work/'tconstruct.hnmod';common_bundle=work/'plutolib.hnmod'
        runner.run('main-package',[sys.executable,str(roots['main']/'tconstruct/tools/package_hnmod.py'),'--output',str(main_bundle)],cwd=roots['main'])
        evidence=prepare.verify_main_bundle(roots['main'],main_bundle);report['gates']['main_bundle']={'passed':True,**evidence}
        runner.run('common-package',[sys.executable,str(ROOT/'pluto-lib/tools/package_native.py'),str(common_bundle)])
        if sha(common_bundle.read_bytes())!=prepare.BUNDLE:raise ValueError('Common archive differs from successful exact bundle')
        report['gates']['common_bundle']={'passed':True,'sha256':prepare.BUNDLE,'bytes':common_bundle.stat().st_size}
        mods=work/'combined';evidence=prepare.compose(main_bundle,common_bundle,mods)
        snapshots[str(mods)]=common.checkout_seal(mods)
        report['gates']['combined_payload']={'passed':True,'headers':evidence['headers'],'members':evidence['members'],
          'members_seal_sha256':sha(json.dumps(evidence['member_sha256'],sort_keys=True).encode())}
        for p in [main_bundle,common_bundle]:artifact_seals[str(p)]=sha(p.read_bytes())
        # Tool cold-build is complete. From now on compiler execution is forbidden.
        real_cargo=Path(runner.run('cargo-path',['rustup','which','--toolchain','1.95.0','cargo']).strip()).resolve()
        real_rustc=Path(runner.run('rustc-path',['rustup','which','--toolchain','1.95.0','rustc']).strip()).resolve()
        toolchain=Path(env.get('RUSTUP_HOME',str(Path.home()/'.rustup'))).resolve()/'toolchains'
        if not real_cargo.is_relative_to(toolchain) or not real_rustc.is_relative_to(toolchain):raise ValueError('Unexpected toolchain binaries')
        bindir=work/'metadata-only';wrapper_log=work/'tool-argv.jsonl'
        prepare.write_wrappers(bindir,official/'Cargo.toml',real_cargo,real_rustc,wrapper_log)
        snapshots[str(bindir)]=common.checkout_seal(bindir)
        env.update(CARGO=str(bindir/'cargo'),RUSTC=str(bindir/'rustc'),PATH=str(bindir)+os.pathsep+env['PATH'])
        plan=work/'plan.json';result=attempt_plan(runner,cli,official,mods,plan)
        calls=[json.loads(line) for line in wrapper_log.read_text().splitlines()] if wrapper_log.exists() else []
        if not all(c['allowed'] for c in calls):raise ValueError('Unexpected tool invocation')
        if result['accepted'] and not any(c['kind']=='cargo' for c in calls):raise ValueError('Accepted plan lacks actual metadata witness')
        if any(not prepare.allowed(c['kind'],c['argv'],official/'Cargo.toml') for c in calls):raise ValueError('Wrapper argv mismatch')
        report['gates']['plan_diagnostic']={'passed':True,**result,'tool_calls':calls}
        report['plan_accepted']=result['accepted'];report['compatibility']=False if not result['accepted'] else None
        if result['accepted']:
            artifact_seals[str(plan)]=sha(plan.read_bytes())
            common.verify_plan(plan,common.read_inputs())
            runner.run('complete-guards',[sys.executable,str(ROOT/'pluto-lib/tools/check_native_plan.py'),str(plan),str(official)])
            report['gates']['complete_guards']={'passed':True,'groups':12}
        report['diagnostic_complete']=True
        if set(report['gates'])!=(REQUIRED|({'complete_guards'} if result['accepted'] else set())):raise ValueError('Missing required diagnostic gates')
    except BaseException as exc:common.record_error(report,exc)
    finally:
        try:
            common.assert_git_clean(ROOT);common.assert_full_seal(ROOT,original)
            if prepare.git_state(ROOT)!=original_git:raise ValueError('Original checkout refs/index/HEAD drift')
            for path,before in snapshots.items():common.assert_full_seal(Path(path),before)
            for path,before in git_snapshots.items():
                common.assert_git_clean(Path(path))
                if prepare.git_state(Path(path))!=before:raise ValueError('Fixed checkout refs/index/HEAD drift')
            for path,before in artifact_seals.items():
                if sha(Path(path).read_bytes())!=before:raise ValueError('Artifact drift')
            if cli is not None and sha(cli.read_bytes())!=cli_hash:raise ValueError('Official CLI drift')
            runner.guard();report['source_verified']=True
        except BaseException as exc:common.record_error(report,exc,'verification: ')
        report.update(minimum_free_bytes=runner.minimum_free,minimum_memory_bytes=getattr(runner,'minimum_memory',None),
          source_seal_sha256=sha(json.dumps(original,sort_keys=True).encode()),
          input_seals={p:sha(json.dumps(v,sort_keys=True).encode()) for p,v in snapshots.items()},
          artifact_seals=artifact_seals)

def main():
    report={'head':os.environ.get('GITHUB_SHA'),'gates':{},'phases':[],'error':None,'source_verified':False,
      'diagnostic_complete':False,'plan_accepted':None,'compatibility':None,'probe_completed':False,
      'scope':'Unchanged main + successful common, one official plan only. CLI tool cold-build only; no apply/Game/consumer build. A complete rejection is not compatibility success.'}
    try:execute(report)
    except BaseException as exc:common.record_error(report,exc)
    report['probe_completed']=report['diagnostic_complete'] and report['source_verified'] and report['error'] is None
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as out:out.write('## Unchanged main + common plan-only diagnostic\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:common.record_error(report,exc,'summary: ');report['probe_completed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['probe_completed']:raise SystemExit(1)

if __name__=='__main__':
    if len(sys.argv)==5 and sys.argv[1]=='--export':common.export(Path(sys.argv[2]),sys.argv[3],Path(sys.argv[4]))
    elif len(sys.argv)==1:main()
    else:raise SystemExit('Invalid controller arguments')
