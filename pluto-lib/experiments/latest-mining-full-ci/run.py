#!/usr/bin/env python3
"""Full frozen mining candidate plan diagnostic, never apply/type/compatibility success."""
import importlib.util,json,os,sys
from pathlib import Path
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
def load(name,path):
    s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
baseline=load('full_mining_baseline',HERE.parent/'latest-main-common-ci/run.py')
small=load('full_mining_small',HERE.parent/'latest-mining-probe-ci/run.py')
full=load('full_mining_prepare',HERE/'prepare.py')
observer=load('full_mining_observer',HERE/'observe_plan.py')
common=baseline.common;prepare=baseline.prepare;safety=baseline.safety;sha=common.sha
Runner=baseline.Runner
cli_witness=baseline.cli_witness
collect_tool_calls=baseline.collect_tool_calls
export_latest=baseline.export_latest
attempt_guards=baseline.attempt_guards
REPOSITORY=baseline.REPOSITORY
MAIN=baseline.MAIN
OFFICIAL=baseline.OFFICIAL
SOURCE='b78ee288d7627e1be1c046db73541fab1d40c249'
COLLECTOR='5b87b28da1e707ca68367ca454bd3defb0256402'
BRANCH='checkpoint/plutolib-latest-mining-full-plan-ci-20261106'
RUNNER_TESTS=24
PREFIX='pluto-lib/experiments/latest-mining-full-ci/'
COLLECTOR_PATH='pluto-lib/experiments/latest-terrain-witness/'
INPUT_PATHS={PREFIX+n for n in ['run.py','prepare.py','observe_plan.py','test_runner.py','README.md']}
INPUT_PATHS|={COLLECTOR_PATH+n for n in ['collect.py','contract.json','test_collect.py','README.md']}
INPUT_PATHS|={'.github/workflows/latest-mining-full-plan-wip.yml','pluto-lib/experiments/latest-mining-probe-ci/inputs.json'}
REQUIRED={'runner','memory_admission','fixed_checkouts','cli_compile','main_bundle','common_bundle','combined_payload','revision_only_candidate','mining_candidate','candidate_sources','collector_sources','plan_diagnostic'}

def inputs():return json.loads((HERE/'inputs.json').read_text())
def validate(config):
    if (config['repository'],config['branch'],config['candidate_source'],config['collector_source'])!=(REPOSITORY,BRANCH,SOURCE,COLLECTOR):raise ValueError('Wrong fixed source or trigger')
    if config['official']!=OFFICIAL:raise ValueError('Wrong official source or lock')
    if set(config['source_sha256'])!=INPUT_PATHS:raise ValueError('Input file-set changed')
    for name,digest in config['source_sha256'].items():
        path=ROOT/name
        if path.is_symlink() or sha(path.read_bytes())!=digest:raise ValueError('Changed source: '+name)
    small.validate(small.inputs())

def event_allowed(env,event):
    return (env.get('GITHUB_ACTIONS')=='true' and env.get('GITHUB_REPOSITORY')==REPOSITORY and env.get('GITHUB_EVENT_NAME')=='push' and env.get('GITHUB_REF')=='refs/heads/'+BRANCH and event.get('repository',{}).get('private') is False)

def required_gates(report):
    gates=set(REQUIRED)
    if report.get('plan_accepted'):
        gates|={'full_plan_contract','terrain_witness'}
        if report.get('gates',{}).get('terrain_witness',{}).get('passed'):gates.add('complete_guards')
    return gates

def attempt_terrain(runner,plan,official):
    args=[sys.executable,str(ROOT/COLLECTOR_PATH/'collect.py'),'--plan',str(plan),'--official-root',str(official),'--mod-root',str(ROOT)]
    try:output=runner.run('terrain-witness',args)
    except RuntimeError:
        phase=runner.report['phases'][-1] if runner.report['phases'] else {}
        if phase.get('name')!='terrain-witness' or phase.get('exit_code')!=1 or phase.get('error')!='RuntimeError: terrain-witness exited 1':raise
        output=(runner.work/'terrain-witness.log').read_text()
        if len(output.encode())>128*1024:raise ValueError('Oversized collector rejection')
        value=json.loads(output)
        if set(value)!={'status','compatibility','plan_sha256','error'} or value['status']!='rejected' or value['compatibility'] is not None or value['plan_sha256']!=sha(full.plan_bytes(plan)) or not isinstance(value['error'],str):raise ValueError('Wrong collector refusal')
        return {'passed':False,'exit_code':1,'result':value,'later_guards':'not run after collector refusal'}
    if len(output.encode())>128*1024:raise ValueError('Oversized terrain witness')
    value=json.loads(output)
    if value.get('status')!='observed_only' or value.get('compatibility') is not None or value.get('native_executed') is not False or value.get('plan',{}).get('sha256')!=sha(full.plan_bytes(plan)) or value.get('contract_sha256')!=sha((ROOT/COLLECTOR_PATH/'contract.json').read_bytes()):raise ValueError('Wrong selected terrain witness')
    return {'passed':True,'exit_code':0,'result':value,'scope':'Observed selected IR only; old guards unchanged.'}

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
    return {'accepted':True,'summary':value,'plan_sha256':sha(full.plan_bytes(plan)),'plan_bytes':len(full.plan_bytes(plan))}

def execute(report):
    config=inputs();validate(config)
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if not event_allowed(os.environ,event):raise ValueError('Require exact public dedicated push branch; no local execution')
    if common.git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA'):raise ValueError('Wrong CI head')
    common.assert_git_clean(ROOT)
    original=common.checkout_seal(ROOT);original_git=prepare.git_state(ROOT)
    report['memory_observation']=safety.memory_observation()
    work=Path(os.environ['RUNNER_TEMP'])/('latest-mining-full-plan-'+os.environ['GITHUB_SHA'])
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
        out=runner.run('candidate-source-checks',[sys.executable,str(HERE.parent/'latest-mining-context/test_candidate.py'),'-v'])
        common.validate_python_checks(out,16);report['gates']['candidate_sources']={'passed':True,'tests':16,'source':SOURCE}
        out=runner.run('collector-source-checks',[sys.executable,str(ROOT/COLLECTOR_PATH/'test_collect.py'),'-v'])
        common.validate_python_checks(out,20);report['gates']['collector_sources']={'passed':True,'tests':20,'source':COLLECTOR}
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
        contract_sources=prepare.verify_official(roots)
        report['gates']['fixed_checkouts']={'passed':True,'main':MAIN,'official':OFFICIAL,'contract_sources':contract_sources,'ir_contract_sources':small.probe.verify_ir_sources(roots['mixins'])}
        official=work/'horizon'
        runner.run('horizon-export',[sys.executable,str(HERE/'run.py'),'--export',str(roots['horizon']),prepare.HORIZON,str(official)])
        report['gates']['fixed_checkouts']['candidate_official_sources']=full.candidate.verify_official(official)
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
        original_mods=work/'combined-original';evidence=prepare.compose(main_bundle,common_bundle,original_mods)
        snapshots[str(original_mods)]=common.checkout_seal(original_mods)
        revised=work/'revision-only-candidate'
        candidate=prepare.revise(original_mods,revised,evidence['member_sha256'])
        snapshots[str(revised)]=common.checkout_seal(revised)
        mods=work/'mining-candidate'
        report['gates']['mining_candidate']=full.write_candidate(original_mods,revised,mods,prepare.contract()['manifest_changes'])
        snapshots[str(mods)]=common.checkout_seal(mods)
        report['gates']['revision_only_candidate']={'passed':True,**candidate}
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
            artifact_seals[str(plan)]=sha(full.plan_bytes(plan))
            report['gates']['full_plan_contract']=full.verify_plan(plan,mods)
            terrain=attempt_terrain(runner,plan,official)
            report['gates']['terrain_witness']=terrain
            if terrain['passed']:
                guards=attempt_guards(runner,plan,official)
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
      'scope':'Original main/common archives plus exact b78 four-path mining/revision candidate. One full646 plan only; accepted plan witness then unchanged old12 guards. No apply/Game/consumer build or compatibility-success claim.'}
    try:execute(report)
    except BaseException as exc:common.record_error(report,exc)
    if report['diagnostic_complete'] and (set(report['gates'])!=required_gates(report) or type(report['plan_accepted']) is not bool or report['compatibility'] is True):
        common.record_error(report,ValueError('Terminal gates or diagnostic scope mismatch'))
    report['probe_completed']=report['diagnostic_complete'] and report['source_verified'] and report['error'] is None
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as out:out.write('## Latest mining complete-candidate diagnostic\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:common.record_error(report,exc,'summary: ');report['probe_completed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['probe_completed']:raise SystemExit(1)

if __name__=='__main__':
    if len(sys.argv)==5 and sys.argv[1]=='--export':export_latest(Path(sys.argv[2]),sys.argv[3],Path(sys.argv[4]))
    elif len(sys.argv)==1:main()
    else:raise SystemExit('Invalid controller arguments')
