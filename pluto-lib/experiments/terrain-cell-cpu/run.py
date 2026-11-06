#!/usr/bin/env python3
"""Explicitly scheduled CPU fixture execution; continuous disk floor, no GPU.

Do not invoke without the shared native slot and a separately validated/applied
fixture stage. This runner never prepares/applies or downloads anything.
"""
import argparse
import importlib.util
import json
import math
import os
from pathlib import Path
import re
import signal
import subprocess
import time
HERE=Path(__file__).resolve().parent
FLOOR=10*1024**3+128*1024**2
TEST='mesher::pluto_terrain_cell_cpu::tests::real_native_mesh_differential_and_timing'
def checked_log(text,mode):
    if f'PLUTO_TERRAIN_MODE {mode} complete;' not in text or not re.search(r'test result: ok\. 1 passed; 0 failed;',text):
        raise ValueError('expected exactly one exercised native test and mode completion, not a skipped/zero-test pass')
    timings=[json.loads(line.split('PLUTO_TERRAIN_CPU ',1)[1]) for line in text.splitlines() if 'PLUTO_TERRAIN_CPU ' in line]
    expected={'no-provider'} if mode=='no-provider' else {'registered-ordinary','provider-no-claim','claimed-mixed'}
    if {x['case'] for x in timings}!=expected or len(timings)!=len(expected):raise ValueError('missing or repeated timing cases')
    for x in timings:
        if not math.isfinite(x['median_ratio']) or any(len(x[k])!=7 or any(not isinstance(n,int) or n<=0 for n in x[k]) for k in ['baseline_ns_per_section','adapter_ns_per_section']):raise ValueError('invalid timing result')
    return timings

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--workspace',type=Path,required=True);p.add_argument('--fixture',type=Path,required=True);p.add_argument('--target',type=Path,required=True);p.add_argument('--iterations',type=int,default=64);a=p.parse_args()
    if not 1<=a.iterations<=4096:raise ValueError('iterations must be bounded')
    if os.environ.get('RUSTFLAGS') or os.environ.get('CARGO_ENCODED_RUSTFLAGS'):raise ValueError('unreviewed Rust flags')
    spec=importlib.util.spec_from_file_location('terrain_cpu_prepare',HERE/'prepare.py');prepare=importlib.util.module_from_spec(spec);spec.loader.exec_module(prepare)
    evidence=prepare.verify(a.fixture.resolve());work=a.workspace.resolve();target=a.target.resolve();out=a.fixture.resolve()
    if not (work/'Cargo.toml').is_file() or not target.is_dir():raise ValueError('expected existing applied stage and shared target')
    env=os.environ.copy();env.update(CARGO_TARGET_DIR=str(target),CARGO_BUILD_JOBS='2',CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_TEST_OPT_LEVEL='0',PLUTO_TERRAIN_FIXTURE_RESOURCES=str(out/'resources'),PLUTO_TERRAIN_PROFILE_LABEL='test; opt-level=0; debug=0; incremental=0',PLUTO_TERRAIN_TIMING_ITERS=str(a.iterations))
    command=['cargo','test','--offline','--locked','--manifest-path',str(work/'Cargo.toml'),'-p','horizon-client-render','--lib',TEST,'--','--exact','--nocapture','--test-threads=1']
    free=lambda:os.statvfs(target).f_bavail*os.statvfs(target).f_frsize
    result={'scope':'Unoptimized real CPU mesher diagnostic; original synthetic resources, no GPU/production-performance proof','fixture_bundle_sha256':evidence['bundle_sha256'],'command':command,'profile':{k:env[k] for k in ['CARGO_BUILD_JOBS','CARGO_INCREMENTAL','CARGO_PROFILE_TEST_DEBUG','CARGO_PROFILE_TEST_OPT_LEVEL']},'runs':[]}
    for mode in ['no-provider','registered']:
        minimum=free();start=time.monotonic()
        if minimum<FLOOR:raise ValueError('continuous disk floor unavailable before start')
        logpath=out/f'native-{mode}.log';env['PLUTO_TERRAIN_FIXTURE_MODE']=mode
        with logpath.open('x') as log:
            process=subprocess.Popen(command,cwd=work,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            stopped=False
            while process.poll() is None:
                minimum=min(minimum,free())
                if minimum<FLOOR:
                    os.killpg(process.pid,signal.SIGINT);stopped=True;process.wait();break
                time.sleep(0.25)
        entry={'mode':mode,'exit':process.returncode,'seconds':time.monotonic()-start,'minimum_free_bytes':minimum,'disk_guard_stop':stopped,'log':str(logpath)}
        result['runs'].append(entry)
        try:
            if process.returncode or stopped:raise ValueError('native test did not finish successfully')
            entry['timings']=checked_log(logpath.read_text(),mode)
        except ValueError as error:
            entry['error']=str(error);(out/'native-result.json').write_text(json.dumps(result,indent=2)+'\n');raise
        (out/'native-result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
if __name__=='__main__':
    try:main()
    except (ValueError,OSError,KeyError) as error:raise SystemExit(str(error))
