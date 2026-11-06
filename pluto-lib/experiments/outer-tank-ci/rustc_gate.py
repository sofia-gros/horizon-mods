#!/usr/bin/env python3
"""Allow cold compilation, then deny all compilation before sealed execution."""
import json
import os
from pathlib import Path
import sys

def values(args, flag):
    out=[]
    for i,arg in enumerate(args):
        if arg==flag:out.append(args[i+1] if i+1<len(args) else None)
        elif arg.startswith(flag+'='):out.append(arg[len(flag)+1:])
    return out

def query(args):
    if args in (['-vV'],['--version']):return True
    prints=values(args,'--print')
    if not prints or any(p not in {'file-names','sysroot','split-debuginfo','crate-name','cfg','target-libdir','target-list'} for p in prints):return False
    bare=[];skip=False
    for arg in args:
        if skip:skip=False;continue
        if arg in {'--crate-name','--crate-type','--print','--target','-W'}:skip=True
        elif arg=='-' or not arg.startswith('-'):bare.append(arg)
        elif not (arg.startswith(('--print=','--crate-type=','--crate-name=','--target=')) or arg=='-Wwarnings'):return False
    return not skip and bare in ([],['-']) and values(args,'--crate-name') in ([],['___'])

def decision(phase,args):
    if phase not in {'compile','sealed'}:return 'denied'
    if query(args):return 'query'
    return 'compile' if phase=='compile' else 'denied'

def main():
    if len(sys.argv)<2:return 90
    compiler,args=sys.argv[1],sys.argv[2:]
    phase=Path(os.environ['OUTER_TANK_COMPILER_PHASE']).read_text()
    result=decision(phase,args)
    with Path(os.environ['OUTER_TANK_COMPILER_LOG']).open('a') as log:
        log.write(json.dumps({'compiler':compiler,'args':args,'cwd':os.getcwd(),'phase':phase,'decision':result})+'\n')
    if result=='denied':
        print('Denied compiler invocation before execution',file=sys.stderr)
        return 91
    os.execv(compiler,[compiler,*args])

if __name__=='__main__':raise SystemExit(main())
