#!/usr/bin/env python3
"""Frozen outer-tank comparison on real official dependencies, in dedicated CI only."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
FLOOR = 10 * 1024**3 + 128 * 1024**2

def existing_parent(path):
    path = Path(path).expanduser().resolve()
    while not path.exists():
        if path.parent == path: raise ValueError('No existing output filesystem: ' + str(path))
        path = path.parent
    return path

def write_filesystems(work, env):
    # apt writes /; rustup may use a different mount from workspace and Cargo.
    return [existing_parent(p) for p in [ROOT, work, env['CARGO_HOME'], Path('/'),
            env.get('RUSTUP_HOME', str(Path.home()/'.rustup'))]]

def sha(data): return hashlib.sha256(data).hexdigest()
def read_inputs(): return json.loads((HERE / 'inputs.json').read_text())
def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
def seal(root, tracked=False):
    # Include ignored/untracked files. Preserve official tracked documentation
    # symlinks as link targets rather than following them outside the tree.
    import stat
    result={}
    for path in sorted(root.rglob('*')):
        relative=path.relative_to(root)
        if '.git' in relative.parts:continue
        mode=path.lstat().st_mode
        if stat.S_ISDIR(mode):continue
        if stat.S_ISREG(mode):result[str(relative)]={'kind':'file','sha256':sha(path.read_bytes())}
        elif stat.S_ISLNK(mode):result[str(relative)]={'kind':'symlink','target':os.readlink(path)}
        else:raise ValueError('Special source node: '+str(path))
    return result
def assert_seal(root, expected, tracked=False):
    if seal(root, tracked) != expected: raise ValueError('Source/lock/file-set drift: ' + str(root))
def out_dirs(target):
    return {str(p.relative_to(target)): sha(p.read_bytes())
            for out in sorted(target.glob('debug/build/*/out')) for p in sorted(out.rglob('*')) if p.is_file()}

class Runner:
    def __init__(self, work, env):
        self.work, self.env = work, env
        self.minimum_free = None
    def guard(self, reserve=0):
        free = min(shutil.disk_usage(p).free for p in write_filesystems(self.work, self.env))
        self.minimum_free = free if self.minimum_free is None else min(self.minimum_free, free)
        memory = check_memory(reserve=2 * 1024**3 if reserve else 512 * 1024**2)
        self.minimum_memory = min(getattr(self, 'minimum_memory', memory), memory)
        if free < FLOOR + reserve: raise RuntimeError('Disk floor reached; own process group stopped')
    def run(self, name, command, cwd=None, timeout=900):
        self.guard()
        path = self.work / (name + '.log')
        print('PHASE', name, json.dumps(command), flush=True)
        started = time.monotonic()
        with path.open('w') as log:
            proc = subprocess.Popen(command, cwd=cwd or ROOT, env=self.env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            try:
                while proc.poll() is None:
                    self.guard()
                    if time.monotonic() - started > timeout: raise TimeoutError(name)
                    time.sleep(0.2)
            finally:
                # Never leave a child behind merely because the direct leader exited.
                # TERM has one bounded second; KILL and reap are bounded separately.
                if proc.poll() is None:
                    try: os.killpg(proc.pid, signal.SIGTERM)
                    except ProcessLookupError: pass
                    deadline = time.monotonic() + 1.0
                    while proc.poll() is None and time.monotonic() < deadline:
                        time.sleep(0.05)
                try: os.killpg(proc.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                proc.wait(timeout=2)
                deadline = time.monotonic() + 1.0
                while True:
                    try: os.killpg(proc.pid, 0)
                    except ProcessLookupError: break
                    if time.monotonic() >= deadline: raise RuntimeError('Own process group remains: ' + name)
                    time.sleep(0.02)
                print(path.read_text(errors='replace'), end='', flush=True)
        self.guard()
        print('TERMINAL', name, proc.returncode, 'seconds', round(time.monotonic()-started, 3), flush=True)
        if proc.returncode != 0: raise RuntimeError(name + ' exited ' + str(proc.returncode))
        return path.read_text()

def validate_tests(log, names, filtered=None):
    results = re.findall(r'^test (\S+) \.\.\. (\w+)\s*$', log, re.M)
    if len(results) != len(names) or {name for name, _ in results} != set(names):
        raise ValueError('Missing/duplicate/unexpected test names')
    if any(result != 'ok' for _, result in results): raise ValueError('A required test did not pass')
    summaries = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', log, re.M)
    if len(summaries) != 1 or summaries[0][:4] != (str(len(names)),'0','0','0'):
        raise ValueError('Result/count mismatch or skipped tests')
    if filtered is not None and summaries[0][4] != str(filtered): raise ValueError('Unexpected filtered test count')


def actual_compile(log, crate, test=False):
    for line in log.splitlines():
        if 'Running `' in line and 'rustc' in line and '--crate-name '+crate+' ' in line:
            if not test or ' --test' in line: return
    raise ValueError('Missing actual compiler evidence: '+crate)

def validate_suite(log, count, required):
    rows=re.findall(r'^test (\S+) \.\.\. (\w+)\s*$',log,re.M)
    names=[n for n,_ in rows]
    if len(names)!=count or len(set(names))!=count or not set(required)<=set(names):
        raise ValueError('Unexpected test names/count or omitted new cases')
    if any(state!='ok' for _,state in rows):raise ValueError('Nonpassing test')
    summaries=re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',log,re.M)
    if summaries!=[(str(count),'0','0','0','0')]:raise ValueError('Unexpected summary/skip/filter')
    return sorted(names)

def regular_seal(root):
    import stat
    result={}
    if root.is_symlink() or not root.is_dir():raise ValueError('Source directory is not regular')
    for p in root.rglob('*'):
        mode=p.lstat().st_mode
        if stat.S_ISDIR(mode):continue
        if not stat.S_ISREG(mode):raise ValueError('Symlink/special source: '+str(p))
        result[p.relative_to(root).as_posix()]=sha(p.read_bytes())
    return result

def identities(lock):
    import tomllib
    return {(p['name'],p['version'],p.get('source'),p.get('checksum')) for p in tomllib.loads(lock.read_text())['package']}

def assert_metadata(metadata, paths):
    local=[p for p in metadata['packages'] if p.get('source') is None]
    observed={p['name']:str(Path(p['manifest_path']).resolve()) for p in local}
    expected={name:str((path/'Cargo.toml').resolve()) for name,path in paths.items()}
    if len(local)!=len(paths) or observed!=expected:raise ValueError('Duplicate/substitute local API package')

def test_after_compile(runner, name, command, count, required):
    before=out_dirs(runner.work/'target')
    output=runner.run(name,command,cwd=runner.work/'fixture')
    if any('Running `' in line and 'rustc' in line for line in output.splitlines()):
        raise ValueError('Unexpected recompilation during sealed execution: '+name)
    if out_dirs(runner.work/'target')!=before:raise ValueError('Generated inputs changed during '+name)
    return validate_suite(output,count,required)

def memory_hierarchy(proc, root):
    """Accept only an unmasked cgroup2 mount exposing the real hierarchy root.

    CFTYPE_NOT_ON_ROOT checks cgroup_parent(), not the namespace root. Thus a
    delegated root retains cgroup.events/type. ENOENT is allowed only for the
    four root-only omissions, after mount/device/regular-node validation.
    Other topologies deliberately remain unsupported. This creates no limit.
    """
    import stat
    root = Path(root)
    if not root.is_absolute() or root.resolve() != root:
        raise ValueError('Symlink or unresolved cgroup mount')
    mounts = (proc/'self/mountinfo').read_text()
    rows = []
    for line in mounts.splitlines():
        before, separator, after = line.partition(' - ')
        fields, filesystem = before.split(), after.split()
        if not separator or len(fields) < 6 or len(filesystem) < 3:
            raise ValueError('Malformed mount identity')
        rows.append((fields, filesystem))
    owners = [(f, fs) for f, fs in rows if fs[0] == 'cgroup2']
    if len(owners) != 1:
        raise ValueError('Require one unambiguous cgroup2 mount')
    fields, _ = owners[0]
    if fields[3] != '/' or fields[4] != str(root):
        raise ValueError('Unsupported cgroup mount root or mount point')
    covering = [f for f, _ in rows if f[4] == str(root) or f[4].startswith(str(root)+'/')]
    if len(covering) != 1:
        raise ValueError('Masked or nested cgroup mount')
    if not re.fullmatch(r'[0-9]+:[0-9]+', fields[2]):
        raise ValueError('Malformed cgroup mount device')
    device = tuple(map(int, fields[2].split(':')))

    def checked(path, directory=False):
        info = path.lstat()
        kind = stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)
        if not kind or (os.major(info.st_dev), os.minor(info.st_dev)) != device:
            raise ValueError('Substituted cgroup node: '+str(path))
        return path

    checked(root, directory=True)
    controllers = checked(root/'cgroup.controllers').read_text().split()
    if 'memory' not in controllers:
        raise ValueError('Memory controller not observable at hierarchy root')
    for name in ['cgroup.events', 'cgroup.type', 'memory.max', 'memory.current']:
        try:
            (root/name).lstat()
        except FileNotFoundError:
            continue
        raise ValueError('Visible root is not the actual hierarchy root: '+name)

    membership = (proc/'self/cgroup').read_text()
    lines = membership.splitlines()
    if len(lines) != 1 or not lines[0].startswith('0::/'):
        raise ValueError('Require unique unified cgroup membership')
    relative = lines[0][3:]
    if (not re.fullmatch(r'/[A-Za-z0-9_.:@/-]*', relative) or '//' in relative or
        any(p in ('.', '..') for p in relative.split('/'))):
        raise ValueError('Unsupported or escaping cgroup membership')
    current = root / relative.lstrip('/')
    values = []
    while current != root:
        checked(current, directory=True)
        # Missing, permission-denied and malformed non-root values all reject.
        kind = checked(current/'cgroup.type').read_text().strip()
        if kind not in ('domain', 'domain threaded'):
            raise ValueError('Unsupported cgroup domain')
        checked(current/'cgroup.events').read_text()
        limit = checked(current/'memory.max').read_text().strip()
        usage = checked(current/'memory.current').read_text().strip()
        if (limit != 'max' and not re.fullmatch(r'[0-9]+', limit)) or not re.fullmatch(r'[0-9]+', usage):
            raise ValueError('Malformed cgroup memory observation')
        values.append((None if limit == 'max' else int(limit), int(usage)))
        current = current.parent
    if ((proc/'self/cgroup').read_text() != membership or
        (proc/'self/mountinfo').read_text() != mounts):
        raise ValueError('Cgroup topology changed during observation')
    return values

def check_memory(reserve, proc=Path('/proc'), root=Path('/sys/fs/cgroup')):
    """Observe both host availability and every enclosing cgroup-v2 limit.

    The true root lacks memory files by kernel design; delegated/hidden roots
    are not accepted. Thresholds are admission/stop limits, not hard caps or
    measured peak estimates. The native entry point is still unreachable.
    """
    mem = {}
    for line in (proc/'meminfo').read_text().splitlines():
        key, value = line.split(':', 1)
        if key in ('MemAvailable', 'MemTotal'): mem[key] = int(value.split()[0]) * 1024
    if set(mem) != {'MemAvailable', 'MemTotal'}: raise ValueError('Unknown RAM capacity')
    if not 0 <= mem['MemAvailable'] <= mem['MemTotal']:
        raise ValueError('Inconsistent host memory values')
    capacity, available = mem['MemTotal'], mem['MemAvailable']
    for limit, usage in memory_hierarchy(proc, root):
        if limit is not None:
            capacity = min(capacity, limit)
            available = min(available, max(0, limit-usage))
    if capacity < 6 * 1024**3 or available < reserve: raise RuntimeError('RAM/cgroup safety limit reached')
    return available

def memory_observation(proc=Path('/proc'), cgroup_root=Path('/sys/fs/cgroup')):
    """Read-only evidence, never an exception to the fail-closed memory guard."""
    def read(path):
        try:
            with path.open('r') as file: value=file.read(65537)
            if len(value)>65536:return {'error':'observation exceeds 64 KiB'}
            return {'text':value}
        except OSError as exc:return {'error':type(exc).__name__+': '+str(exc)}
    meminfo=read(proc/'meminfo')
    memory={}
    for line in meminfo.get('text','').splitlines():
        if line.startswith(('MemTotal:','MemAvailable:')):
            key,value=line.split(':',1);memory[key]=value.strip()
    self_group=read(proc/'self/cgroup');mounts=read(proc/'self/mountinfo')
    result={'admission':'not determined by this observation; guard remains independent',
            'host_memory':memory,'self_cgroup':self_group,'cgroup2_mounts':[],
            'visible_ancestors':[]}
    if 'error' in meminfo:result['host_memory_error']=meminfo['error']
    if 'error' in mounts:result['mount_error']=mounts['error']
    for line in mounts.get('text','').splitlines():
        before,separator,after=line.partition(' - ')
        if separator and after.split()[0]=='cgroup2':
            fields=before.split()
            result['cgroup2_mounts'].append({'root':fields[3],'mount_point':fields[4],
                                           'options':fields[5],'filesystem':after})
    entries=[line[3:] for line in self_group.get('text','').splitlines() if line.startswith('0::')]
    root=cgroup_root.resolve()
    if len(entries)!=1:result['hierarchy_error']='No unique observable cgroup-v2 membership';return result
    current=(root/entries[0].lstrip('/')).resolve()
    if not current.is_relative_to(root):result['hierarchy_error']='Membership path outside visible root';return result
    while True:
        facts={name:read(current/name) for name in ['memory.max','memory.current','cgroup.controllers','cgroup.events','cgroup.type']}
        result['visible_ancestors'].append({'path':str(current),'facts':facts})
        if current==root:break
        current=current.parent
    return result

def prepare_prefix(source, expected_sha):
    """Only relocate the frozen harness, stopping before its first Cargo call.

    The old compiler/test driver is deliberately not run. Production Rust,
    complete old functions, scaffold and their construction remain byte-exact.
    The two substitutions are inverted before this program can be executed.
    """
    if sha(source.encode()) != expected_sha: raise ValueError('Frozen preparation source changed')
    boundary = '    lock_fixture_to_host(manifest, host, root)\n'
    if source.count(boundary) != 1: raise ValueError('Nonunique preparation boundary')
    prefix = source[:source.index(boundary)]
    edits = [
        ('root = Path(__file__).resolve().parents[2]', "root = Path(__import__('sys').argv[2]).resolve()"),
        ("with tempfile.TemporaryDirectory(prefix='outer-tank-patch-') as temp:",
         "with __import__('contextlib').nullcontext(__import__('sys').argv[1]) as temp:"),
    ]
    relocated = prefix
    for old, new in edits:
        if relocated.count(old) != 1 or new in relocated: raise ValueError('Relocation boundary changed')
        relocated = relocated.replace(old, new, 1)
    restored = relocated
    for old, new in reversed(edits): restored = restored.replace(new, old, 1)
    if restored != prefix: raise ValueError('Preparation inverse reconstruction failed')
    # The prefix may import the real dependency helpers, but must not call Cargo.
    import ast
    tree = ast.parse(relocated)
    for node in ast.walk(tree):
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and isinstance(node.func.value, ast.Name) and node.func.value.id == 'subprocess':
            raise ValueError('Preparation unexpectedly invokes a process')
    return relocated

def frozen_files(config, source):
    expected = {p: row['sha256'] for p,row in config['files'].items()}
    if len(expected) != 55 or regular_seal(source) != expected: raise ValueError('Frozen 55-file input drift')
    api = source/config['api_path']
    prefix = config['api_path']+'/'
    if regular_seal(api) != {p[len(prefix):]:expected[p] for p in config['api_files']}:
        raise ValueError('Actual full API file set differs')
    return expected

def verify_generated(fixture, source):
    """Independent construction check, not another semantic implementation."""
    source_runner = (source/'tconstruct/tests/run_outer_tank_patch.py').read_text()
    meta = json.loads((source/'tconstruct/tests/rust/outer_tank_legacy.json').read_text())
    production = (source/'tconstruct/mixins/tconstruct/source-fluid-cannon.rs').read_bytes()
    old = (source/meta['methods']['cannon']['oracle_path']).read_text()
    old = old.replace('fn tank_to_nbt_preserving(', 'fn legacy_tank_to_nbt_preserving(', 1)
    expected = production + f'\n#[cfg(test)]\nimpl Cannon {{\n{old}\n}}\n'.encode()
    if (fixture/'cannon-with-oracle.rs').read_bytes() != expected: raise ValueError('Actual Cannon prefix/oracle differs')
    # Exact generated Rust is independently regenerated from the unchanged
    # prefix in a second directory. Compare after only its own absolute path
    # relocation; no native semantics or fixtures are replaced by stand-ins.
    return sha(source_runner.encode()), sha(expected)

def package_paths(host, source, fixture):
    result = {name:host/'crates'/name for name in ['horizon-core','horizon-world','horizon-client-core','horizon-items']}
    result['pluto-inventory-api'] = source/'pluto-lib/mixins/pluto_native_creative/inventory-api'
    result['outer-tank-patch-fixture'] = fixture
    return result

def verify_lock(host, fixture):
    allowed = identities(host/'Cargo.lock') | {('pluto-inventory-api','0.1.0',None,None),('outer-tank-patch-fixture','0.1.0',None,None)}
    if identities(fixture/'Cargo.lock')-allowed: raise ValueError('Nonofficial dependency identity in fixture lock')

REQUIRED_GATES={'source_guards','exact_preparation','official_dependencies','outer_tank'}
NATIVE_BRANCH='checkpoint/plutolib-outer-tank-native-ci-20261106'

def guarded_native(report, config):
    """Separate reviewed-mode proposal; never fall back when admission fails."""
    report.update(execution_mode='guarded-native',setup_started=False,native_started=False)
    if config.get('execution_mode')!='guarded-native' or config['branch']!=NATIVE_BRANCH:
        raise ValueError('Unsupported outer-tank execution mode or branch')
    available=check_memory(reserve=2*1024**3)
    report['memory_admission']={'passed':True,'profile':'verified-real-root-and-every-ancestor',
                                'available_bytes':available,'reserve_bytes':2*1024**3,
                                'hard_limit_applied':False}
    execute_native_prepared(report,config)

def validate_source_config(config):
    official={'url':'https://github.com/horizon-rust/horizon.git',
              'revision':'624c4f6265c02296fbe8a80f029dd0d987fb94c0',
              'lock_sha256':'463913123985e344f2889e5beae65982773e6dbfacdcac7a916bcec522a5bd31'}
    if (config['repository']!='sofia-gros/horizon-mods' or
        config['source_url']!='https://github.com/sofia-gros/horizon-mods.git' or
        config['source_revision']!='794a0c526ca0a96d0ca9c9ff40010611b5968a89' or
        config['source_tree']!='0e80347d4573f47984753cd02f78514a5352243f' or
        config['official']!=official):
        raise ValueError('Unexpected frozen MOD or official source')

def diagnostic_only(report, config):
    if config.get('execution_mode')!='memory-diagnostic-only':
        raise ValueError('This checkpoint permits only read-only memory diagnostics')
    report['execution_mode']='memory-diagnostic-only'
    report['setup_started']=False
    report['native_started']=False
    report['memory_preflight_observation']=memory_observation()
    print('MEMORY_PREFLIGHT',json.dumps(report['memory_preflight_observation'],sort_keys=True),flush=True)
    raise RuntimeError('Unverified memory topology: diagnostic-only checkpoint cannot start setup or compilation')

def execute(report):
    config=read_inputs()
    validate_source_config(config)
    event=json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text())
    if (os.environ.get('GITHUB_ACTIONS')!='true' or os.environ.get('GITHUB_REPOSITORY')!=config['repository'] or
        os.environ.get('GITHUB_EVENT_NAME')!='push' or os.environ.get('GITHUB_REF')!='refs/heads/'+config['branch'] or
        event.get('repository',{}).get('private') is not False or
        git(ROOT,'rev-parse','HEAD')!=os.environ.get('GITHUB_SHA') or git(ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching')):
        raise ValueError('Require clean exact authorized public CI branch')
    guarded_native(report,config)

def execute_native_prepared(report,config):
    """Prepared native driver, called only after strict real-root admission."""
    original=seal(ROOT,True)
    work=Path(os.environ['RUNNER_TEMP'])/('outer-tank-'+os.environ['GITHUB_SHA'])
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER']:env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),
               CARGO_BUILD_JOBS='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',
               CARGO_TERM_COLOR='never',TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1')
    runner=Runner(work,env)
    runner.guard(reserve=4*1024**3)
    report['setup_started']=True
    work.mkdir();Path(env['CARGO_HOME']).mkdir();Path(env['TMPDIR']).mkdir()
    gates={};snapshots={};host=None;official_before=None;error=None;verified=False;generated=None
    try:
        checks=runner.run('runner-checks',[sys.executable,str(HERE/'test_runner.py'),'-v'])
        if re.findall(r'^Ran (\d+) tests in .+$',checks,re.M)!=['29'] or re.findall(r'^OK$',checks,re.M)!=['OK']:
            raise ValueError('Expected twenty-nine unskipped runner checks')
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal'],timeout=600)
        roots=[]
        for name,url,revision in [('frozen-mod',config['source_url'],config['source_revision']),('official',config['official']['url'],config['official']['revision'])]:
            path=work/name
            runner.guard(reserve=4*1024**3)
            runner.run(name+'-init',['git','init',str(path)])
            runner.run(name+'-origin',['git','-C',str(path),'remote','add','origin',url])
            runner.run(name+'-fetch',['git','-C',str(path),'fetch','--depth','1','origin',revision],timeout=600)
            if git(path,'rev-parse','FETCH_HEAD')!=revision or git(path,'remote','get-url','origin')!=url:raise ValueError('Source provenance changed')
            roots.append(path)
        frozen,host=roots
        if git(frozen,'rev-parse','FETCH_HEAD^{tree}')!=config['source_tree']:raise ValueError('Frozen MOD tree differs')
        runner.run('official-checkout',['git','-C',str(host),'checkout','--detach',config['official']['revision']])
        if sha((host/'Cargo.lock').read_bytes())!=config['official']['lock_sha256']:raise ValueError('Wrong official lock')
        official_before=seal(host,True)
        source=work/'source';source.mkdir()
        for relative,row in config['files'].items():
            if Path(relative).is_absolute() or '..' in Path(relative).parts:raise ValueError('Unsafe frozen path')
            runner.guard()
            blob=subprocess.check_output(['git','-C',str(frozen),'show',config['source_revision']+':'+relative])
            if sha(blob)!=row['sha256'] or len(blob)!=row['bytes']:raise ValueError('Frozen blob mismatch')
            dest=source/relative;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(blob)
        snapshots[str(source)]=frozen_files(config,source)
        env['HORIZON_HOST']=str(host)
        env['PYTHONPATH']=str(source/'tconstruct/tests')
        guardlog=runner.run('source-guards',[sys.executable,str(source/'tconstruct/tests/test_outer_tank_patch.py'),'-v'])
        if re.findall(r'^Ran (\d+) tests in .+$',guardlog,re.M)!=['5'] or re.findall(r'^OK$',guardlog,re.M)!=['OK']:
            raise ValueError('Expected five unchanged source guards')
        gates['source_guards']={'passed':True,'tests':5}
        script=source/config['fixture_runner']
        prepared=prepare_prefix(script.read_text(),config['files'][config['fixture_runner']]['sha256'])
        prefix=work/'prepare.py';prefix.write_text(prepared)
        fixtures=[work/'fixture',work/'preparation-check']
        for f in fixtures:
            f.mkdir();runner.run('prepare-'+f.name,[sys.executable,str(prefix),str(f),str(source)])
            verify_generated(f,source)
        fixture,independent=fixtures
        first=regular_seal(fixture);second=regular_seal(independent)
        if first.keys()!=second.keys():raise ValueError('Generated file-set differs')
        for p in first:
            a=(fixture/p).read_bytes();b=(independent/p).read_bytes().replace(str(independent).encode(),str(fixture).encode())
            if a!=b:raise ValueError('Generated source not reproducible: '+p)
        snapshots[str(independent)]=second
        gates['exact_preparation']={'passed':True,'prefix_sha256':sha(prepared.encode()),'files':first}
        shutil.copyfile(host/'Cargo.lock',fixture/'Cargo.lock')
        # Cold metadata may only prune the official lock and add two local
        # packages; check every identity/path before any compilation.
        runner.guard(reserve=4*1024**3)
        meta=runner.run('cold-metadata',['cargo','metadata','--format-version','1','--manifest-path',str(fixture/'Cargo.toml')],cwd=fixture,timeout=600)
        assert_metadata(json.loads(meta),package_paths(host,source,fixture));verify_lock(host,fixture)
        snapshots[str(fixture)]=regular_seal(fixture)
        runner.run('locked-fetch',['cargo','fetch','--locked','--manifest-path',str(fixture/'Cargo.toml')],cwd=fixture,timeout=600)
        meta=runner.run('locked-metadata',['cargo','metadata','--locked','--offline','--format-version','1','--manifest-path',str(fixture/'Cargo.toml')],cwd=fixture)
        assert_metadata(json.loads(meta),package_paths(host,source,fixture));verify_lock(host,fixture)
        gates['official_dependencies']={'passed':True,'packages':sorted(package_paths(host,source,fixture))}
        phase=work/'compiler-phase';phase.write_text('compile')
        env.update(RUSTC_WRAPPER=str(HERE/'rustc_gate.py'),OUTER_TANK_COMPILER_PHASE=str(phase),OUTER_TANK_COMPILER_LOG=str(work/'rustc-invocations.jsonl'))
        command=['cargo','test','--lib','--locked','--offline','--manifest-path',str(fixture/'Cargo.toml'),'-vv']
        report['native_started']=True
        built=runner.run('actual-compile',command+['--no-run'],cwd=fixture,timeout=1200)
        for crate in ['horizon_core','horizon_world','horizon_client_core','horizon_items','pluto_inventory_api']:
            actual_compile(built,crate)
        actual_compile(built,'outer_tank_patch_fixture',True)
        phase.write_text('sealed')
        generated=out_dirs(work/'target')
        for path,expected in snapshots.items():
            if regular_seal(Path(path))!=expected:raise ValueError('Copied source/file-set drift before execution')
        assert_seal(host,official_before,True)
        names=test_after_compile(runner,'actual-tests',command+['--','--test-threads=1'],160,config['required_tests'])
        calls=[json.loads(line) for line in (work/'rustc-invocations.jsonl').read_text().splitlines()]
        if any(row['decision']=='denied' for row in calls):raise ValueError('Compiler invocation denied')
        gates['outer_tank']={'passed':True,'tests':len(names),'names':names,'new_required_tests':config['required_tests'],'inherited_tests':156}
        if set(gates)!=REQUIRED_GATES:raise ValueError('Missing gate')
    except BaseException as exc:
        error=type(exc).__name__+': '+str(exc)
    finally:
        try:
            assert_seal(ROOT,original,True)
            if git(ROOT,'status','--porcelain','--untracked-files=all','--ignored=matching'):raise ValueError('Dirty/untracked checkout after proof')
            for path,before in snapshots.items():
                if regular_seal(Path(path))!=before:raise ValueError('Copied source/file-set drift after execution')
            if official_before is not None:
                assert_seal(host,official_before,True)
                if git(host,'status','--porcelain','--untracked-files=all','--ignored=matching'):raise ValueError('Dirty/untracked official source')
            if generated is not None and out_dirs(work/'target')!=generated:raise ValueError('Generated inputs changed after compile')
            runner.guard();verified=True
        except BaseException as exc:
            error=(error+'; ' if error else '')+'verification: '+type(exc).__name__+': '+str(exc)
        report.update({'head':os.environ['GITHUB_SHA'],'verified':verified,'error':error,'gates':gates,
                'minimum_free_bytes':runner.minimum_free,'minimum_available_memory_bytes':getattr(runner,'minimum_memory',None),'floor_bytes':FLOOR,
                'frozen_source':config['source_revision'],'official':config['official'],
                'source_seal_sha256':sha(json.dumps(original,sort_keys=True).encode()),
                'copied_input_seals':{p:sha(json.dumps(s,sort_keys=True).encode()) for p,s in snapshots.items()},
                'official_source_seal_sha256':sha(json.dumps(official_before,sort_keys=True).encode()) if official_before is not None else None,
                'generated_output_seal_sha256':sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None,
                'scope':config['scope']})

def main():
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{}}
    try:
        execute(report)
        if report['error'] is None and set(report['gates'])!=REQUIRED_GATES:raise ValueError('Missing required final gate')
    except BaseException as exc:
        report['error']=(report['error']+'; ' if report['error'] else '')+type(exc).__name__+': '+str(exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        summary='## Outer tank proof\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n'
        destination=Path(os.environ['GITHUB_STEP_SUMMARY'])
        if shutil.disk_usage(existing_parent(destination)).free < FLOOR+len(summary.encode()):raise RuntimeError('Insufficient disk floor for summary write')
        with destination.open('a') as f:f.write(summary)
    except BaseException as exc:
        report['error']=(report['error']+'; ' if report['error'] else '')+'summary: '+type(exc).__name__+': '+str(exc)
        report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)

if __name__=='__main__':main()

