#!/usr/bin/env python3
"""Ten legacy response value contracts on actual Text; no Game or LAN proof."""
import importlib.util
import json
import os
from pathlib import Path
import re
import shlex
import sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
BRANCH='checkpoint/guide-reply-value-ci-20261106'
REPOSITORY='sofia-gros/horizon-mods'
BASE='b5bd8131ca371af3ceaeee471741f6449856ca02'
PREP='pluto-lib/experiments/guide-legacy-response-bridge/wire-fixture'
API_PATH='pluto-lib/mixins/pluto_native_creative/inventory-api'
UTILITY=['pluto-lib/experiments/guide-storage-ci/run.py','tconstruct/tests/guide-identity-ci/run.py']
NATIVE=['horizon-client-core','pluto-inventory-api']
NAME='pluto-guide-reply-tests'
RUNNER_TESTS=24
REQUIRED={'runner','preparer21','source6','toolchain','official','mod_inputs','metadata','compile','inventory21','reply10'}

def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module

old=load('reviewed_guide_runner',ROOT/UTILITY[0])
prep=load('frozen_reply_preparer',ROOT/PREP/'prepare.py')
base,Runner,sha,seal=old.base,old.Runner,old.sha,old.seal

def config():
    value=json.loads((HERE/'inputs.json').read_text())
    if (value['repository'],value['branch'],value['base'])!=(REPOSITORY,BRANCH,BASE):raise ValueError('Wrong immutable source scope')
    expected=set(UTILITY)|{PREP+'/'+n for n in ['prepare.py','test_prepare.py','prepare-inputs.json','README.md','render_wire.py','regression_tests.rs','wire-inputs.json','test_wire_source.py','source-extraction.json']}|{
        'pluto-lib/experiments/guide-reply-value-ci/'+n for n in ['run.py','test_runner.py','README.md']}
    expected|={'pluto-lib/experiments/guide-legacy-response-bridge/'+n for n in ['native-bridge.rs','consumer.rs']}
    expected.add('.github/workflows/guide-reply-value-wip.yml')
    if set(value['source_sha256'])!=expected:raise ValueError('Wrong controller source set')
    for name,digest in value['source_sha256'].items():
        if sha(prep.read(ROOT,name))!=digest:raise ValueError('Changed controller input: '+name)
    p=prep.config(); wire=json.loads((ROOT/PREP/'wire-inputs.json').read_text())
    if value['official']!={'url':old.HORIZON_URL,'revision':prep.HORIZON,'lock_git_blob':p['official_git_blobs']['Cargo.lock']}:raise ValueError('Wrong official input')
    p['inventory_names']=sorted(n for group in wire['expected_tests'].values() for n in group)
    p['reply_names']=wire['expected_tests']['regression_tests']
    if len(p['inventory_names'])!=21 or len(set(p['inventory_names']))!=21 or len(p['reply_names'])!=10:raise ValueError('Wrong bounded inventory')
    if set(p['reply_names'])!={n for n in p['inventory_names'] if n.startswith('regression_tests::')}:raise ValueError('Wrong reply filter')
    return value,p

def workspace_diagnostic(metadata):
    # Bounded public ownership facts only. Full metadata stdout already has a
    # byte/hash receipt; truncation here never affects strict admission.
    members=metadata.get('workspace_members',[])
    defaults=metadata.get('workspace_default_members',[])
    known={p.get('id'):p for p in metadata.get('packages',[])}
    def rows(ids):
        result=[]
        for ident in ids[:8]:
            package=known.get(ident,{})
            result.append({key:str(value)[:240] for key,value in {
                'id':ident,'name':package.get('name'),
                'manifest_path':package.get('manifest_path')}.items()})
        return result
    return {'workspace_root':str(metadata.get('workspace_root'))[:240],
            'member_count':len(members),'default_member_count':len(defaults),
            'members':rows(members),'default_members':rows(defaults),
            'truncated':len(members)>8 or len(defaults)>8 or
                any(len(str(value))>240 for ids in [members,defaults] for ident in ids[:8]
                    for value in [ident,known.get(ident,{}).get('name'),known.get(ident,{}).get('manifest_path')]) or
                len(str(metadata.get('workspace_root')))>240}


def event(env,event,head):
    if (env.get('GITHUB_ACTIONS')!='true' or env.get('GITHUB_REPOSITORY')!=REPOSITORY or
        env.get('GITHUB_EVENT_NAME')!='push' or env.get('GITHUB_REF')!='refs/heads/'+BRANCH or
        env.get('GITHUB_SHA')!=head or event.get('repository',{}).get('private') is not False or
        event.get('repository',{}).get('full_name')!=REPOSITORY):raise ValueError('Require exact public dedicated push')

def inventory(log,names):
    if len(names)!=21 or len(set(names))!=21:raise ValueError('Wrong expected inventory')
    if log!=''.join(n+': test\n' for n in sorted(names)):raise ValueError('Wrong entire terse inventory')

def tests(log,names):
    rows=re.findall(r'^test (\S+) \.\.\. (\w+)\s*$',log,re.M)
    if len(names)!=10 or len(set(names))!=10 or len(rows)!=10 or {n for n,_ in rows}!=set(names) or any(s!='ok' for _,s in rows):raise ValueError('Wrong ten passing names')
    if re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',log,re.M)!=[('10','0','0','0','11')]:raise ValueError('Wrong exact selected result')

def compiler_arguments(line):
    # Cargo's leading shell-style environment values are not Rust inputs.
    # In particular CARGO_PKG_HOMEPAGE=https://serde.rs and RUSTC=/x/rustc
    # must not be confused with a source path or the executable token.
    tokens=shlex.split(line.split('Running `',1)[1].rsplit('`',1)[0])
    at=0
    while at<len(tokens) and re.match(r'^[A-Za-z_][A-Za-z0-9_]*=',tokens[at]):at+=1
    command=tokens[at:]
    if not command:return None
    if Path(command[0]).name!='rustc':
        if '--crate-name' in command:raise ValueError('Unexpected compiler command')
        return None
    if sum(Path(token).name=='rustc' for token in command)!=1:
        raise ValueError('Ambiguous rustc executable tokens')
    return command[1:]

def compilers(log,metadata,fixture):
    wanted={}
    for name in NATIVE+[NAME]:
        packages=[p for p in metadata['packages'] if p['name']==name]
        if len(packages)!=1:raise ValueError('Wrong compiler package')
        targets=[t for t in packages[0]['targets'] if t['kind']==['lib']]
        if len(targets)!=1:raise ValueError('Wrong native/fixture target')
        wanted[name.replace('-','_')]=Path(targets[0]['src_path']).resolve()
    result=[]
    for line in log.splitlines():
        if 'Running `' not in line or 'rustc' not in line:continue
        args=compiler_arguments(line)
        if args is None or '--crate-name' not in args:continue
        if args.count('--crate-name')!=1 or args.index('--crate-name')+1==len(args):raise ValueError('Malformed crate-name argument')
        name=args[args.index('--crate-name')+1]
        if name not in wanted:continue
        paths=[(fixture/a).resolve() for a in args if a.endswith('.rs')]
        if paths!=[wanted[name]] or ('--test' in args)!=(name==NAME.replace('-','_')):
            detail={'crate':name,'source_count':len(paths),'sources':[str(p)[:240] for p in paths[:4]],
                    'expected':str(wanted[name])[:240],'test':'--test' in args,'command_sha256':sha(line.encode())}
            print('COMPILER_SOURCE_MISMATCH',json.dumps(detail,sort_keys=True),flush=True)
            raise ValueError('Wrong cold compiler source/profile')
        result.append({'crate':name,'source':str(paths[0]),'test':'--test' in args,'command_sha256':sha(line.encode())})
    if len(result)!=3 or {r['crate'] for r in result}!=set(wanted):raise ValueError('Missing/duplicate cold compiler')
    return result

def artifact(log,metadata,target):
    rows=[json.loads(line) for line in log.splitlines()]
    packages=[p for p in metadata['packages'] if p['name']==NAME]
    if len(packages)!=1:raise ValueError('Wrong artifact owner')
    candidates=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('package_id')==packages[0]['id'] and r.get('profile',{}).get('test') is True and r.get('executable')]
    if len(candidates)!=1:raise ValueError('Missing/duplicate fixture executable')
    row=candidates[0];path=Path(row['executable'])
    if row.get('fresh') is not False or row['target']['kind']!=['lib'] or Path(row['target']['src_path']).resolve()!=Path(packages[0]['manifest_path']).parent/'lib.rs' or not path.is_absolute() or not path.resolve().is_relative_to(target.resolve()):raise ValueError('Wrong executable path/target')
    return path

def outputs(target):
    return {p.relative_to(target).as_posix():seal(p) for p in sorted(target.glob('debug/build/*/out'))}

def python_pass(log,count):
    old.python_pass(log,count)
    for line in log.splitlines():
        if not line or re.fullmatch(r'test_\S+ .* \.\.\. ok',line) or re.fullmatch(r'-+',line) or re.fullmatch(r'Ran \d+ tests in .+',line) or line=='OK':continue
        raise ValueError('Unexpected Python output')


def export_mod_inputs(runner,store,api,legacy,p):
    # Read only one fixed horizon-mods commit; no checkout or engine fork.
    runner.run('mod-init',['git','init',str(store)])
    runner.run('mod-origin',['git','-C',str(store),'remote','add','origin','https://github.com/'+REPOSITORY+'.git'])
    runner.run('mod-fetch',['git','-C',str(store),'fetch','--depth','1','origin',p['api_source_commit']],timeout=600)
    if runner.run('mod-head',['git','-C',str(store),'rev-parse','FETCH_HEAD']).strip()!=p['api_source_commit']:raise ValueError('Wrong source commit')
    prefix=API_PATH+'/'
    listed=runner.run('mod-api-tree',['git','-C',str(store),'ls-tree','-r','--name-only',p['api_source_commit'],'--',API_PATH])
    if listed!=''.join(prefix+n+'\n' for n in sorted(p['api_git_blobs'])):raise ValueError('Wrong full API file set')
    wanted=[(prefix+n,api/n,digest) for n,digest in p['api_git_blobs'].items()]
    wanted += [(path,legacy/path,None) for path in p['legacy_paths'].values()]
    for index,(path,destination,digest) in enumerate(wanted):
        phase='mod-source-'+str(index)
        runner.run(phase,['git','-C',str(store),'show',p['api_source_commit']+':'+path],separate_stdout=True)
        raw=(runner.work/(phase+'.log')).read_bytes()
        if digest is not None and prep.blob(raw)!=digest:raise ValueError('Wrong Git source blob: '+path)
        destination.parent.mkdir(parents=True,exist_ok=True)
        with destination.open('xb') as stream:stream.write(raw)
    if prep.files(api)!=p['api_git_blobs']:raise ValueError('API export drift')
    # The legacy renderer independently checks all three SHA256 values.
    prep.rendered(legacy,p)
    return {'commit':p['api_source_commit'],'api_files':len(p['api_git_blobs']),'legacy_files':len(p['legacy_paths'])}


def prepare_command(host,api,legacy,fixture):
    if os.environ.get('PLUTO_REPLY_CONTROLLER')!='reply-value-v1' or os.getpid()!=os.getpgrp():raise ValueError('Require guarded outer process group')
    event(os.environ,json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text()),base.git('rev-parse','HEAD'))
    prep.prepare(host,api,legacy,fixture)
    print('Prepared exact selected inputs; no metadata or native invocation.',flush=True)

def execute(report):
    value,p=config();head=base.git('rev-parse','HEAD')
    event(os.environ,json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text()),head);base.clean()
    original=seal(ROOT);work=Path(os.environ['RUNNER_TEMP']).resolve()/('guide-reply-value-'+head)
    old.independent_roots(ROOT,work)
    env=os.environ.copy()
    for key in ['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','CARGO_NET_OFFLINE','CARGO','RUSTC','GIT_DIR','GIT_INDEX_FILE','GIT_WORK_TREE']:env.pop(key,None)
    env.update(RUSTUP_TOOLCHAIN='1.95.0',CARGO_HOME=str(work/'cargo-home'),CARGO_TARGET_DIR=str(work/'target'),
        CARGO_BUILD_JOBS='1',CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',
        TMPDIR=str(work/'tmp'),PYTHONDONTWRITEBYTECODE='1',CARGO_TERM_COLOR='never',PLUTO_REPLY_CONTROLLER='reply-value-v1')
    runner=Runner(work,env);runner.guard(reserve=2*1024**3)
    work.mkdir();(work/'cargo-home').mkdir();(work/'tmp').mkdir()
    host=work/'official';fixture=work/'fixture';target=work/'target'
    api=work/'mod-inputs/inventory-api';legacy=work/'mod-inputs/legacy';modstore=work/'mod-git'
    runner.env.update(GUIDE_WIRE_INPUTS=str(legacy),GUIDE_WIRE_BRIDGE=str(ROOT/PREP).rsplit('/wire-fixture',1)[0])
    mod_before=None
    host_before=None;fixture_before=None;generated=None;binary=None;compiled=None;metadata=None;g=report['gates']
    try:
        for gate,path,count in [('runner',HERE/'test_runner.py',RUNNER_TESTS),('preparer21',ROOT/PREP/'test_prepare.py',21)]:
            python_pass(runner.run(gate,[sys.executable,str(path),'-v']),count);g[gate]={'passed':True,'tests':count}
        runner.run('toolchain',['rustup','toolchain','install','1.95.0','--profile','minimal','--no-self-update'],timeout=600)
        g['toolchain']=runner.run('compiler-identity',['rustc','--version','--verbose']).strip()
        runner.run('official-init',['git','init',str(host)])
        runner.run('official-origin',['git','-C',str(host),'remote','add','origin',value['official']['url']])
        runner.run('official-fetch',['git','-C',str(host),'fetch','--depth','1','origin',prep.HORIZON],timeout=600)
        runner.run('official-checkout',['git','-C',str(host),'checkout','--detach',prep.HORIZON])
        actual=runner.run('official-head',['git','-C',str(host),'rev-parse','HEAD']).strip()
        origin=runner.run('official-url',['git','-C',str(host),'remote','get-url','origin']).strip()
        if actual!=prep.HORIZON or origin!=old.HORIZON_URL:raise ValueError('Wrong official Git identity')
        if runner.run('official-clean',['git','-C',str(host),'status','--porcelain','--untracked-files=all','--ignored=matching']).strip():raise ValueError('Dirty official checkout')
        prep.check_official(host,p);host_before=seal(host)
        g['official']={'revision':actual,'selected_files':len(p['official_git_blobs']),'lock_sha256':sha((host/'Cargo.lock').read_bytes())}
        runner.run('locked-fetch',['cargo','fetch','--locked','--manifest-path',str(host/'Cargo.toml')],cwd=host,timeout=900)
        g['mod_inputs']=export_mod_inputs(runner,modstore,api,legacy,p);mod_before=seal(work/'mod-inputs')
        python_pass(runner.run('source6',[sys.executable,str(ROOT/PREP/'test_wire_source.py'),'-v']),6);g['source6']={'passed':True,'tests':6}
        runner.run('prepare',[sys.executable,str(HERE/'run.py'),'--prepare',str(host),str(api),str(legacy),str(fixture)])
        raw=runner.run('fixture-metadata',['cargo','metadata','--offline','--format-version','1','--manifest-path',str(fixture/'Cargo.toml')],cwd=fixture,separate_stdout=True)
        metadata=json.loads(raw)
        print('WORKSPACE_METADATA',json.dumps(workspace_diagnostic(metadata),sort_keys=True),flush=True)
        prep.verify(fixture,metadata)
        locked=runner.run('locked-metadata',['cargo','metadata','--locked','--offline','--format-version','1','--manifest-path',str(fixture/'Cargo.toml')],cwd=fixture,separate_stdout=True)
        if json.loads(locked)!=metadata:raise ValueError('Resolved metadata changed')
        fixture_before=seal(fixture);g['metadata']={'sha256':sha(raw.encode()),'packages':len(metadata['packages']),'lock_sha256':sha((fixture/'Cargo.lock').read_bytes())}
        raw=runner.run('native-compile',['cargo','test','--lib','--no-run','--locked','--offline','--manifest-path',str(fixture/'Cargo.toml'),'-vv','--message-format=json'],cwd=fixture,timeout=1800,separate_stdout=True)
        witnesses=compilers((work/'native-compile.stderr.log').read_text(),metadata,fixture)
        binary=artifact(raw,metadata,target);compiled=base.binary_seal(binary);generated=outputs(target)
        g['compile']={'passed':True,'actual_compilers':witnesses,'binary':compiled}
        if seal(fixture)!=fixture_before or seal(host)!=host_before or seal(ROOT)!=original:raise ValueError('Source changed during compile')
        inventory(runner.run('inventory',[str(binary),'--list','--format','terse']),p['inventory_names'])
        if base.binary_seal(binary)!=compiled or outputs(target)!=generated:raise ValueError('Binary/generated drift after inventory')
        g['inventory21']={'names':sorted(p['inventory_names']),'tests':21}
        tests(runner.run('reply10',[str(binary),'regression_tests::','--test-threads=1']),p['reply_names'])
        g['reply10']={'names':sorted(p['reply_names']),'tests':10,'ignored':0,'filtered':11}
    except BaseException as exc:base.error(report,exc)
    finally:
        try:
            base.clean();config()
            if seal(ROOT)!=original:raise ValueError('Mod source drift')
            if host_before is not None and seal(host)!=host_before:raise ValueError('Official source drift')
            if mod_before is not None and seal(work/'mod-inputs')!=mod_before:raise ValueError('Mod input export drift')
            if fixture_before is not None and seal(fixture)!=fixture_before:raise ValueError('Fixture/lock drift')
            if metadata is not None:prep.verify(fixture,metadata)
            if generated is not None and outputs(target)!=generated:raise ValueError('OUT_DIR drift')
            if compiled is not None and base.binary_seal(binary)!=compiled:raise ValueError('Executable drift')
            runner.guard();report['verified']=True
        except BaseException as exc:base.error(report,exc)
        report.update(minimum_free_bytes=runner.minimum,source_seal_sha256=sha(json.dumps(original,sort_keys=True).encode()),
            official_seal_sha256=sha(json.dumps(host_before,sort_keys=True).encode()) if host_before else None,
            mod_inputs_seal_sha256=sha(json.dumps(mod_before,sort_keys=True).encode()) if mod_before else None,
            fixture_seal_sha256=sha(json.dumps(fixture_before,sort_keys=True).encode()) if fixture_before else None,
            output_seal_sha256=sha(json.dumps(generated,sort_keys=True).encode()) if generated is not None else None)

def main():
    if len(sys.argv)>1:
        if len(sys.argv)!=6 or sys.argv[1]!='--prepare':raise ValueError('Unknown child command')
        prepare_command(*(Path(arg) for arg in sys.argv[2:]));return
    report={'head':os.environ.get('GITHUB_SHA'),'passed':False,'verified':False,'error':None,'gates':{},
        'scope':'Ten legacy/proposed reply value cases using actual 41f core Text and the actual API Payload. Eleven old wire/identity tests are compiled/inventoried only. No Game acquisition, registry, observer, UI/lifetime, transport or installed package proof.'}
    try:
        execute(report)
        if set(report['gates'])!=REQUIRED:raise ValueError('Missing required gates')
    except BaseException as exc:base.error(report,exc)
    report['passed']=report['error'] is None and report['verified']
    try:
        with open(os.environ['GITHUB_STEP_SUMMARY'],'a') as f:f.write('## Actual Text legacy reply ten\n\n```json\n'+json.dumps(report,indent=2)+'\n```\n')
    except BaseException as exc:base.error(report,exc);report['passed']=False
    print('FINAL_RECEIPT',json.dumps(report,sort_keys=True),flush=True)
    if not report['passed']:raise SystemExit(1)
if __name__=='__main__':main()
