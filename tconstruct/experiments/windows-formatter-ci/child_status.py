"""Existing barrier worker with a bounded, separate child-returncode witness.

No extra process, retry, status-only formatter call, or component acquisition.
The imported phase still owns assignment, limits, teardown and stdout unchanged.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys

LIMIT = 1024
STATUS_ENV = 'PLUTO_PHASE_CHILD_STATUS'


def argv_hash(argv):
    if not isinstance(argv, list) or not argv or any(not isinstance(x,str) for x in argv):
        raise ValueError('Expected one explicit string argv')
    return hashlib.sha256(json.dumps(argv,ensure_ascii=True,separators=(',',':')).encode('utf-8')).hexdigest()


def status_path(value):
    path=Path(value)
    if not path.is_absolute() or path.parent!=Path.cwd() or not re.fullmatch('[a-z][a-z0-9-]{0,63}\\.child-status\\.json',path.name):
        raise ValueError('Child status must be one fixed file in its phase work directory')
    if path.exists() or path.is_symlink():raise ValueError('Child status destination is not fresh')
    return path


def write_status(path,value):
    raw=(json.dumps(value,sort_keys=True,separators=(',',':'))+'\n').encode('utf-8')
    if len(raw)>LIMIT:raise ValueError('Child status byte limit')
    with path.open('xb') as stream:stream.write(raw);stream.flush();os.fsync(stream.fileno())


def worker():
    if sys.stdin.buffer.read(1)!=b'G':return 97
    argv=sys.argv[2:];digest=argv_hash(argv)
    path=status_path(os.environ[STATUS_ENV])
    child_env={k:v for k,v in os.environ.items() if k.upper()!=STATUS_ENV}
    try:
        result=subprocess.run(argv,env=child_env,check=False)
    except OSError as error:
        write_status(path,{'schema':1,'argv_sha256':digest,'kind':'spawn_error',
                           'errno':error.errno,'winerror':getattr(error,'winerror',None)})
        raise
    write_status(path,{'schema':1,'argv_sha256':digest,'kind':'exit','returncode':result.returncode})
    # Preserve the old wrapper-exit behavior. Receipt now also has the raw value
    # before CPython's sys.exit conversion, including unsigned Windows statuses.
    return result.returncode


def read_status(path,argv):
    before=path.lstat()
    if not stat.S_ISREG(before.st_mode) or getattr(before,'st_file_attributes',0)&0x400 or not 0<before.st_size<=LIMIT:
        raise ValueError('Child status must be a bounded regular file')
    def signature(s):return (s.st_dev,s.st_ino,s.st_mode,s.st_size,s.st_mtime_ns,s.st_nlink,getattr(s,'st_file_attributes',0),getattr(s,'st_reparse_tag',0))
    with path.open('rb') as stream:
        opened=os.fstat(stream.fileno())
        if signature(before)!=signature(opened):raise ValueError('Child status changed before read')
        raw=stream.read(LIMIT+1)
        after_handle=os.fstat(stream.fileno())
    after=path.lstat()
    # No execute-suffix mode issue: these files always end in .json.
    if len(raw)!=before.st_size or len(raw)>LIMIT or signature(opened)!=signature(after_handle) or signature(before)!=signature(after):raise ValueError('Child status changed during read')
    def pairs(items):
        result={}
        for k,v in items:
            if k in result:raise ValueError('Duplicate child status key')
            result[k]=v
        return result
    def reject(_):raise ValueError('Noninteger child status number')
    value=json.loads(raw.decode('utf-8'),object_pairs_hook=pairs,parse_float=reject,parse_constant=reject)
    if not isinstance(value,dict) or type(value.get('schema')) is not int or value['schema']!=1 or value.get('argv_sha256')!=argv_hash(argv):raise ValueError('Child status identity differs')
    def integer(v):return type(v) is int and -2**31<=v<2**32
    if value.get('kind')=='exit':
        if set(value)!={'schema','argv_sha256','kind','returncode'} or not integer(value['returncode']):raise ValueError('Invalid raw child returncode')
    elif value.get('kind')=='spawn_error':
        if set(value)!={'schema','argv_sha256','kind','errno','winerror'} or any(v is not None and not integer(v) for v in (value['errno'],value['winerror'])):raise ValueError('Invalid child spawn error')
    else:raise ValueError('Unknown child status kind')
    return {'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest(),'result':value}


def phase(base,name,argv,work,env,receipt):
    if re.fullmatch('[a-z][a-z0-9-]{0,63}',name) is None:raise ValueError('Unexpected phase name')
    argv_hash(argv);path=work/(name+'.child-status.json')
    if path.exists() or path.is_symlink():raise ValueError('Child status destination is not fresh')
    child_env={k:v for k,v in env.items() if k.upper()!=STATUS_ENV};child_env[STATUS_ENV]=str(path)
    old_worker=base.__file__;before=len(receipt['phases']);failed=None;raw=None
    try:
        # Replace the existing worker script, never nest a new wrapper process.
        base.__file__=__file__
        raw=base.phase(name,argv,work,child_env,receipt)
    except BaseException as error:failed=error
    finally:base.__file__=old_worker
    if len(receipt['phases'])!=before+1 or receipt['phases'][-1].get('name')!=name:
        raise RuntimeError('Missing single owned phase receipt') from failed
    item=receipt['phases'][-1]
    try:
        item['child_status']=read_status(path,argv)
    except Exception as error:
        item['child_status_error']=type(error).__name__+': '+str(error)
        item['process_succeeded']=False
        if failed is None:raise
    if failed is not None:raise failed
    status=item['child_status']['result']
    if status['kind']!='exit' or status['returncode']!=0 or item['exit']!=0 or item.get('job_empty') is not True:
        item['process_succeeded']=False
        raise RuntimeError('Raw child result does not confirm the successful phase')
    return raw


if __name__=='__main__':
    if sys.argv[1:2]!=['--worker']:raise SystemExit('Only the assigned phase worker is supported')
    sys.exit(worker())
