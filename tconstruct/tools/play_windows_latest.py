#!/usr/bin/env python3
"""Explicit, offline Windows GNU playtest. No installer, clone, fetch or deletion.

The complete native plan must pass the repository's unchanged guards before apply.
This source candidate is not evidence of Windows compilation or game startup.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import struct
import stat
import subprocess
import sys
import tomllib

from playtest_pluto_inputs import directory, plain, signature, fingerprint, read_signature, require_opened_file
import prepare_playtest_bundles as bundles
import playtest_guard_rustfmt as guard_format

HORIZON = '41f822b659789780f58d32c63ddfb6887ea1b8bf'
MIXINS = '5a64af938ec86b00d53d16a408aabc0038fb3e57'
HORIZON_LOCK = '0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f'
MIXINS_LOCK = '35846f4a621cf8e1e54f6f0f7e13ac848db590d4ec401620501c107954eae6ca'
VERSION = '1.99.1'
TARGET = 'x86_64-pc-windows-gnu'
MODS_FEATURE = 'horizon-client/mods'
PROFILE = 'fast'
STAGE_SHA = 'c2d14a49bfc55caadb8159441e87ee6a21203586bf7a725b4648ecd653166092'
# These are read budgets, not claims about maximum compiler/resource disk growth.
MAX_FILES = 100000
MAX_TOTAL = 64 * 1024**3
SCRUB = {'RUSTUP_TOOLCHAIN', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
         'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTDOCFLAGS',
         'CARGO_ENCODED_RUSTDOCFLAGS', 'CARGO_INCREMENTAL',
         'PYTHONUTF8', 'PYTHONIOENCODING', 'PLUTO_GUARD_RUSTFMT', 'CARGO', 'RUSTC', 'RUSTDOC', 'CC', 'CXX', 'AR', 'RANLIB', 'WINDRES',
         'CFLAGS', 'CXXFLAGS', 'LDFLAGS', 'HORIZON_MOD_INJECTIONS'}


def sha(data): return hashlib.sha256(data).hexdigest()
def canonical(value): return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()
def read(path):
    info = path.lstat(); plain(info)
    return path.read_bytes()


def run(argv, *, cwd, env, capture=False):
    # Admission supplies only reviewed binaries. No shell, no external script expansion.
    result = subprocess.run([str(x) for x in argv], cwd=cwd, env=env, check=True,
                            stdout=subprocess.PIPE if capture else None,
                            text=capture, encoding='utf-8' if capture else None)
    return result.stdout.strip() if capture else None


def file_hash(path, before):
    # Rust's LLVM DLL and prepared resource files can exceed Pluto's 64 MiB limit.
    if before.st_size > 2 * 1024**3: raise ValueError('Input file exceeds 2 GiB read budget')
    result = hashlib.sha256(); count = 0
    with path.open('rb') as stream:
        opened = os.fstat(stream.fileno()); plain(opened)
        require_opened_file(path, before, opened)
        while chunk := stream.read(1024*1024):
            count += len(chunk)
            if count > 2 * 1024**3: raise ValueError('Input file grew past read budget')
            result.update(chunk)
        after_handle = os.fstat(stream.fileno()); plain(after_handle)
        if read_signature(after_handle) != read_signature(opened): raise ValueError('Input changed during read')
    after = path.lstat(); plain(after)
    if read_signature(after) != read_signature(before) or count != before.st_size: raise ValueError('Input changed after read')
    return result.hexdigest(), count


def tree_digest(root, *, skip=frozenset()):
    root = directory(root); rows = []; total = 0
    def walk(current):
        nonlocal total
        for path in sorted(current.iterdir(), key=lambda x: x.name):
            info = path.lstat(); plain(info)
            if path.name in skip: continue
            if path.is_dir(): walk(path)
            else:
                digest, size = file_hash(path, info)
                rows.append([path.relative_to(root).as_posix(), digest, size]); total += size
                if len(rows) > MAX_FILES or total > MAX_TOTAL:
                    raise ValueError('Input tree exceeds bounded read budget')
    walk(root)
    return sha(canonical(rows))


def no_parent_configs(path):
    for parent in (path, *path.parents):
        for name in ('config', 'config.toml'):
            if (parent/'.cargo'/name).exists():
                raise ValueError('Unexpected inherited Cargo config: '+str(parent/'.cargo'/name))


def disjoint(paths):
    values = [(name, Path(value).resolve()) for name, value in paths]
    for i, (left, a) in enumerate(values):
        for right, b in values[i+1:]:
            if a == b or a in b.parents or b in a.parents:
                raise ValueError('Input/output paths overlap: '+left+' / '+right)


def validate_compat(value):
    expected = {'horizon_repository': 'https://github.com/horizon-rust/horizon.git',
                'mixins_repository': 'https://github.com/horizon-rust/horizon-mixins.git',
                'horizon_revision': HORIZON, 'mixins_revision': MIXINS}
    for key, want in expected.items():
        if value.get(key) != want:
            raise ValueError('Latest guarded playtest requires compatibility.json '+key+' = '+want+
                             '; this launcher does not rewrite revisions or bypass guards.')


def source_identity(path, repo, revision, lock, git, env):
    path = directory(path)
    def get(*args): return run([git, '-C', path, *args], cwd=path, env=env, capture=True)
    if get('rev-parse', 'HEAD') != revision: raise ValueError('Wrong official source HEAD: '+repo)
    if get('remote', 'get-url', 'origin').removesuffix('.git') != 'https://github.com/horizon-rust/'+repo:
        raise ValueError('Only the exact official HTTPS origin is admitted: '+repo)
    if get('status', '--porcelain', '--untracked-files=all', '--ignored=matching'):
        raise ValueError('Official checkout contains edits, untracked or ignored inputs: '+repo)
    if sha(read(path/'Cargo.lock')) != lock: raise ValueError('Official Cargo.lock differs: '+repo)
    attributes = Path(get('rev-parse', '--git-path', 'info/attributes'))
    if not attributes.is_absolute(): attributes = path/attributes
    if attributes.exists() or attributes.is_symlink():
        raise ValueError('Local Git archive attributes are not admitted: '+str(attributes))
    # stage.py exports these committed trees rather than compiling dirty checkout bytes.
    return {'head': revision, 'tree': get('rev-parse', 'HEAD^{tree}'), 'lock': lock,
            'files': tree_digest(path, skip=frozenset({'.git'}))}


def build_environment(original, toolchain, home):
    env = {k: v for k, v in original.items() if k.upper() not in SCRUB and
           not k.upper().startswith(('CARGO_BUILD_', 'CARGO_TARGET_', 'CARGO_PROFILE_',
                                     'CC_', 'CXX_', 'AR_', 'RANLIB_', 'CFLAGS_', 'CXXFLAGS_', 'HORIZON_', 'GIT_'))}
    env.update(PATH=str(toolchain/'bin')+os.pathsep+str(toolchain/'mingw64/bin')+os.pathsep+original.get('PATH',''),
               PLUTO_GUARD_RUSTFMT=str(toolchain/'bin/rustfmt.exe'),
               CARGO=str(toolchain/'bin/cargo.exe'), RUSTC=str(toolchain/'bin/rustc.exe'), RUSTDOC=str(toolchain/'bin/rustdoc.exe'),
               CARGO_HOME=str(home/'cargo-home-latest-gnu'), CARGO_NET_OFFLINE='true',
               CARGO_TARGET_DIR=str(home/'cargo-target-latest-gnu'),
               TMP=str(home/'tmp-latest-gnu'), TEMP=str(home/'tmp-latest-gnu'),
               TMPDIR=str(home/'tmp-latest-gnu'), PYTHONDONTWRITEBYTECODE='1',
               PYTHONUTF8='1', PYTHONIOENCODING='utf-8',
               CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=str(toolchain/'mingw64/bin/gcc.exe'),
               CC=str(toolchain/'mingw64/bin/gcc.exe'), CXX=str(toolchain/'mingw64/bin/g++.exe'),
               AR=str(toolchain/'mingw64/bin/ar.exe'), RANLIB=str(toolchain/'mingw64/bin/ranlib.exe'),
               WINDRES=str(toolchain/'mingw64/bin/windres.exe'),
               GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_SYSTEM=os.devnull,
               GIT_NO_REPLACE_OBJECTS='1', GIT_ATTR_NOSYSTEM='1',
               GIT_CONFIG_COUNT='1', GIT_CONFIG_KEY_0='core.attributesfile', GIT_CONFIG_VALUE_0=os.devnull)
    return env


def validate_rust(text):
    values = dict(line.split(': ', 1) for line in text.splitlines() if ': ' in line)
    if values.get('release') != VERSION or values.get('host') != TARGET:
        raise ValueError('Select official launcher Rust '+VERSION+' / '+TARGET+' toolchain; MSVC or another release is not this build contract.')


def feature_contract(stage):
    features = tomllib.loads(read(stage/'crates/horizon-client/Cargo.toml').decode()).get('features',{})
    if features.get('default') != ['audio','controllers']:
        raise ValueError('Unexpected default gameplay feature contract')
    return {'defaults':['audio','controllers'], 'extra':[MODS_FEATURE] if 'mods' in features else []}


def compile_args(cargo, stage, target):
    # Default audio/controllers remain enabled. This is the client, not the headless engine.
    args = [str(cargo), 'build', '--locked', '--offline', '--manifest-path', str(stage/'Cargo.toml'),
            '-p', 'horizon-client', '--bin', 'horizon-client', '--profile', PROFILE,
            '--target', TARGET, '--target-dir', str(target)]
    extras = feature_contract(stage)['extra']
    if extras: args += ['--features', ','.join(extras)]
    if (stage/'.cargo/config.toml').is_file(): args += ['--config', str(stage/'.cargo/config.toml')]
    return args


def ready_identity(inputs): return sha(canonical({'schema': 1, 'inputs': inputs}))


def pe_x64(path):
    with path.open('rb') as f:
        if f.read(2) != b'MZ': raise ValueError('Not a Windows executable')
        f.seek(0x3c); offset = f.read(4)
        if len(offset) != 4: raise ValueError('Truncated PE header')
        f.seek(struct.unpack('<I', offset)[0]); header = f.read(6)
    if header != b'PE\0\0\x64\x86': raise ValueError('Built executable is not Windows x64 PE')


def link_mode(build_only, resources):
    if build_only and resources is not None:
        raise ValueError('Build-only mode does not accept resources; omit --resources. No game will be started.')
    if not build_only and resources is None:
        raise ValueError('Game startup requires explicitly prepared --resources; use --build-only for link verification only.')
    return 'link-only' if build_only else 'playtest'


def execute(args):
    if os.name != 'nt' or platform.machine().upper() not in ('AMD64', 'X86_64'):
        raise ValueError('This launch route requires Windows 11 x64. Nothing was installed or compiled.')
    repo = directory(Path(__file__).parents[1]); pluto = directory(repo.parent/'pluto-lib')
    validate_compat(json.loads(read(repo/'compatibility.json')))
    guard_format.require_guard_routing(pluto)
    mode = link_mode(args.build_only, args.resources)
    horizon, mixins, toolchain = map(directory, (args.horizon, args.mixins, args.toolchain))
    resources = directory(args.resources) if args.resources is not None else None
    external_bundle = directory(args.pluto_bundle) if args.pluto_bundle else None
    rustfmt_archive = Path(os.path.abspath(args.rustfmt_archive))
    directory(rustfmt_archive.parent)
    home = Path(os.path.abspath(args.home))
    # All explicitly selected inputs must remain outside generated outputs and saves.
    disjoint([('home',home),('Horizon',horizon),('Mixins',mixins),('toolchain',toolchain),('repo',repo.parent),('rustfmt archive',rustfmt_archive)]
             + ([('resources',resources)] if resources is not None else []))
    if external_bundle is not None and (home == external_bundle or home in external_bundle.parents or external_bundle in home.parents): raise ValueError('Bundle/output overlap')
    directory(home.parent); no_parent_configs(home)
    for path in (horizon, mixins): no_parent_configs(path.parent)
    for path in ('bin/rustc.exe','bin/cargo.exe','bin/rustdoc.exe','bin/rustfmt.exe','mingw64/bin/gcc.exe',
                 'mingw64/bin/g++.exe','mingw64/bin/ar.exe','mingw64/bin/ranlib.exe','mingw64/bin/windres.exe'):
        if not (toolchain/path).is_file(): raise ValueError('Missing official launcher toolchain component: '+path+'; prepare it explicitly, no automatic installer is used.')
    if resources is not None and not (resources/'horizon-resources.json').is_file(): raise ValueError('Select explicitly prepared official Horizon 26.3 resources, not a source checkout or saves folder.')
    cargo_home = home/'cargo-home-latest-gnu'
    for name in ('config','config.toml'):
        if (cargo_home/name).exists(): raise ValueError('Unexpected isolated Cargo config: '+str(cargo_home/name))
    env = build_environment(os.environ, toolchain, home)
    for path in (home, cargo_home, home/'tmp-latest-gnu'):
        path.mkdir(parents=True, exist_ok=True); directory(path)
    # CWD at the volume root prevents home/repository Cargo config discovery.
    cwd = Path(home.anchor); no_parent_configs(cwd)
    # Reject toolchain links/reparse/special entries BEFORE executing even a version query.
    toolchain_before = tree_digest(toolchain)
    formatter = guard_format.admit(toolchain, rustfmt_archive)
    formatter_version = run([toolchain/'bin/rustfmt.exe', '--version'], cwd=cwd, env=env, capture=True)
    guard_format.validate_version(formatter_version)
    rust = run([toolchain/'bin/rustc.exe', '-vV'], cwd=cwd, env=env, capture=True); validate_rust(rust)
    cargo = run([toolchain/'bin/cargo.exe', '-vV'], cwd=cwd, env=env, capture=True)
    if not cargo.startswith('cargo '+VERSION+' '): raise ValueError('Cargo version differs from selected Rust toolchain')
    gcc = run([toolchain/'mingw64/bin/gcc.exe', '-dumpmachine'], cwd=cwd, env=env, capture=True)
    gcc_version = run([toolchain/'mingw64/bin/gcc.exe', '-dumpfullversion'], cwd=cwd, env=env, capture=True)
    if gcc != 'x86_64-w64-mingw32' or gcc_version != '16.2.0': raise ValueError('Expected official launcher WinLibs GCC 16.2.0 x64. No installer was run.')
    if tree_digest(toolchain) != toolchain_before: raise ValueError('Toolchain changed during version queries')
    import shutil
    git = shutil.which('git', path=env['PATH'])
    if not git: raise ValueError('Git is required; it was not installed automatically')
    stage_bytes = read(repo/'tools/stage.py')
    if sha(stage_bytes) != STAGE_SHA: raise ValueError('Guarded stage source differs from reviewed launch contract')
    stage_text = stage_bytes.decode()
    markers = ["'plan', '--workspace'", "pluto-lib/tools/check_native_plan.py", "'apply', '--workspace'"]
    positions = [stage_text.find(x) for x in markers]
    if min(positions)<0 or positions != sorted(positions): raise ValueError('Guarded stage plan/check/apply sequence is missing')
    def package_sources():
        return {'tconstruct':{name:tree_digest(repo/name,skip=frozenset({'__pycache__'})) for name in ('mixins','content','assets','tools','third_party','docs')},
                'pluto':{name:tree_digest(pluto/name,skip=frozenset({'__pycache__'})) for name in ('mixins','tools','pluto_lib')},
                'native_header':sha(read(pluto/'horizon-native.json')),
                'compatibility':sha(read(repo/'compatibility.json'))}
    source_before = package_sources()
    tc_mods,bundle,package_receipt = bundles.prepare(repo.parent,home,run,sys.executable,cwd,env,external_bundle)
    if package_sources()!=source_before: raise ValueError('Same-checkout package sources changed during preparation')
    def snapshot():
        if package_sources()!=source_before: raise ValueError('Package sources changed after generation')
        bundles.verify_prepared(tc_mods,bundle,package_receipt)
        return {'official_horizon': source_identity(horizon,'horizon',HORIZON,HORIZON_LOCK,git,env),
                'official_mixins': source_identity(mixins,'horizon-mixins',MIXINS,MIXINS_LOCK,git,env),
                'package_sources':package_sources(), 'packages':package_receipt,
                'compatibility':sha(read(repo/'compatibility.json')), 'pluto':fingerprint(bundle,pluto),
                'mode':mode,
                'resources':({'path':str(resources),'files':tree_digest(resources)} if resources is not None else None),
                'toolchain':{'path':str(toolchain),'files':tree_digest(toolchain),'rust':rust,'cargo':cargo,'gcc':gcc,'gcc_version':gcc_version},
                'formatter':{**guard_format.admit(toolchain,rustfmt_archive),'version':formatter_version},
                'target':TARGET,'profile':PROFILE,'default_features':True,'features':feature_contract(horizon),
                'stage_sha256':sha(read(repo/'tools/stage.py'))}
    initial = snapshot(); key = ready_identity(initial)
    build = home/'builds-latest-gnu'/key; ready = build/('.linked.json' if args.build_only else '.ready.json')
    game = build/'horizon-client.exe'; game_resources = build/'resources'
    def outputs():
        value = {'game':sha(read(game))}
        if not args.build_only: value['resources'] = tree_digest(game_resources)
        return value
    if build.exists(): directory(build)
    if not ready.exists():
        if build.exists(): raise ValueError('Incomplete generated build preserved at '+str(build)+'; inspect it before choosing a new output home. No cleanup is automatic.')
        build.mkdir(parents=True); no_parent_configs(build)
        stage = build/'stage'
        run([sys.executable, repo/'tools/stage.py','--horizon',horizon,'--mixins',mixins,
             '--mixins-revision',MIXINS,'--mods',tc_mods,'--pluto-bundle',bundle,'--output-dir',stage,
             '--cli-target-dir',home/'mixins-cli-latest-gnu'],cwd=cwd,env=env)
        # stage.py owns the mandatory complete-plan guard and stops before apply on rejection.
        if feature_contract(stage/'modded') != initial['features']: raise ValueError('Applied gameplay features differ from admitted source contract')
        run(compile_args(toolchain/'bin/cargo.exe',stage/'modded',home/'cargo-target-latest-gnu'),cwd=cwd,env=env)
        compiled = home/'cargo-target-latest-gnu'/TARGET/PROFILE/'horizon-client.exe'
        pe_x64(compiled); shutil.copyfile(compiled,game)
        if not args.build_only:
            overlay = build/'item-resources'
            run([sys.executable,repo/'tools/resources.py',resources,overlay],cwd=cwd,env=env)
            run([game,'--tconstruct-prepare',overlay,repo/'content',game_resources],cwd=cwd,env=env)
            run([sys.executable,repo/'tools/prepare_horizon_source_data.py',resources,game_resources/'minecraft-original-pack'],cwd=cwd,env=env)
            run([sys.executable,repo/'tools/prepare_bundled_forge.py',game_resources/'forge-original-pack'],cwd=cwd,env=env)
        if snapshot()!=initial: raise ValueError('Inputs changed; no ready marker was written')
        ready.write_text(json.dumps({'schema':1,'key':key,'inputs':initial,'outputs':outputs()},sort_keys=True,indent=2)+'\n',encoding='utf-8')
    directory(build)
    if not args.build_only: directory(game_resources)
    receipt = json.loads(read(ready))
    if receipt.get('key')!=key or receipt.get('inputs')!=initial or receipt.get('outputs')!=outputs():
        raise ValueError('Cached input/output identity mismatch; nothing was launched or deleted')
    if snapshot()!=initial: raise ValueError('Inputs changed before startup')
    pe_x64(game)
    if args.build_only:
        print(json.dumps({'scope':'guarded-windows-link-only','game_started':False,
                          'resources_prepared':False,'receipt':str(ready),
                          'executable':str(game),'sha256':sha(read(game))},sort_keys=True),flush=True)
        return
    game_dir = home/('game-latest-'+HORIZON[:12]); game_dir.mkdir(exist_ok=True)
    directory(game_dir); env['HORIZON_HOME']=str(game_dir)
    print('Starting isolated latest world. Saves: '+str(game_dir/'saves'),flush=True)
    run([game,'--resources',game_resources,'--world','TConstruct-Playtest','--seed','42','--creative'],cwd=game_dir,env=env)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ('horizon','mixins','toolchain','home'): p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--resources',type=Path,help='required for startup; omit with --build-only')
    p.add_argument('--build-only',action='store_true',help='same guarded stage and GNU link, without assets, resource preparation or game startup')
    p.add_argument('--rustfmt-archive',type=Path,required=True,help='existing official Rust 1.99.1 GNU rustfmt tar.gz; verified/read only, never installed automatically')
    p.add_argument('--pluto-bundle',type=Path,help='optional external bundle, accepted only if byte-identical to same-checkout generated Pluto')
    try: execute(p.parse_args())
    except (OSError,ValueError,subprocess.CalledProcessError) as exc:
        p.exit(1,'Latest playtest stopped: '+str(exc)+'\nNo existing build cleanup, source fetch, installation or guard bypass was attempted. Normal/legacy worlds were not selected; an already started isolated playtest may have written its own world.\n')

if __name__=='__main__': main()
