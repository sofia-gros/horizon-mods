#!/usr/bin/env python3
"""Windows source-only phases: PS parse and 20+12+11 pure cases. Never Cargo/game."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
import time

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
FLOOR=10*1024**3+128*1024**2
LOG_LIMIT=128*1024
TIMEOUT=40
SUITES=[('latest','tconstruct/tests/test_windows_latest_launch.py',20),
        ('historical','tconstruct/tests/test_windows_pluto_cache.py',12),
        ('bundles','tconstruct/tests/test_windows_playtest_bundles.py',11)]


def check_suites(groups):
    if len(groups)!=len(SUITES):raise ValueError('Wrong pure suite inventory')
    for group,(name,path,count) in zip(groups,SUITES):
        if group['name']!=name or group['path']!=path or len(group['names'])!=count or len(set(group['names']))!=count:
            raise ValueError('Wrong fixed suite/path/count')


def sha(raw):return hashlib.sha256(raw).hexdigest()
def snapshot(root):
    files={}
    def walk(path):
        for child in sorted(path.iterdir(),key=lambda p:p.name):
            if child==root/'.git':continue
            info=child.lstat()
            if stat.S_ISLNK(info.st_mode) or getattr(info,'st_file_attributes',0)&0x400:raise ValueError('Source link/reparse entry')
            if stat.S_ISDIR(info.st_mode):walk(child)
            elif stat.S_ISREG(info.st_mode):files[child.relative_to(root).as_posix()]=sha(child.read_bytes())
            else:raise ValueError('Source special entry')
    walk(root)
    return files


def disk_guard(paths):
    for path in paths:
        if shutil.disk_usage(path).free<FLOOR:raise RuntimeError('Disk floor reached')


def phase(name,argv,work,env,receipt):
    from winjob import Job
    log=work/(name+'.log');paths=[ROOT,work,Path(ROOT.anchor),Path(sys.executable).parent]
    guard=lambda:disk_guard(paths)
    job=None;process=None;assigned=False;start=time.monotonic()
    item={'name':name,'argv':argv,'exit':None,'process_succeeded':False};receipt['phases'].append(item)
    try:
        guard();job=Job()
        with log.open('xb') as output:
            process=subprocess.Popen([sys.executable,str(__file__),'--worker',*argv],cwd=work,env=env,
                stdin=subprocess.PIPE,stdout=output,stderr=subprocess.STDOUT,
                creationflags=subprocess.CREATE_NEW_PROCESS_GROUP)
            # Worker cannot spawn the known phase process until assignment succeeds.
            job.assign(process);assigned=True
            process.stdin.write(b'G');process.stdin.close()
            while process.poll() is None:
                guard()
                if time.monotonic()-start>TIMEOUT:raise RuntimeError('Phase timeout')
                if log.stat().st_size>LOG_LIMIT:raise RuntimeError('Phase log limit')
                time.sleep(0.05)
            item['exit']=process.returncode
        guard()
    finally:
        try:
            if assigned:
                job.terminate()
                if process is not None:process.wait(timeout=2)
                job.wait_empty(guard);item['job_empty']=True
            elif process is not None:
                # The unassigned worker is still behind stdin and has no child.
                process.kill();process.wait(timeout=2)
        finally:
            if job is not None:job.close()
        item['seconds']=time.monotonic()-start
        if log.exists():
            size=log.stat().st_size;item['log_bytes']=size
            if size<=LOG_LIMIT:item['log_sha256']=sha(log.read_bytes())
    raw=log.read_bytes()
    if len(raw)>LOG_LIMIT:raise RuntimeError('Phase log limit')
    if item['exit']!=0:
        print(raw[-8192:].decode('utf-8',errors='replace'),flush=True)
        raise RuntimeError(name+' failed')
    item['process_succeeded']=True
    return raw


def worker():
    if sys.stdin.buffer.read(1)!=b'G':return 97
    # Parent supplied one fixed known phase argv, never a shell string.
    return subprocess.run(sys.argv[2:],check=False).returncode


def execute(receipt):
    import validate
    if os.name!='nt' or sys.version_info[:2]!=(3,12):raise ValueError('Requires existing Windows x64 Python3.12; no installer is used')
    config=json.loads((HERE/'inputs.json').read_text())
    if os.environ.get('GITHUB_REPOSITORY')!='sofia-gros/horizon-mods' or os.environ.get('GITHUB_EVENT_NAME')!='push' or os.environ.get('GITHUB_REF')!='refs/heads/'+config['branch']:
        raise ValueError('Wrong repository/event/exact branch')
    check_suites(config['suites'])
    before=snapshot(ROOT)
    for path,digest in config['source_sha256'].items():
        if before.get(path)!=digest:raise ValueError('Frozen input drift: '+path)
    receipt['source_before_sha256']=sha(json.dumps(before,sort_keys=True).encode())
    receipt['source_files']=len(before);receipt['head']=os.environ.get('GITHUB_SHA')
    receipt['python']=sys.version
    with tempfile.TemporaryDirectory(prefix='windows-source-',dir=os.environ['RUNNER_TEMP']) as temp:
        work=Path(temp);env=dict(os.environ);env.update(TEMP=str(work),TMP=str(work),TMPDIR=str(work),PYTHONDONTWRITEBYTECODE='1')
        ps=shutil.which('powershell.exe')
        if not ps:raise ValueError('Existing Windows PowerShell unavailable')
        env['PLUTO_PS_ENTRY']=str(ROOT/'tconstruct/tools/play-windows.ps1')
        command="$tokens=$null; $errors=$null; $null=[System.Management.Automation.Language.Parser]::ParseFile($env:PLUTO_PS_ENTRY,[ref]$tokens,[ref]$errors); if ($errors.Count -ne 0) { $errors | ForEach-Object { Write-Output $_.Message }; exit 1 }; Write-Output 'PS_PARSE_OK'"
        raw=phase('powershell-parse',[ps,'-NoLogo','-NoProfile','-NonInteractive','-Command',command],work,env,receipt)
        receipt['gates']['powershell']=validate.powershell(raw)
        for group in config['suites']:
            raw=phase(group['name'],[sys.executable,str(ROOT/group['path']),'-v'],work,env,receipt)
            receipt['gates'][group['name']]=validate.suite(raw,group['names'])
        if set(receipt['gates'])!={'powershell',*[x['name'] for x in config['suites']]}:raise ValueError('Missing mandatory phase')
    after=snapshot(ROOT)
    if before!=after:raise ValueError('Source changed during pure validation')
    receipt['source_after_sha256']=sha(json.dumps(after,sort_keys=True).encode());receipt['verified']=True
    receipt['passed']=True


def main():
    receipt={'schema':1,'passed':False,'verified':False,'scope':'PowerShell syntax and pure Python only','native_executed':False,'gates':{},'phases':[]}
    try:execute(receipt)
    except Exception as error:
        receipt['error']=type(error).__name__+': '+str(error)
    finally:
        if 'source_before_sha256' in receipt:
            try:
                receipt['source_after_sha256']=sha(json.dumps(snapshot(ROOT),sort_keys=True).encode())
                receipt['verified']=receipt['source_after_sha256']==receipt['source_before_sha256']
                if not receipt['verified']:receipt['passed']=False
            except Exception as error:
                receipt['verified']=False;receipt['passed']=False;receipt['final_seal_error']=type(error).__name__+': '+str(error)
        print('FINAL_RECEIPT '+json.dumps(receipt,sort_keys=True),flush=True)
    return 0 if receipt['passed'] else 1

if __name__=='__main__':
    sys.exit(worker() if sys.argv[1:2]==['--worker'] else main())
