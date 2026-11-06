#!/usr/bin/env python3
"""Excluded std Batch predicates only. No native Mesher, plan or host type proof."""
import importlib.util
import json
import os
from pathlib import Path
import re
import resource
import stat
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
BRANCH='checkpoint/plutolib-terrain-bounds-ci-20261106'
REPO='sofia-gros/horizon-mods'
BASE='e6d82461c1035a05d557700704bf212c97f531a2'
WORKFLOW='.github/workflows/terrain-bounds-wip.yml'
AS_LIMIT=1024**3
LOG_LIMIT=8*1024**2
RUNNER_TESTS=20
PREFIX='pluto-lib/experiments/'
INPUT_PATHS={PREFIX+'terrain-bounds-ci/'+p for p in ('run.py','test_runner.py','README.md')}
INPUT_PATHS|={WORKFLOW,PREFIX+'terrain-material-cpu-ci/run.py',PREFIX+'terrain-author-ci/run.py'}
INPUT_PATHS|={PREFIX+'terrain-local-bounds/'+p for p in ('candidate.rs','fixture.rs','source.py','test_source.py','inputs.json','README.md')}
INPUT_PATHS|={PREFIX+'terrain-host-bounds/'+p for p in ('compose.py','test_source.py','inputs.json','README.md')}
INPUT_PATHS|={'pluto-lib/mixins/pluto_native_creative/terrain-cell-'+p+'.rs' for p in ('host','geometry','providers')}
REQUIRED={'runner','bounds_source11','host_composer8','toolchain','compose',
          'compile0','inventory0','tests0','compile3','inventory3','tests3'}


def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m


C=load('frozen_cpu_utilities',HERE.parent/'terrain-material-cpu-ci/run.py')
A=C.A
B=load('frozen_bounds_composer',HERE.parent/'terrain-local-bounds/source.py')
H=load('frozen_host_composer',HERE.parent/'terrain-host-bounds/compose.py')
sha=A.sha


def config():
    c=json.loads((HERE/'inputs.json').read_text())
    if (c['repository'],c['branch'],c['base'])!=(REPO,BRANCH,BASE):raise ValueError('Wrong fixed scope')
    if set(c['source_sha256'])!=INPUT_PATHS:raise ValueError('Input inventory changed')
    for p,digest in c['source_sha256'].items():H.read(ROOT,p,digest)
    geometry=(ROOT/B.GEOMETRY).read_text()
    fixture=(B.HERE/'fixture.rs').read_text()
    expected=['geometry::tests::'+n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(',geometry)]
    expected+=['bounds_tests::'+n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(',fixture)]
    if len(expected)!=14 or len(set(expected))!=14 or c['test_names']!=sorted(expected):raise ValueError('Wrong expected test inventory')
    raw,receipt=B.compose(ROOT);_,host=H.compose(ROOT)
    if sha(raw)!=c['generated_sha256'] or receipt['prepared_tests']!=14 or host['generated_sha256']!=c['host_generated_sha256']:
        raise ValueError('Changed exact composed inputs')
    return c


def event(head):
    e=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if (os.environ.get('GITHUB_ACTIONS')!='true' or os.environ.get('GITHUB_REPOSITORY')!=REPO or
        os.environ.get('GITHUB_EVENT_NAME')!='push' or os.environ.get('GITHUB_REF')!='refs/heads/'+BRANCH or
        os.environ.get('GITHUB_SHA')!=head or e.get('repository',{}).get('private') is not False or
        e.get('repository',{}).get('full_name')!=REPO):raise ValueError('Exact public dedicated push required')


def binary_seal(path):
    mode=path.lstat().st_mode
    if not stat.S_ISREG(mode) or not mode & stat.S_IXUSR:raise ValueError('Missing regular executable')
    return {'sha256':sha(path.read_bytes()),'bytes':path.stat().st_size,'mode':stat.S_IMODE(mode)}


def inventory(log,names):
    if log!=''.join(n+': test\n' for n in sorted(names)):raise ValueError('Wrong complete terse inventory')


def timings(log):
    lines=[s for s in log.splitlines() if s.startswith('LOCAL_BOUNDS_MICRO')]
    rows=[]
    pattern=(r'LOCAL_BOUNDS_MICRO case=(\w+) iterations=4096 warmup=128 samples=7 '
             r'old_ns=(\[[0-9, ]+\]) candidate_ns=(\[[0-9, ]+\]) observed_clock_ns=(\d+) inconclusive=(true|false)')
    for line in lines:
        found=re.fullmatch(pattern,line)
        if found is None:raise ValueError('Malformed micro receipt')
        name,old,new,clock,short=found.groups();old=json.loads(old);new=json.loads(new);clock=int(clock)
        if not 0<clock<2**64 or any(len(x)!=7 or any(type(v)is not int or not 0<=v<2**64 for v in x) for x in (old,new)):
            raise ValueError('Invalid bounded micro observation')
        inconclusive=any(n<1_000_000 or n<min(2**64-1,clock*100) for n in old+new)
        if (short=='true')!=inconclusive:raise ValueError('Short sample status mismatch')
        rows.append({'case':name,'original_total_ns':old,'candidate_total_ns':new,
                     'observed_clock_ns':clock,'timing_inconclusive':inconclusive})
    if [r['case'] for r in rows]!=['front6','paired12','nonlocal','empty']:raise ValueError('Missing/duplicate micro cases')
    return rows


def compile_argv(compiler,source,binary,opt):
    if type(opt)is not int or opt not in (0,3):raise ValueError('Unsupported profile')
    return [str(compiler),'--edition=2021','--test','--crate-name','pluto_terrain_local_bounds',
            '-C','opt-level='+str(opt),'-C','debug-assertions=on','-C','overflow-checks=on',
            '-C','debuginfo=0','-C','codegen-units=16','-C','lto=off',str(source),'-o',str(binary)]


class Runner(C.Runner):
    def guard(self,reserve=0):
        super().guard(reserve)
        if self.work.exists() and any(p.stat().st_size>LOG_LIMIT for p in self.work.glob('*.log')):
            raise RuntimeError('Own log budget exceeded')


def child():
    # All compiler/linker/test processes inherit these per-process hard limits.
    # This is not an aggregate cgroup/RAM guarantee and is never a local escape.
    if os.getpid()!=os.getpgrp() or os.getsid(0)!=os.getpid() or os.environ.get('PLUTO_BOUNDS_CONTROLLER')!='std-only-v1':raise ValueError('Guarded child required')
    event(A.git(ROOT,'rev-parse','HEAD'));c=config()
    mode,opt=sys.argv[2],int(sys.argv[3]);work=Path(os.environ['PLUTO_BOUNDS_WORK'])
    expected=Path(os.environ['RUNNER_TEMP']).resolve()/('terrain-bounds-'+os.environ['GITHUB_SHA'])
    if work!=expected or mode not in ('compile','test','inventory') or opt not in (0,3):raise ValueError('Unreviewed child operation')
    source=work/'fixture.rs';binary=work/('bounds-opt'+str(opt));compiler=Path(os.environ['PLUTO_BOUNDS_COMPILER'])
    if sha(source.read_bytes())!=c['generated_sha256'] or binary_seal(compiler)['sha256']!=os.environ['PLUTO_BOUNDS_COMPILER_SHA']:
        raise ValueError('Changed compiler or source')
    if mode=='compile':
        if binary.exists() or binary.is_symlink():raise ValueError('Compiler output already exists')
        argv=compile_argv(compiler,source,binary,opt)
    else:
        if binary_seal(binary)['sha256']!=os.environ['PLUTO_BOUNDS_BINARY_SHA']:raise ValueError('Changed sealed executable')
        argv=[str(binary)]+(['--list','--format','terse'] if mode=='inventory' else ['--test-threads=1','--show-output'])
    resource.setrlimit(resource.RLIMIT_AS,(AS_LIMIT,AS_LIMIT))
    resource.setrlimit(resource.RLIMIT_CPU,(120,120))
    resource.setrlimit(resource.RLIMIT_FSIZE,(128*1024**2,128*1024**2))
    if mode=='compile':print('COMPILE_ARGV',json.dumps(argv),flush=True)
    os.execv(argv[0],argv)


def error(report,exc):
    text=type(exc).__name__+': '+str(exc)
    report['error']=(report['error']+'; ' if report['error'] else '')+text


def execute(report):
    c=config();head=A.git(ROOT,'rev-parse','HEAD');event(head);C.assert_checkout()
    original=C.strict_seal(ROOT)
    work=Path(os.environ['RUNNER_TEMP']).resolve()/('terrain-bounds-'+head)
    if ROOT.is_relative_to(work) or work.is_relative_to(ROOT):raise ValueError('Output/source overlap')
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','RUSTC','CARGO','GIT_DIR','GIT_WORK_TREE','GIT_INDEX_FILE']:
        env.pop(key,None)
    env.update(PYTHONDONTWRITEBYTECODE='1',RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),
               TMPDIR=str(work/'tmp'),PLUTO_BOUNDS_CONTROLLER='std-only-v1',PLUTO_BOUNDS_WORK=str(work))
    runner=Runner(work,env,report);runner.guard(reserve=2*1024**3)
    work.mkdir();(work/'tmp').mkdir();(work/'cargo-home').mkdir()
    compiler=None;compiler_before=None;compiled={};g=report['gates']
    try:
        for key,path,n in [('runner',HERE/'test_runner.py',RUNNER_TESTS),
                           ('bounds_source11',B.HERE/'test_source.py',11),('host_composer8',H.HERE/'test_source.py',8)]:
            A.validate_python_checks(runner.run(key,[sys.executable,str(path),'-v'],timeout=60),n)
            g[key]={'passed':True,'tests':n}
        runner.run('toolchain-install',['rustup','toolchain','install','1.95.0','--profile','minimal','--no-self-update'],timeout=600)
        env['RUSTUP_AUTO_INSTALL']='0'
        compiler=Path(runner.run('compiler-path',['rustup','which','--toolchain','1.95.0','rustc'],timeout=30).strip())
        if not compiler.is_absolute():raise ValueError('Compiler must be absolute')
        compiler_before=binary_seal(compiler)
        identity=runner.run('compiler-version',[str(compiler),'--version','--verbose'],timeout=30)
        if '\nrelease: 1.95.0\n' not in '\n'+identity:raise ValueError('Wrong compiler identity')
        env.update(PLUTO_BOUNDS_COMPILER=str(compiler),PLUTO_BOUNDS_COMPILER_SHA=compiler_before['sha256'])
        g['toolchain']={'identity':identity,'binary':compiler_before}
        raw,receipt=B.compose(ROOT);_,host=H.compose(ROOT)
        with (work/'fixture.rs').open('xb') as output:output.write(raw)
        g['compose']={'std':receipt,'host_source_only':host}
        for opt in (0,3):
            binary=work/('bounds-opt'+str(opt))
            command=[sys.executable,str(HERE/'run.py'),'--child','compile',str(opt)]
            output=runner.run('compile'+str(opt),command,timeout=240)
            expected=compile_argv(compiler,work/'fixture.rs',binary,opt)
            if re.findall(r'^COMPILE_ARGV (.+)$',output,re.M)!=[json.dumps(expected)]:raise ValueError('Missing exact compiler invocation')
            compiled[opt]=binary_seal(binary);env['PLUTO_BOUNDS_BINARY_SHA']=compiled[opt]['sha256']
            g['compile'+str(opt)]={'argv':expected,'binary':compiled[opt],'per_process_address_limit':AS_LIMIT}
            inventory(runner.run('inventory'+str(opt),[sys.executable,str(HERE/'run.py'),'--child','inventory',str(opt)],timeout=60),c['test_names'])
            g['inventory'+str(opt)]={'tests':14,'names':c['test_names']}
            log=runner.run('tests'+str(opt),[sys.executable,str(HERE/'run.py'),'--child','test',str(opt)],timeout=240)
            A.validate_tests(log,c['test_names'],filtered=0)
            rows=timings(log);g['tests'+str(opt)]={'tests':14,'ignored':0,'filtered':0,'micro':rows}
            print('BOUNDS_MICRO_RECEIPT',json.dumps({'opt_level':opt,'rows':rows,'scope':'pure predicates, not Mesher timing'}),flush=True)
            if binary_seal(binary)!=compiled[opt]:raise ValueError('Executable changed during test')
    except BaseException as exc:error(report,exc)
    finally:
        try:
            C.assert_checkout();config();C.assert_strict(ROOT,original)
            if compiler_before is not None and binary_seal(compiler)!=compiler_before:raise ValueError('Compiler changed')
            if (work/'fixture.rs').exists() and sha((work/'fixture.rs').read_bytes())!=c['generated_sha256']:raise ValueError('Generated source changed')
            for opt,expected in compiled.items():
                if binary_seal(work/('bounds-opt'+str(opt)))!=expected:raise ValueError('Sealed executable changed')
            runner.guard();report['verified']=True
        except BaseException as exc:error(report,exc)
        report['minimum_free_bytes']=runner.minimum_free
        report['source_seal_sha256']=sha(json.dumps(original,sort_keys=True).encode())


def main():
    if len(sys.argv)>1:
        if len(sys.argv)!=4 or sys.argv[1]!='--child':raise ValueError('Unknown child operation')
        child();return
    report={'passed':False,'verified':False,'error':None,'gates':{},'phases':[],
            'scope':'std private Batch predicates at opt0/3 only; no actual host type, native Mesher, latest compatibility, GPU or shipping performance proof'}
    try:
        execute(report)
        if set(report['gates'])!=REQUIRED:raise ValueError('Missing/unexpected required gate')
    except BaseException as exc:error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)


if __name__=='__main__':main()
