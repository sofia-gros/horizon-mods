#!/usr/bin/env python3
"""Exact integration package diagnostic; independent old guards, no apply/type success."""
import importlib.util,json,os,re,sys
from pathlib import Path
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
def load(name,path):
    s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
baseline=load('full_mining_baseline',HERE.parent/'latest-main-common-ci/run.py')
package=load('playtest_prepare',HERE/'prepare.py')
guard_once=load('playtest_guard_once',HERE/'guard_once.py')
observer=load('full_mining_observer',HERE/'observe_plan.py')
evidence=load('public_plan_evidence',HERE/'evidence.py')
common=baseline.common;prepare=package;safety=baseline.safety;sha=common.sha
Runner=baseline.Runner
cli_witness=baseline.cli_witness
collect_tool_calls=baseline.collect_tool_calls
REPOSITORY=baseline.REPOSITORY
OFFICIAL={k:dict(v) for k,v in baseline.OFFICIAL.items()}
OFFICIAL['horizon']['revision']=package.HORIZON
COLLECTOR='8cfc1247a0c18d5c93bdd0c185fe1b2e797f2937'
BRANCH='checkpoint/tinkers-playtest-complete-plan-ci-20261106'
RUNNER_TESTS=51
PREFIX='pluto-lib/experiments/playtest-plan-ci/'
COLLECTOR_PATH=PREFIX+'terrain-witness/'
OWN_INPUTS={PREFIX+n for n in ('run.py','prepare.py','guard_once.py','observe_plan.py','test_runner.py','README.md','evidence.py')} | {COLLECTOR_PATH+n for n in ('collect.py','contract.json','test_collect.py','README.md','source-delta-41f.json')} | {'.github/workflows/playtest-complete-plan-wip.yml'}
REQUIRED={'runner','memory_admission','fixed_checkouts','package_headers','cli_compile','tconstruct_bundle','pluto_bundle','combined_payload','collector_sources','plan_diagnostic'}

def inputs():return json.loads((HERE/'inputs.json').read_text())
def validate(config):
    if (config['repository'],config['branch'],config['collector_source'])!=(REPOSITORY,BRANCH,COLLECTOR):raise ValueError('Wrong fixed source or trigger')
    if config['official']!=OFFICIAL:raise ValueError('Wrong official source or lock')
    frozen=config['integration']
    if config.get('ready_for_reviewed_execution') is not True or any(not isinstance(frozen.get(k),str) or not re.fullmatch('[0-9a-f]{40}',frozen[k]) for k in ('head','tree')):
        raise ValueError('Integration cutoff not frozen and reviewed; source preparation only')
    required={'tconstruct/compatibility.json','tconstruct/mixins/tconstruct/horizon-mixin.toml','tconstruct/tools/package_hnmod.py','pluto-lib/horizon-native.json','pluto-lib/tools/package_native.py'} | {'pluto-lib/tools/'+n for n in guard_once.GUARDS} | {'pluto-lib/mixins/'+n+'/horizon-mixin.toml' for n in package.COMPONENTS}
    if not required.issubset(frozen.get('source_sha256',{})):raise ValueError('Missing exact integration package/guard input pins')
    if set(config['source_sha256'])!=OWN_INPUTS:raise ValueError('Controller source pin set changed')
    for name,digest in config['source_sha256'].items():
        path=ROOT/name
        if path.is_symlink() or sha(path.read_bytes())!=digest:raise ValueError('Changed controller source: '+name)
    baseline.validate(baseline.inputs())

def export_latest(source,revision,destination):
    if revision!=package.HORIZON:raise ValueError('Wrong integration official export')
    common.export(source,revision,destination)
    marker=destination/'.horizon-revision'
    if marker.exists() or marker.is_symlink():raise ValueError('Existing revision marker')
    with marker.open('x') as out:out.write(revision+'\n')


def event_allowed(env,event):
    return (env.get('GITHUB_ACTIONS')=='true' and env.get('GITHUB_REPOSITORY')==REPOSITORY and env.get('GITHUB_EVENT_NAME')=='push' and env.get('GITHUB_REF')=='refs/heads/'+BRANCH and event.get('repository',{}).get('private') is False)

def required_gates(report):
    return REQUIRED | ({'full_plan_contract','terrain_witness','complete_guards'} if report.get('plan_accepted') else set())

def attempt_terrain(runner,plan,official,integration):
    args=[sys.executable,str(ROOT/COLLECTOR_PATH/'collect.py'),'--plan',str(plan),'--official-root',str(official),'--mod-root',str(integration)]
    code=0
    try:output=runner.run('terrain-witness',args)
    except RuntimeError:
        phase=runner.report['phases'][-1] if runner.report['phases'] else {}
        if phase.get('name')!='terrain-witness' or phase.get('exit_code')!=1 or phase.get('error')!='RuntimeError: terrain-witness exited 1':raise
        code=1;output=(runner.work/'terrain-witness.log').read_text()
    if len(output.encode())>128*1024:raise ValueError('Oversized collector output')
    value=json.loads(output);digest=sha(package.plan_bytes(plan))
    if code:
        if set(value)!={'status','compatibility','plan_sha256','error'} or value['status']!='rejected' or value['compatibility'] is not None or value['plan_sha256']!=digest or not isinstance(value['error'],str):raise ValueError('Malformed collector refusal')
    elif value.get('status')!='observed_only' or value.get('compatibility') is not None or value.get('native_executed') is not False or value.get('plan',{}).get('sha256')!=digest or value.get('contract_sha256')!=sha((ROOT/COLLECTOR_PATH/'contract.json').read_bytes()):raise ValueError('Wrong selected terrain witness')
    return {'passed':code==0,'exit_code':code,'result':value,'scope':'Observation only. Normal refusal retained before all12 independent guards.'}

def attempt_guards(runner,plan,official,integration):
    result={'passed':False,'completed':False,'groups_configured':12,'apply_allowed':False,
        'records':[{'script':script,'reached':False,'passed':False,'exit_code':None} for script in guard_once.GUARDS]}
    runner.report['gates']['complete_guards']=result
    for i,script in enumerate(guard_once.GUARDS):
        row=result['records'][i];row.update(attempted=True,reached=None)
        try:
            out=runner.run('guard-'+str(i+1),[sys.executable,str(HERE/'guard_once.py'),str(integration),str(plan),str(official),script,str(runner.work/('guard-streams-'+str(i+1)))],timeout=180)
            if len(out.encode())>96*1024:raise ValueError('Oversized guard envelope')
            record=guard_once.validate_record(json.loads(out),script)
            row.update(record);row['reached']=True;row['passed']=record['exit_code']==0
            if not row['passed']:runner.report['compatibility']=False
        except BaseException as exc:
            row['error']=type(exc).__name__+': '+str(exc)
            for remaining in result['records'][i+1:]:remaining['reason']='not reached after safety/transport/malformed-output failure'
            raise
    result['completed']=True;result['passed']=all(row['passed'] for row in result['records'])
    return result


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
    return {'accepted':True,'summary':value,'plan_sha256':sha(package.plan_bytes(plan)),'plan_bytes':len(package.plan_bytes(plan))}

def execute(report):
    config=inputs();validate(config)
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if not event_allowed(os.environ,event):raise ValueError('Require exact public dedicated push branch; no local execution')
    if common.git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA'):raise ValueError('Wrong CI head')
    common.assert_git_clean(ROOT)
    original=common.checkout_seal(ROOT);original_git=prepare.git_state(ROOT)
    report['memory_observation']=safety.memory_observation()
    work=Path(os.environ['RUNNER_TEMP'])/('playtest-complete-plan-'+os.environ['GITHUB_SHA'])
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
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal','--component','rustfmt'],timeout=600)
        roots={}
        pins={**OFFICIAL,'integration':{'url':'https://github.com/sofia-gros/horizon-mods.git','revision':config['integration']['head']}}
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
            if name=='integration' and common.git(root,'rev-parse','HEAD^{tree}')!=config['integration']['tree']:
                raise ValueError('Wrong integration tree')
            snapshots[str(root)]=common.checkout_seal(root);git_snapshots[str(root)]=prepare.git_state(root)
        integration=roots['integration']
        for name,digest in config['integration']['source_sha256'].items():
            if sha(package.regular(integration/name,4*1024**2))!=digest:raise ValueError('Integration source pin drift: '+name)
        contract_sources=package.verify_official(roots,config['official_contract'])
        report['gates']['fixed_checkouts']={'passed':True,'integration':config['integration'],'official':OFFICIAL,'contract_sources':contract_sources}
        report['gates']['package_headers']={'passed':True,**package.validate_headers(integration)}
        env['PLUTO_WITNESS_MOD_ROOT']=str(integration)
        out=runner.run('collector-source-checks',[sys.executable,str(ROOT/COLLECTOR_PATH/'test_collect.py'),'-v'])
        common.validate_python_checks(out,22);report['gates']['collector_sources']={'passed':True,'tests':22,'source':COLLECTOR}
        official=work/'horizon'
        runner.run('horizon-export',[sys.executable,str(HERE/'run.py'),'--export',str(roots['horizon']),package.HORIZON,str(official)])
        snapshots[str(official)]=common.checkout_seal(official)
        runner.run('cli-locked-fetch',['cargo','fetch','--locked','--manifest-path',str(roots['mixins']/'Cargo.toml')],timeout=900)
        env['CARGO_NET_OFFLINE']='true'
        output=runner.run('cli-cold-build',['cargo','build','--release','--offline','--locked','-p','horizon-mixins-cli','--bin','horizon-mixins','--target-dir',str(work/'cli-target'),'-vv'],cwd=roots['mixins'],timeout=1800)
        witness=cli_witness(output);cli=work/'cli-target/release/horizon-mixins';cli_hash=sha(cli.read_bytes())
        report['gates']['cli_compile']={'passed':True,'witness':witness,'binary_sha256':cli_hash,'scope':'official tool only; no Game or consumer build'}
        main_bundle=work/'tconstruct.hnmod';common_bundle=work/'plutolib.hnmod'
        runner.run('tconstruct-package',[sys.executable,str(integration/'tconstruct/tools/package_hnmod.py'),'--output',str(main_bundle)],cwd=integration)
        tc_evidence=package.verify_tconstruct(integration,main_bundle)
        # Nested support map is fully checked, but only its count/hash is retained in console.
        tc_evidence['support'].pop('member_sha256')
        report['gates']['tconstruct_bundle']={'passed':True,**tc_evidence}
        runner.run('pluto-package',[sys.executable,str(integration/'pluto-lib/tools/package_native.py'),str(common_bundle)],cwd=integration)
        report['gates']['pluto_bundle']={'passed':True,**package.verify_pluto(integration,common_bundle)}
        for p in [main_bundle,common_bundle]:artifact_seals[str(p)]=sha(package.regular(p,package.MAX_ARCHIVE))
        mods=work/'combined-mods';combined=package.compose(main_bundle,common_bundle,mods)
        snapshots[str(mods)]=common.checkout_seal(mods)
        report['gates']['combined_payload']={'passed':True,**combined}
        # No candidate transforms, repinning, truncation or source replacement here.
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
        report['plan_accepted']=result['accepted'];report['compatibility']=False if not result['accepted'] else None
        if result['accepted']:
            # Whole plan hash/size are already retained before any selected validator.
            artifact_seals[str(plan)]=sha(package.plan_bytes(plan))
            report['gates']['full_plan_contract']=package.verify_plan(plan,mods,config['required_plan_inputs'])
            terrain=attempt_terrain(runner,plan,official,integration)
            report['gates']['terrain_witness']=terrain
            guards=attempt_guards(runner,plan,official,integration)
            report['gates']['complete_guards']=guards
            if not guards['passed']:report['compatibility']=False
        report['diagnostic_complete']=True
        if set(report['gates'])!=required_gates(report):raise ValueError('Missing required diagnostic gates')
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
      'native_type_verified':False,'apply_executed':False,
      'scope':'Exact integration checkout with both real packages. One complete plan only, bounded terrain observation and all12 independent unchanged guards. No apply/Game/consumer build or compatibility-success claim.'}
    try:execute(report)
    except BaseException as exc:common.record_error(report,exc)
    if report['diagnostic_complete'] and (set(report['gates'])!=required_gates(report) or type(report['plan_accepted']) is not bool or report['compatibility'] is True):
        common.record_error(report,ValueError('Terminal gates or diagnostic scope mismatch'))
    report['probe_completed']=report['diagnostic_complete'] and report['source_verified'] and report['error'] is None
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as out:out.write('## Integrated playtest plan-only diagnostic\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:common.record_error(report,exc,'summary: ');report['probe_completed']=False
    try:
        # Reuse the existing guards after all owned subprocesses and source seals finish.
        temp=Path(os.environ['RUNNER_TEMP']);plan,_=evidence.paths(temp,report['head'])
        guard_runner=Runner(plan.parent,{'CARGO_HOME':str(plan.parent/'cargo-home'),'RUSTUP_HOME':os.environ.get('RUSTUP_HOME',str(Path.home()/'.rustup'))})
        output,_=evidence.persist(report,temp,guard_runner.guard)
        evidence.verify_saved(output)
        with open(os.environ['GITHUB_OUTPUT'],'a') as out:out.write('evidence_ready=true\n')
    except BaseException as exc:
        common.record_error(report,exc,'artifact preservation: ');report['probe_completed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['probe_completed']:raise SystemExit(1)

if __name__=='__main__':
    if len(sys.argv)==5 and sys.argv[1]=='--export':export_latest(Path(sys.argv[2]),sys.argv[3],Path(sys.argv[4]))
    elif len(sys.argv)==1:main()
    else:raise SystemExit('Invalid controller arguments')
