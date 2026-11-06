"""Bounded numeric CPU samples for one known owned CLI PID, never a system scan."""
import json,math,os,signal,stat,subprocess,sys,time
from pathlib import Path

MAX_BYTES=64*1024
MAX_SAMPLES=128
INTERVAL=5.0
BRANCH='refs/heads/checkpoint/plutolib-language-root-ci-20261106'

class ObservationInterrupted(Exception):pass
def interrupted(_signum,_frame):raise ObservationInterrupted()

def read_stat(pid):
    # stat includes comm; discard it without recording it. Never read cmdline,
    # environ, status, fd links or another process selected by a filesystem scan.
    with Path('/proc',str(pid),'stat').open('r') as stream:raw=stream.read(4097)
    if len(raw)>4096:raise ValueError('stat_bound')
    split=raw.rfind(')');left=raw.find('(')
    if left<0 or split<=left or int(raw[:left].strip())!=pid:raise ValueError('stat_format')
    fields=raw[split+1:].split()
    if len(fields)<22:raise ValueError('stat_fields')
    values={key:int(fields[index]) for key,index in
      [('ppid',1),('pgid',2),('session',3),('user_ticks',11),('system_ticks',12),('start_ticks',19)]}
    if any(v<0 for v in values.values()):raise ValueError('stat_negative')
    return values

def identity(value,owner,pgid,session,birth=None):
    if (value['ppid'],value['pgid'],value['session'])!=(owner,pgid,session):raise ValueError('not_owned_child_group')
    if birth is not None and value['start_ticks']!=birth:raise ValueError('pid_identity_changed')

def emit(stream,value):
    data=json.dumps(value,sort_keys=True,allow_nan=False)+'\n'
    if stream.tell()+len(data.encode())>MAX_BYTES:raise ValueError('observation_bound')
    stream.write(data);stream.flush()

def observe(command,output):
    owner=os.getpid();pgid=os.getpgrp();session=os.getsid(0)
    if owner!=pgid or owner!=session:raise ValueError('require_owned_session_leader')
    proc=None;started=time.monotonic();samples=0;next_sample=0.0;previous=None
    with output.open('x') as stream:
        try:
            # Inherit the existing Runner's session/process group, stdio and env.
            proc=subprocess.Popen(command,start_new_session=False)
            try:first=read_stat(proc.pid)
            except FileNotFoundError:
                # ENOENT is benign only when our own Popen confirms/reaps exit.
                if proc.poll() is None:raise
                emit(stream,{'kind':'early_end','pid':proc.pid,'owner_pid':owner,
                  'pgid':pgid,'session':session,'elapsed_seconds':round(time.monotonic()-started,3),
                  'exit_code':proc.returncode,'samples':0})
                return proc.returncode
            identity(first,owner,pgid,session);birth=first['start_ticks']
            emit(stream,{'kind':'start','pid':proc.pid,'owner_pid':owner,'pgid':pgid,
              'session':session,'start_ticks':birth,'ticks_per_second':os.sysconf('SC_CLK_TCK')})
            while proc.poll() is None:
                elapsed=time.monotonic()-started
                if elapsed>=next_sample:
                    if samples>=MAX_SAMPLES:raise ValueError('sample_bound')
                    try:value=read_stat(proc.pid)
                    except FileNotFoundError:
                        if proc.poll() is not None:break
                        raise
                    identity(value,owner,pgid,session,birth)
                    pair=(value['user_ticks'],value['system_ticks'])
                    if previous is not None and any(x<y for x,y in zip(pair,previous)):raise ValueError('cpu_ticks_regressed')
                    emit(stream,{'kind':'sample','elapsed_seconds':round(elapsed,3),
                      'user_ticks':pair[0],'system_ticks':pair[1]})
                    previous=pair;samples+=1;next_sample=elapsed+INTERVAL
                time.sleep(0.2)
            emit(stream,{'kind':'end','elapsed_seconds':round(time.monotonic()-started,3),
              'exit_code':proc.returncode,'samples':samples})
            return proc.returncode
        except BaseException as exc:
            # No exception text, argv or paths are collected as process metrics.
            emit(stream,{'kind':'error','elapsed_seconds':round(time.monotonic()-started,3),
              'reason':type(exc).__name__})
            raise
        finally:
            if proc is not None and proc.poll() is None:
                proc.terminate()
                try:proc.wait(timeout=0.2)
                except subprocess.TimeoutExpired:proc.kill();proc.wait(timeout=0.5)
            # The outer unchanged Runner always terminates/verifies its whole
            # process group, including any CLI children, even if this exits early.

def validate_paths(argv):
    if len(argv)!=5:raise ValueError('expected_five_paths')
    cli,official,mods,plan,output=map(Path,argv)
    if any(not p.is_absolute() for p in (cli,official,mods,plan,output)):raise ValueError('absolute_paths_required')
    work=cli.parents[2]
    if cli!=work/'cli-target/release/horizon-mixins' or official!=work/'horizon' or mods!=work/'language-root-probe' or plan!=work/'plan.json' or output!=work/'plan-cpu.jsonl':raise ValueError('unexpected_owned_path')
    return [str(cli),'plan','--workspace',str(official),'--mods',str(mods),'--out',str(plan)],output

def collect(output):
    result={'scope':'Samples of the direct owned CLI PID only; no child aggregation, CPU percentage, completion or planner target inference.',
      'present':False,'complete':False,'records':[]}
    try:info=output.lstat()
    except FileNotFoundError:return result
    if not stat.S_ISREG(info.st_mode) or info.st_size>MAX_BYTES:raise ValueError('unsafe_observation_file')
    raw=output.read_bytes()
    if len(raw)!=info.st_size or len(raw)>MAX_BYTES or raw and not raw.endswith(b'\n'):raise ValueError('partial_observation_file')
    lines=raw.decode().splitlines()
    if len(lines)>MAX_SAMPLES+2:raise ValueError('observation_record_bound')
    rows=[json.loads(line) for line in lines];result.update(present=True,records=rows,raw_bytes=len(raw))
    start=None;ended=False;count=0;last=(-1.0,-1,-1)
    result['cli_exit_code']=None
    for i,row in enumerate(rows):
        if not isinstance(row,dict):raise ValueError('observation_shape')
        kind=row.get('kind')
        if kind=='start':
            if i!=0 or set(row)!={'kind','pid','owner_pid','pgid','session','start_ticks','ticks_per_second'}:raise ValueError('start_shape')
            if any(type(v) is not int or v<=0 for k,v in row.items() if k!='kind'):raise ValueError('start_numbers')
            if row['owner_pid']!=row['pgid'] or row['owner_pid']!=row['session'] or row['pid']==row['owner_pid']:raise ValueError('start_identity')
            start=row
        elif kind=='early_end':
            if i!=0 or len(rows)!=1 or set(row)!={'kind','pid','owner_pid','pgid','session','elapsed_seconds','exit_code','samples'}:raise ValueError('early_end_shape')
            if any(type(row[k]) is not int or row[k]<=0 for k in ('pid','owner_pid','pgid','session')):raise ValueError('early_end_identity')
            if row['owner_pid']!=row['pgid'] or row['owner_pid']!=row['session'] or row['pid']==row['owner_pid']:raise ValueError('early_end_group')
            if type(row['exit_code']) is not int or type(row['samples']) is not int or row['samples']!=0:raise ValueError('early_end_numbers')
            if type(row['elapsed_seconds']) not in (int,float) or not math.isfinite(row['elapsed_seconds']) or row['elapsed_seconds']<0:raise ValueError('early_end_elapsed')
            ended=True;result.update(complete=True,cli_exit_code=row['exit_code'],cpu_sample_absence='CLI exit confirmed/reaped before first available stat')
        elif kind in ('sample','end','error'):
            elapsed=row.get('elapsed_seconds')
            if type(elapsed) not in (int,float) or not math.isfinite(elapsed) or elapsed<max(0,last[0]) or ended:raise ValueError('elapsed_or_order')
            if kind=='sample':
                if start is None or set(row)!={'kind','elapsed_seconds','user_ticks','system_ticks'}:raise ValueError('sample_shape')
                values=(row['user_ticks'],row['system_ticks'])
                if any(type(v) is not int or v<0 for v in values) or any(v<old for v,old in zip(values,last[1:])):raise ValueError('sample_ticks')
                if count and elapsed-last[0]<4.999:raise ValueError('sample_rate')
                count+=1
                if count>MAX_SAMPLES:raise ValueError('sample_bound')
                last=(elapsed,*values)
            elif kind=='end':
                if start is None or set(row)!={'kind','elapsed_seconds','exit_code','samples'} or type(row['exit_code']) is not int or type(row['samples']) is not int or row['samples']!=count:raise ValueError('end_shape')
                ended=True;result.update(complete=True,cli_exit_code=row['exit_code'])
            else:
                if set(row)!={'kind','elapsed_seconds','reason'} or row['reason'] not in ('ValueError','FileNotFoundError','PermissionError','ProcessLookupError','OSError','KeyboardInterrupt','TimeoutExpired','ObservationInterrupted'):raise ValueError('error_shape')
                ended=True
        else:raise ValueError('unknown_observation')
    return result

def main():
    if os.environ.get('GITHUB_ACTIONS')!='true' or os.environ.get('GITHUB_REPOSITORY')!='sofia-gros/horizon-mods' or os.environ.get('GITHUB_REF')!=BRANCH:raise ValueError('remote_only')
    signal.signal(signal.SIGTERM,interrupted)
    command,output=validate_paths(sys.argv[1:]);return observe(command,output)

if __name__=='__main__':
    try:code=main()
    except ObservationInterrupted:code=143
    except BaseException as exc:
        print('Owned plan observer failed: '+type(exc).__name__,file=sys.stderr);code=98
    raise SystemExit(code)
