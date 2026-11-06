#!/usr/bin/env python3
"""Latest mining formal-entry probe only; no apply, type proof or complete composition."""
import importlib.util,json,os,re,shlex,shutil,stat,sys
from pathlib import Path
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
baseline=load('latest_baseline',HERE.parent/'latest-main-common-ci/run.py')
common=baseline.common
prepare=baseline.prepare
safety=baseline.safety
observer=load('mining_plan_observer',HERE/'observe_plan.py')
probe=load('mining_probe_prepare',HERE/'prepare.py')
sha=common.sha
REPOSITORY='sofia-gros/horizon-mods'
BRANCH='checkpoint/plutolib-latest-mining-probe-ci-20261106'
SOURCE='b78ee288d7627e1be1c046db73541fab1d40c249'
OFFICIAL=baseline.OFFICIAL
RUNNER_TESTS=20
PREFIX='pluto-lib/experiments/latest-mining-probe-ci/'
INPUT_PATHS={PREFIX+p for p in ['run.py','prepare.py','test_runner.py','observe_plan.py','README.md']}
INPUT_PATHS|={'.github/workflows/latest-mining-probe-wip.yml','pluto-lib/experiments/latest-main-common-ci/inputs.json'}
INPUT_PATHS|={'pluto-lib/experiments/latest-mining-context/'+p for p in ['candidate.py','inputs.json','README.md','oracles.json','mining-context.rs','source-mining-context-native.rs','test_candidate.py','review/recipe-evidence.json','review/recipe-review.md']}
REQUIRED={'runner','candidate_sources','memory_admission','fixed_checkouts','cli_compile','small_probe','plan_diagnostic'}
Runner=baseline.Runner

def inputs():return json.loads((HERE/'inputs.json').read_text())
def validate(config):
    if (config['repository'],config['branch'],config['candidate_source'])!=(REPOSITORY,BRANCH,SOURCE):raise ValueError('Wrong source or trigger')
    if config['official']!=OFFICIAL:raise ValueError('Wrong official source or lock')
    if set(config['source_sha256'])!=INPUT_PATHS:raise ValueError('Input file-set changed')
    for p,digest in config['source_sha256'].items():
        path=ROOT/p
        if path.is_symlink() or sha(path.read_bytes())!=digest:raise ValueError('Changed source: '+p)
    baseline.validate(baseline.inputs())

def event_allowed(env,event):
    return (env.get('GITHUB_ACTIONS')=='true' and env.get('GITHUB_REPOSITORY')==REPOSITORY and
      env.get('GITHUB_EVENT_NAME')=='push' and env.get('GITHUB_REF')=='refs/heads/'+BRANCH and
      event.get('repository',{}).get('private') is False)

cli_witness=baseline.cli_witness

def attempt_plan(runner,cli,official,mods,plan):
    # One invocation only. Expected planner rejection is evidence, never a retry.
    try:
        output=runner.run('official-plan',[sys.executable,str(HERE/'observe_plan.py'),str(cli),str(official),str(mods),str(plan),str(runner.work/'plan-cpu.jsonl')],timeout=600)
    except RuntimeError:
        phase=runner.report['phases'][-1] if runner.report['phases'] else {}
        if phase.get('name')!='official-plan' or phase.get('exit_code')!=1 or phase.get('error')!='RuntimeError: official-plan exited 1':raise
        if plan.exists():raise ValueError('Rejected planner unexpectedly wrote a plan')
        return {'accepted':False,'diagnostic':prepare.diagnostic((runner.work/'official-plan.log').read_text())}
    value=json.loads(output)
    if value.get('status')!='ok' or not plan.is_file():raise ValueError('Planner success lacks actual plan')
    return {'accepted':True,'summary':value,'plan_sha256':sha(plan.read_bytes())}

def export_latest(source,revision,destination):
    if revision!=prepare.HORIZON:raise ValueError('Wrong latest official export')
    common.export(source,revision,destination)
    marker=destination/'.horizon-revision'
    if marker.exists() or marker.is_symlink():raise ValueError('Unexpected existing revision marker')
    with marker.open('x') as out:out.write(revision+'\n')

collect_tool_calls=baseline.collect_tool_calls

def execute(report):
    config=inputs();validate(config)
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if not event_allowed(os.environ,event):raise ValueError('Require exact public dedicated push branch; no local execution')
    if common.git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA'):raise ValueError('Wrong CI head')
    common.assert_git_clean(ROOT)
    original=common.checkout_seal(ROOT);original_git=prepare.git_state(ROOT)
    report['memory_observation']=safety.memory_observation()
    work=Path(os.environ['RUNNER_TEMP'])/('latest-mining-probe-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for k in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO','RUSTC','CARGO_NET_OFFLINE','GIT_DIR','GIT_INDEX_FILE','GIT_WORK_TREE']:
        env.pop(k,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'cli-target'),
      CARGO_BUILD_JOBS='1',CARGO_PROFILE_RELEASE_DEBUG='0',CARGO_INCREMENTAL='0',CARGO_TERM_COLOR='never',
      TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1',GIT_OPTIONAL_LOCKS='0')
    runner=Runner(work,env,report);snapshots={};git_snapshots={};artifact_seals={};cli=None;cli_hash=None;plan=None;wrapper_log=None;official=None;real_tools=None;wrappers_ready=False
    try:
        runner.guard(reserve=4*1024**3)
        report['gates']['memory_admission']={'passed':True,'available_bytes':runner.minimum_memory,'capacity_minimum':6*1024**3,'admission_available_minimum':2*1024**3}
        work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
        out=runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v'])
        common.validate_python_checks(out,RUNNER_TESTS);report['gates']['runner']={'passed':True,'tests':RUNNER_TESTS}
        candidate_output=runner.run('candidate-source-checks',[sys.executable,str(HERE.parent/'latest-mining-context/test_candidate.py'),'-v'])
        common.validate_python_checks(candidate_output,16);report['gates']['candidate_sources']={'passed':True,'tests':16,'candidate_source':SOURCE}
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal','--component','rustfmt'],timeout=600)
        roots={}
        pins=OFFICIAL
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
            snapshots[str(root)]=common.checkout_seal(root);git_snapshots[str(root)]=prepare.git_state(root)
        contract_sources=prepare.verify_official(roots)
        report['gates']['fixed_checkouts']={'passed':True,'official':OFFICIAL,'contract_sources':contract_sources}
        official=work/'horizon'
        runner.run('horizon-export',[sys.executable,str(HERE/'run.py'),'--export',str(roots['horizon']),prepare.HORIZON,str(official)])
        report['gates']['fixed_checkouts']['candidate_official_sources']=probe.candidate.verify_official(official)
        snapshots[str(official)]=common.checkout_seal(official)
        runner.run('cli-locked-fetch',['cargo','fetch','--locked','--manifest-path',str(roots['mixins']/'Cargo.toml')],timeout=900)
        env['CARGO_NET_OFFLINE']='true'
        output=runner.run('cli-cold-build',['cargo','build','--release','--offline','--locked','-p','horizon-mixins-cli','--bin','horizon-mixins','--target-dir',str(work/'cli-target'),'-vv'],cwd=roots['mixins'],timeout=1800)
        witness=cli_witness(output);cli=work/'cli-target/release/horizon-mixins';cli_hash=sha(cli.read_bytes())
        report['gates']['cli_compile']={'passed':True,'witness':witness,'binary_sha256':cli_hash,'scope':'official tool only; no Game or consumer build'}
        mods=work/'mining-probe'
        evidence=probe.write_probe(mods)
        snapshots[str(mods)]=common.checkout_seal(mods)
        report['gates']['small_probe']={'passed':True,**evidence}
        # Tool cold-build is complete. From now on compiler execution is forbidden.
        real_cargo=Path(runner.run('cargo-path',['rustup','which','--toolchain','1.95.0','cargo']).strip()).resolve()
        real_rustc=Path(runner.run('rustc-path',['rustup','which','--toolchain','1.95.0','rustc']).strip()).resolve()
        toolchain=Path(env.get('RUSTUP_HOME',str(Path.home()/'.rustup'))).resolve()/'toolchains'
        if not real_cargo.is_relative_to(toolchain) or not real_rustc.is_relative_to(toolchain):raise ValueError('Unexpected toolchain binaries')
        bindir=work/'metadata-only';wrapper_log=work/'tool-argv.jsonl'
        prepare.write_wrappers(bindir,official/'Cargo.toml',real_cargo,real_rustc,wrapper_log)
        real_tools={'cargo':str(real_cargo),'rustc':str(real_rustc)};wrappers_ready=True
        snapshots[str(bindir)]=common.checkout_seal(bindir)
        env.update(CARGO=str(bindir/'cargo'),RUSTC=str(bindir/'rustc'),PATH=str(bindir)+os.pathsep+env['PATH'])
        plan=work/'plan.json';result=attempt_plan(runner,cli,official,mods,plan)
        calls=[json.loads(line) for line in wrapper_log.read_text().splitlines()] if wrapper_log.exists() else []
        if not all(c['allowed'] for c in calls):raise ValueError('Unexpected tool invocation')
        if result['accepted'] and not any(c['kind']=='cargo' for c in calls):raise ValueError('Accepted plan lacks actual metadata witness')
        if any(not prepare.allowed(c['kind'],c['argv'],official/'Cargo.toml') for c in calls):raise ValueError('Wrapper argv mismatch')
        report['gates']['plan_diagnostic']={'passed':True,**result,'tool_calls':calls}
        report['plan_accepted']=result['accepted'];report['compatibility']=None
        if result['accepted']:
            artifact_seals[str(plan)]=sha(plan.read_bytes())
            report['formal_binding']=probe.verify_plan(plan)
        report['diagnostic_complete']=True
        if set(report['gates'])!=REQUIRED:raise ValueError('Missing required diagnostic gates')
    except BaseException as exc:common.record_error(report,exc)
    finally:
        try:collect_tool_calls(report,wrapper_log,official,real_tools,wrappers_ready)
        except BaseException as exc:common.record_error(report,exc,'tool-argv verification: ')
        try:
            cpu_path=work/'plan-cpu.jsonl'
            observation=observer.collect(cpu_path)
            if observation['present']:observation['raw_sha256']=sha(cpu_path.read_bytes())
            phase=next((p for p in report['phases'] if p['name']=='official-plan'),None)
            observation['observer_exit_code']=phase['exit_code'] if phase else None
            report['plan_cpu_observation']=observation
            if phase and phase['exit_code'] in (0,1) and (not observation['complete'] or observation.get('cli_exit_code')!=phase['exit_code']):
                raise ValueError('Observer/CLI completion mismatch')
        except BaseException as exc:common.record_error(report,exc,'CPU observation verification: ')
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
      'native_type_verified':False,'full_composition_tested':False,'complete_guards':'not run: reduced probe is not a complete candidate',
      'scope':'Exact candidate native sources, one internal module and one HEAD only. Formal target/owner/binding probe, not full646 compatibility or type proof. CLI cold-build only; no apply/Game/consumer build; one600-second plan.'}
    try:execute(report)
    except BaseException as exc:common.record_error(report,exc)
    if report['diagnostic_complete'] and (set(report['gates'])!=REQUIRED or type(report['plan_accepted']) is not bool or (report['plan_accepted'] and report.get('formal_binding',{}).get('passed') is not True)):
        common.record_error(report,ValueError('Terminal required gates or selected-plan proof missing'))
    report['probe_completed']=report['diagnostic_complete'] and report['source_verified'] and report['error'] is None
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as out:out.write('## Latest mining formal-entry probe\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:common.record_error(report,exc,'summary: ');report['probe_completed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['probe_completed']:raise SystemExit(1)

if __name__=='__main__':
    if len(sys.argv)==5 and sys.argv[1]=='--export':export_latest(Path(sys.argv[2]),sys.argv[3],Path(sys.argv[4]))
    elif len(sys.argv)==1:main()
    else:raise SystemExit('Invalid controller arguments')
