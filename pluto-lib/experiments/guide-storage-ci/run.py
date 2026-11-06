#!/usr/bin/env python3
"""Isolated official-host storage type experiment; no installed Mod or UI proof."""
import contextlib
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PROTOTYPE = ROOT/'pluto-lib/experiments/guide-response-phase/native-patch'
BRANCH = 'checkpoint/guide-storage-type-ci-20261106'
REPOSITORY = 'sofia-gros/horizon-mods'
HORIZON = '624c4f6265c02296fbe8a80f029dd0d987fb94c0'
HORIZON_URL = 'https://github.com/horizon-rust/horizon.git'
LOCK = '463913123985e344f2889e5beae65982773e6dbfacdcac7a916bcec522a5bd31'
RUNNER_TESTS = 15
REQUIRED = {'runner', 'official_source', 'prototype_source', 'experiment_delta', 'metadata', 'host_type'}
LOG_LIMIT = 128*1024**2

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    return module

# One already reviewed process/disk/source utility, not a new runtime registry.
base = load('guide_identity_guard', ROOT/'tconstruct/tests/guide-identity-ci/run.py')
sha, seal, FLOOR = base.sha, base.seal, base.FLOOR

def config():
    value = json.loads((HERE/'inputs.json').read_text())
    if value['official'] != {'url': HORIZON_URL, 'revision': HORIZON, 'lock_sha256': LOCK}:
        raise ValueError('Wrong official-only baseline')
    for name, digest in value['source_sha256'].items():
        p = ROOT/name
        if p.is_symlink() or sha(p.read_bytes()) != digest: raise ValueError('Frozen source differs: '+name)
    if value['branch'] != BRANCH or value['repository'] != REPOSITORY: raise ValueError('Wrong publication scope')
    return value

class Runner(base.Runner):
    def run(self, name, command, cwd=None, timeout=120, separate_stdout=False):
        self.guard(); path = self.work/(name+'.log')
        print('PHASE', name, json.dumps(command), flush=True)
        started = time.monotonic()
        with path.open('x') as log, contextlib.ExitStack() as stack:
            err_path = self.work/(name+'.stderr.log') if separate_stdout else None
            err = stack.enter_context(err_path.open('x')) if err_path else subprocess.STDOUT
            proc = subprocess.Popen(command, cwd=cwd or ROOT, env=self.env, stdout=log,
                                    stderr=err, start_new_session=True)
            try:
                while proc.poll() is None:
                    self.guard()
                    size = path.stat().st_size + (err_path.stat().st_size if err_path else 0)
                    if size > LOG_LIMIT: raise RuntimeError('Bounded log limit reached')
                    if time.monotonic()-started > timeout: raise TimeoutError(name)
                    time.sleep(0.2)
            finally:
                if proc.poll() is None:
                    try: os.killpg(proc.pid, signal.SIGTERM)
                    except ProcessLookupError: pass
                    deadline = time.monotonic()+1
                    while proc.poll() is None and time.monotonic() < deadline: time.sleep(0.05)
                try: os.killpg(proc.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                proc.wait(timeout=2); base.ensure_group_gone(proc.pid, name)
                # Native -vv logs are retained/hash-bound in the runner, not
                # dumped into an unbounded Actions response. On failure show a
                # bounded diagnostic tail; success has exact selected evidence.
                raw = path.read_bytes()
                stderr = err_path.read_bytes() if err_path else b''
                print('LOG', name, json.dumps({'sha256':sha(raw), 'bytes':len(raw)}), flush=True)
                if err_path:
                    print('STDERR_LOG', name, json.dumps({'sha256':sha(stderr), 'bytes':len(stderr)}), flush=True)
                if proc.returncode:
                    print(raw[-65536:].decode(errors='replace'), flush=True)
                    if stderr: print(stderr[-65536:].decode(errors='replace'), flush=True)
        self.guard()
        if len(raw)+len(stderr) > LOG_LIMIT: raise RuntimeError('Bounded log limit reached')
        print('TERMINAL', name, proc.returncode, flush=True)
        if proc.returncode: raise RuntimeError(name+' exited '+str(proc.returncode))
        return raw.decode()

def validate_event(env, event, head):
    if (env.get('GITHUB_ACTIONS') != 'true' or env.get('GITHUB_REPOSITORY') != REPOSITORY or
        env.get('GITHUB_EVENT_NAME') != 'push' or env.get('GITHUB_REF') != 'refs/heads/'+BRANCH or
        env.get('GITHUB_SHA') != head or event.get('repository', {}).get('private') is not False or
        event.get('repository', {}).get('full_name') != REPOSITORY):
        raise ValueError('Require exact authorized public push branch')

def python_pass(log, count):
    if (re.findall(r'^Ran (\d+) tests in .+$', log, re.M) != [str(count)] or
        len(re.findall(r'^test_\S+ .* \.\.\. ok$', log, re.M)) != count or
        re.findall(r'^OK$', log, re.M) != ['OK']): raise ValueError('Missing or skipped Python checks')

def independent_roots(source, destination):
    a, b = source.resolve(), destination.resolve()
    if a.is_relative_to(b) or b.is_relative_to(a):
        raise ValueError('Source and destination must be disjoint roots')

def export(checkout, destination):
    independent_roots(checkout, destination)
    portable = load('portable_git', ROOT/'tconstruct/tools/portable_archive.py')
    destination.mkdir()
    proc = subprocess.Popen(['git','-C',str(checkout),'archive','--format=tar',HORIZON], stdout=subprocess.PIPE)
    try:
        portable.extract_git_tar(proc.stdout, destination)
        if proc.wait(timeout=60): raise ValueError('Official export failed')
    finally:
        proc.stdout.close()
        if proc.poll() is None: proc.terminate(); proc.wait(timeout=2)

def verify_delta(pristine, experiment, expected):
    old, new = seal(pristine), seal(experiment)
    if set(old)-set(new): raise ValueError('Original paths missing from experiment')
    changes = {p for p in old.keys() | new.keys() if old.get(p) != new.get(p)}
    if changes != set(expected): raise ValueError('Unexpected host delta: '+repr(sorted(changes)))
    for name, digest in expected.items():
        entry = new[name]
        if entry[:1] != ['file'] or entry[2] != digest: raise ValueError('Wrong experiment bytes: '+name)
    if (pristine/'Cargo.lock').read_bytes() != (experiment/'Cargo.lock').read_bytes(): raise ValueError('Lock changed')
    return new

def prepare(pristine, experiment):
    independent_roots(pristine, experiment)
    value = config(); gen = load('retained_storage_generator', PROTOTYPE/'generate.py')
    before = seal(pristine)
    patch, receipt = gen.generate(pristine)
    if patch != (PROTOTYPE/'prototype.patch').read_text() or receipt != json.loads((PROTOTYPE/'generation.json').read_text()):
        raise ValueError('Prototype generation drift')
    pins = json.loads((PROTOTYPE/'official-sources.json').read_text())
    original = gen.read_inputs(pristine, pins)
    transformed = gen.apply_in_memory(patch, original)
    if {p:sha(transformed[p].encode()) for p in receipt['changed_files']} != value['expected_delta']:
        raise ValueError('Wrong expected eleven-file patch')
    shutil.copytree(pristine, experiment)  # Exclusive temporary copy, never the pristine checkout.
    for name in receipt['changed_files']:
        target = experiment/name
        if not target.resolve().is_relative_to(experiment.resolve()) or target.is_symlink():
            raise ValueError('Unsafe experiment target')
        target.write_text(transformed[name])
    verify_delta(pristine, experiment, value['expected_delta'])
    if seal(pristine) != before: raise ValueError('Pristine source changed')

def metadata(log, root):
    value = json.loads(log)
    root = root.resolve()
    if Path(value['workspace_root']).resolve() != root or value.get('resolve') is None:
        raise ValueError('Wrong/incomplete metadata workspace')
    local = {p['id']:p for p in value['packages'] if p['source'] is None}
    if not set(value['workspace_members']) <= set(local): raise ValueError('External workspace member')
    for package in local.values():
        if not Path(package['manifest_path']).resolve().is_relative_to(root): raise ValueError('External local manifest')
        for target in package['targets']:
            if not Path(target['src_path']).resolve().is_relative_to(root): raise ValueError('External local target')
    game = [p for p in local.values() if p['name']=='horizon-client']
    if len(game)!=1: raise ValueError('Missing real gameplay package')
    targets = {(t['name'],tuple(t['kind']),Path(t['src_path']).relative_to(root).as_posix()) for t in game[0]['targets']}
    if not {('horizon_client',('lib',),'crates/horizon-client/src/lib.rs'),
            ('horizon-client',('bin',),'crates/horizon-client/src/main.rs')} <= targets:
        raise ValueError('Wrong gameplay lib/bin targets')
    return json.dumps(value, sort_keys=True).replace(str(root), '$OFFICIAL_ROOT')

def host_invocations(log):
    wanted = [('horizon_client_lib','client_lib.rs'),('horizon_client','src/lib.rs'),('horizon_client','src/main.rs')]
    selected = []
    for crate, source in wanted:
        lines = [line.strip() for line in log.splitlines() if 'Running `' in line and 'rustc' in line and
                 '--crate-name '+crate+' ' in line and source in line]
        if len(lines)!=1: raise ValueError('Missing/duplicate actual compiler invocation: '+crate+'/'+source)
        selected.append({'crate':crate,'source':source,'command_sha256':sha(lines[0].encode())})
    return selected

def out_dirs(target):
    return {p.relative_to(target).as_posix():sha(p.read_bytes()) for out in sorted(target.glob('debug/build/*/out'))
            for p in sorted(out.rglob('*')) if p.is_file()}

def execute(report):
    value = config(); head = base.git('rev-parse','HEAD')
    validate_event(os.environ,json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text()),head)
    base.clean(); source = seal(ROOT)
    work = Path(os.environ['RUNNER_TEMP']).resolve()/('guide-storage-'+head)
    if ROOT==work or ROOT in work.parents: raise ValueError('Work must be outside source')
    env = os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']: env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),
               CARGO_BUILD_JOBS='1',CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',
               TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1',CARGO_TERM_COLOR='never')
    runner = Runner(work,env); runner.guard(reserve=2*1024**3)
    work.mkdir(); (work/'cargo-home').mkdir(); (work/'tmp').mkdir()
    checkout=work/'official-git'; pristine=work/'pristine'; experiment=work/'experiment'
    seals={}; generated=None
    try:
        python_pass(runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v']),RUNNER_TESTS)
        report['gates']['runner']=RUNNER_TESTS
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal','--no-self-update'],timeout=600)
        runner.run('official-init',['git','init',str(checkout)])
        runner.run('official-origin',['git','-C',str(checkout),'remote','add','origin',HORIZON_URL])
        runner.run('official-fetch',['git','-C',str(checkout),'fetch','--depth','1','origin',HORIZON],timeout=600)
        runner.run('official-checkout',['git','-C',str(checkout),'checkout','--detach',HORIZON])
        exact=runner.run('official-head',['git','-C',str(checkout),'rev-parse','HEAD']).strip()
        if exact!=HORIZON: raise ValueError('Wrong official commit')
        seals[str(checkout)]=seal(checkout)
        runner.run('official-export',[sys.executable,str(HERE/'run.py'),'--export',str(checkout),str(pristine)])
        if sha((pristine/'Cargo.lock').read_bytes())!=LOCK: raise ValueError('Official lock drift')
        if 'sofia-gros/horizon' in (pristine/'Cargo.lock').read_text(): raise ValueError('Forbidden dependency destination')
        seals[str(pristine)]=seal(pristine); report['gates']['official_source']={'revision':exact,'lock_sha256':LOCK}
        env['PLUTO_OFFICIAL_SOURCE']=str(pristine)
        python_pass(runner.run('prototype-source',[sys.executable,str(PROTOTYPE/'test_patch.py'),'-v']),7)
        report['gates']['prototype_source']=7
        runner.run('prepare-experiment',[sys.executable,str(HERE/'run.py'),'--prepare',str(pristine),str(experiment)])
        seals[str(experiment)]=verify_delta(pristine,experiment,value['expected_delta'])
        report['gates']['experiment_delta']={'paths':value['expected_delta'],'count':11}
        runner.run('locked-fetch',['cargo','fetch','--locked','--manifest-path',str(pristine/'Cargo.toml')],cwd=pristine,timeout=900)
        arguments=['cargo','metadata','--locked','--offline','--no-default-features','--format-version','1','--manifest-path']
        old=metadata(runner.run('pristine-metadata',arguments+[str(pristine/'Cargo.toml')],cwd=pristine,separate_stdout=True),pristine)
        new=metadata(runner.run('experiment-metadata',arguments+[str(experiment/'Cargo.toml')],cwd=experiment,separate_stdout=True),experiment)
        if old!=new: raise ValueError('Dependency/feature graph changed beyond source-root relocation')
        report['gates']['metadata']={'normalized_sha256':sha(old.encode()),'full_dependencies':True}
        typed=runner.run('host-type',['cargo','check','--offline','--locked','--manifest-path',str(experiment/'Cargo.toml'),
                        '-p','horizon-client','--no-default-features','--lib','--bin','horizon-client','-vv'],cwd=experiment,timeout=2400)
        report['gates']['host_type']={'actual_compiler_invocations':host_invocations(typed),'log_sha256':sha(typed.encode())}
        generated=out_dirs(work/'target')
    except BaseException as exc: base.error(report,exc)
    finally:
        try:
            base.clean(); config()
            if seal(ROOT)!=source: raise ValueError('Mod source drift')
            for name,before in seals.items():
                if seal(Path(name))!=before: raise ValueError('Official/experiment source drift: '+name)
            if generated is not None and out_dirs(work/'target')!=generated: raise ValueError('Build-script output drift')
            runner.guard(); report['verified']=True
        except BaseException as exc: base.error(report,exc)
        report.update(minimum_free_bytes=runner.minimum,mod_source_seal=sha(json.dumps(source,sort_keys=True).encode()),
                      source_seals={name:sha(json.dumps(s,sort_keys=True).encode()) for name,s in seals.items()},
                      build_outputs_seal=sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None)

def main():
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},'floor_bytes':FLOOR,
            'scope':'Pristine official Horizon plus excluded eleven-file storage prototype: actual gameplay lib/bin typecheck only. Not Mixins-only support, installed Pluto/consumer compatibility, owned-open authority, Client/UI/language refresh or LAN execution.'}
    try:
        execute(report)
        if set(report['gates'])!=REQUIRED: raise ValueError('Missing required gate')
    except BaseException as exc: base.error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:
            f.write('## Isolated Guide storage typing\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc: base.error(report,exc); report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']: raise SystemExit(1)

if __name__=='__main__':
    if len(sys.argv)==4 and sys.argv[1]=='--export': config(); export(Path(sys.argv[2]),Path(sys.argv[3]))
    elif len(sys.argv)==4 and sys.argv[1]=='--prepare': prepare(Path(sys.argv[2]),Path(sys.argv[3]))
    elif len(sys.argv)==1: main()
    else: raise SystemExit('Invalid runner mode')
