"""Minimal Windows1.99 formatter probe using the unchanged successful source-CI phase."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys

HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod);return mod
base=load('fixed_source_phase',HERE.parent/'windows-source-ci/run.py')
validation=load('fixed_named_checks',HERE.parent/'windows-source-ci/validate.py')
probe=load('formatter_probe',HERE/'probe.py')
# This private imported phase retains exact barrier/kill/wait/bounded-read code.
# Only the reviewed per-phase elapsed limit differs from the40-second pure suite.
base.TIMEOUT=120
BRANCH='checkpoint/windows-formatter-199-ci-20261106'
REQUIRED={'pure','stat-pure','archive','version','damage','terrain-cell'}
def config():return json.loads((HERE/'inputs.json').read_bytes())
def verify_sources(c):
    before=base.snapshot(ROOT)
    for path,digest in c['source_sha256'].items():
        if before.get(path)!=digest:raise ValueError('Frozen input differs: '+path)
    return before
def preflight(c):
    if os.name!='nt' or sys.version_info[:2]!=(3,12):raise ValueError('Existing Windows x64 Python3.12 required')
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_bytes())
    if (os.environ.get('GITHUB_ACTIONS')!='true' or os.environ.get('GITHUB_REPOSITORY')!='sofia-gros/horizon-mods' or
        os.environ.get('GITHUB_EVENT_NAME')!='push' or os.environ.get('GITHUB_REF')!='refs/heads/'+BRANCH or event.get('repository',{}).get('private') is not False):raise ValueError('Exact public dedicated push only')
    verify_sources(c)
    paths=[ROOT,Path(os.environ['RUNNER_TEMP']),Path(ROOT.anchor),Path(sys.executable).parent]
    base.disk_guard(paths)
    if any(shutil.disk_usage(p).free<base.FLOOR+1024**3 for p in paths):raise ValueError('One GiB preparation reserve missing')
def artifact(c,path):
    expected={p:v['sha256'] for p,v in c['artifact']['files'].items()}
    if {p.name for p in path.iterdir()}!=set(expected):raise ValueError('Historical artifact file set differs')
    actual={}
    for p,want in c['artifact']['files'].items():
        raw=probe.fmt.bounded_file(path/p,want['bytes'])
        if len(raw)!=want['bytes'] or base.sha(raw)!=want['sha256']:raise ValueError('Historical artifact size/hash differs')
        actual[p]=base.sha(raw)
    probe.verify_plan(path/'plan.json');return actual
def phase_json(name,argv,work,env,receipt):
    raw=base.phase(name,argv,work,env,receipt)
    return json.loads(raw.decode('utf-8'))
def execute(receipt):
    c=config();preflight(c);before=verify_sources(c);receipt['head']=os.environ['GITHUB_SHA']
    receipt['source_before_sha256']=base.sha(json.dumps(before,sort_keys=True).encode())
    artifact_root=Path(os.environ['RUNNER_TEMP'])/'windows-formatter-plan'
    artifact_before=artifact(c,artifact_root)
    work=Path(os.environ['RUNNER_TEMP'])/('windows-formatter-'+os.environ['GITHUB_SHA']);work.mkdir()
    inputs_before=None
    env={k:v for k,v in os.environ.items() if k.upper() not in {'PLUTO_GUARD_RUSTFMT','PYTHONUTF8','PYTHONIOENCODING','PYTHONPATH','PYTHONHOME','PATH'}}
    env.update(TEMP=str(work),TMP=str(work),TMPDIR=str(work),PYTHONDONTWRITEBYTECODE='1',PYTHONUTF8='1',PYTHONIOENCODING='utf-8',
               PATH=os.pathsep.join([str(work/'formatter/bin'),str(Path(sys.executable).parent),str(Path(os.environ['SystemRoot'])/'System32')]),
               PLUTO_GUARD_RUSTFMT=str(work/'formatter/bin/rustfmt.exe'))
    try:
        raw=base.phase('pure',[sys.executable,str(HERE/'test_runner.py'),'-v'],work,env,receipt)
        receipt['gates']['pure']=validation.suite(raw,c['test_names'])
        raw=base.phase('stat-pure',[sys.executable,str(ROOT/'tconstruct/tests/test_windows_stat_compat.py'),'-v'],work,env,receipt)
        receipt['gates']['stat-pure']=validation.suite(raw,c['stat_test_names'])
        result=phase_json('prepare',[sys.executable,str(HERE/'probe.py'),'prepare',str(work)],work,env,receipt)
        admitted=probe.fmt.admit(work/'formatter',work/'rustfmt.tar.gz')
        if result.get('kind')!='preparation' or result.get('formatter')!=admitted or set(result.get('sources',{}))!=set(probe.source_map()):raise ValueError('Preparation witness differs')
        for p,h in probe.source_map().items():
            if base.sha(probe.fmt.bounded_file(work/'official'/p,probe.MAX_SOURCE))!=h:raise ValueError('Prepared official source drift')
        inputs_before={'formatter':base.snapshot(work/'formatter'),'official':base.snapshot(work/'official'),'archive':base.sha(probe.fmt.bounded_file(work/'rustfmt.tar.gz',probe.fmt.ARCHIVE_LIMIT))}
        receipt['gates']['archive']=result
        raw=base.phase('version',[str(work/'formatter/bin/rustfmt.exe'),'--version'],work,env,receipt)
        version=raw.decode('utf-8').strip();probe.fmt.validate_version(version)
        receipt['gates']['version']={'version':version,**admitted}
        for kind in ('damage','terrain-cell'):
            value=phase_json(kind,[sys.executable,str(HERE/'probe.py'),'regenerate',kind,str(work),str(artifact_root/'plan.json')],work,env,receipt)
            if (value.get('kind')!='regeneration' or value.get('guard')!=kind or value.get('passed') is not True or
                value.get('historical_plan_sha256')!=probe.PLAN_SHA or value.get('composition_verified') is not False or value.get('apply_executed') is not False or
                len(value.get('calls',[]))!=probe.COUNTS[kind]):raise ValueError('Regeneration witness differs')
            for row in value['calls']:
                if row!=probe.audit_call(row.get('application_name'),row.get('command_line'),work/'formatter/bin/rustfmt.exe'):raise ValueError('Actual formatter argv differs')
            receipt['gates'][kind]=value
        if set(receipt['gates'])!=REQUIRED:raise ValueError('Missing required gate')
        receipt['completed']=True
    finally:
        if base.snapshot(ROOT)!=before or artifact(c,artifact_root)!=artifact_before:raise ValueError('Source/artifact changed')
        if inputs_before is not None:
            after={'formatter':base.snapshot(work/'formatter'),'official':base.snapshot(work/'official'),'archive':base.sha(probe.fmt.bounded_file(work/'rustfmt.tar.gz',probe.fmt.ARCHIVE_LIMIT))}
            if after!=inputs_before:raise ValueError('Prepared input changed')
            receipt['prepared_input_seal']=base.sha(json.dumps(after,sort_keys=True).encode())
        base.disk_guard([ROOT,work,Path(ROOT.anchor),Path(sys.executable).parent]);receipt['verified']=True
def main():
    receipt={'schema':1,'completed':False,'verified':False,'passed':False,'gates':{},'phases':[],
             'scope':'Windows1.99 formatter regeneration only; historical full-plan helper comparison',
             'latest_composition_verified':False,'apply_executed':False,'game_build_executed':False}
    try:execute(receipt)
    except Exception as exc:receipt['error']=type(exc).__name__+': '+str(exc)
    receipt['passed']=receipt['completed'] and receipt['verified'] and 'error' not in receipt
    print('FINAL_RECEIPT '+json.dumps(receipt,sort_keys=True),flush=True)
    return 0 if receipt['passed'] else 1
if __name__=='__main__':
    if sys.argv[1:]==['--preflight']:
        preflight(config());target=Path(os.environ['RUNNER_TEMP'])/'windows-formatter-plan'
        if target.exists() or target.is_symlink():raise SystemExit('Artifact destination is not fresh')
    elif len(sys.argv)==1:sys.exit(main())
    else:raise SystemExit('Invalid controller arguments')
