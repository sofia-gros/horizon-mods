"""Only bounded input preparation and original guard regeneration; no Cargo/Game."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import subprocess
import urllib.request

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'tconstruct/tools'))
import playtest_guard_rustfmt as fmt
HORIZON='41f822b659789780f58d32c63ddfb6887ea1b8bf'
BASE='https://raw.githubusercontent.com/horizon-rust/horizon/'+HORIZON+'/'
MAX_SOURCE=2*1024**2
PLAN_SHA='7139236d6484c9cf5a862d3e8358d60b2b4e0bd3a8c4c0968c6d313735670f8b'
PLAN_BYTES=6383021
COUNTS={'damage':3,'terrain-cell':4}
def sha(raw):return hashlib.sha256(raw).hexdigest()
def config():return json.loads((HERE/'inputs.json').read_bytes())
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path);mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod);return mod
def verify_plan(path):
    raw=fmt.bounded_file(path,16*1024**2)
    if len(raw)!=PLAN_BYTES or sha(raw)!=PLAN_SHA:raise ValueError('Historical full plan differs')
    return json.loads(raw)
def source_map():
    paths=['ordinary-equipment-source.json','explosion-equipment-source.json','armor-wear-source.json','terrain-cell-source.json']
    return {m['file']:m['file_sha256'] for m in (json.loads((ROOT/'pluto-lib/tools'/p).read_bytes()) for p in paths)}
def download(url,destination,digest,limit):
    # No user data/credentials, redirects, unknown origins or fallback sources.
    approved={fmt.URL,*[BASE+p for p in source_map()]}
    if url not in approved:raise ValueError('Unlisted official download')
    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self,*args,**kwargs):raise ValueError('Unexpected official redirect')
    opener=urllib.request.build_opener(NoRedirect())
    count=0;h=hashlib.sha256()
    with opener.open(url,timeout=15) as response,Path(destination).open('xb') as out:
        if response.status!=200 or response.geturl()!=url:raise ValueError('Unexpected download response')
        length=response.headers.get('Content-Length')
        if length is not None and not 0<int(length)<=limit:raise ValueError('Download size budget')
        while chunk:=response.read(min(1024*1024,limit-count+1)):
            count+=len(chunk)
            if count>limit:raise ValueError('Download size budget')
            h.update(chunk);out.write(chunk)
    if count==0 or h.hexdigest()!=digest:raise ValueError('Official download hash differs')
    return {'bytes':count,'sha256':h.hexdigest()}
def prepare(work):
    work=Path(work);official=work/'official';official.mkdir()
    archive=work/'rustfmt.tar.gz'
    record=download(fmt.URL,archive,fmt.ARCHIVE_SHA256,fmt.ARCHIVE_LIMIT)
    binary=fmt.archive_binary(fmt.bounded_file(archive,fmt.ARCHIVE_LIMIT))
    toolchain=work/'formatter';(toolchain/'bin').mkdir(parents=True)
    executable=toolchain/'bin/rustfmt.exe';executable.write_bytes(binary)
    admitted=fmt.admit(toolchain,archive)
    sources={}
    for path,digest in source_map().items():
        target=official/path;target.parent.mkdir(parents=True,exist_ok=True)
        sources[path]=download(BASE+path,target,digest,MAX_SOURCE)
    return {'kind':'preparation','formatter':admitted,'sources':sources,'archive':record}
def audit_call(executable,args,expected):
    wanted=[str(expected),'--edition','2024','--emit','stdout']
    # CPython's Windows audit event has a rendered CreateProcess command line,
    # and application_name=None when Popen chooses its first argv token.
    if executable not in (None,str(expected)) or not isinstance(args,str) or args!=subprocess.list2cmdline(wanted):raise ValueError('Unexpected child process in regeneration')
    return {'application_name':executable,'command_line':args,'argv':wanted}
def regenerate(kind,work,plan_path):
    if kind not in COUNTS:raise ValueError('Unknown regeneration')
    work=Path(work);official=work/'official';exe=work/'formatter/bin/rustfmt.exe'
    if not exe.is_absolute() or os.environ.get('PLUTO_GUARD_RUSTFMT')!=str(exe):raise ValueError('Formatter routing is not exact absolute path')
    fmt.require_guard_routing(ROOT/'pluto-lib');fmt.admit(work/'formatter',work/'rustfmt.tar.gz')
    for path,digest in source_map().items():
        if sha(fmt.bounded_file(official/path,MAX_SOURCE))!=digest:raise ValueError('Official source drift')
    plan=verify_plan(plan_path);calls=[]
    def audit(event,args):
        if event=='subprocess.Popen':
            if len(calls)>=COUNTS[kind]:raise ValueError('Too many formatter invocations')
            calls.append(audit_call(args[0],args[1],exe))
    sys.addaudithook(audit)
    if kind=='damage':
        guard=load('fixed_damage',ROOT/'pluto-lib/tools/check_damage_equipment_plan.py')
        guard.verify_source(plan,official,guard.entries())
    else:
        guard=load('fixed_terrain',ROOT/'pluto-lib/tools/check_terrain_cell_plan.py')
        guard.verify_regeneration(official,guard.metadata())
    if len(calls)!=COUNTS[kind]:raise ValueError('Missing actual formatter invocation')
    return {'kind':'regeneration','guard':kind,'passed':True,'calls':calls,
            'historical_plan_sha256':PLAN_SHA,'composition_verified':False,'apply_executed':False}
def main():
    if os.name!='nt':raise ValueError('Windows-only formatter execution')
    if sys.argv[1:2]==['prepare'] and len(sys.argv)==3:out=prepare(sys.argv[2])
    elif sys.argv[1:2]==['regenerate'] and len(sys.argv)==5:out=regenerate(*sys.argv[2:])
    else:raise ValueError('Invalid fixed phase arguments')
    print(json.dumps(out,sort_keys=True),flush=True)
if __name__=='__main__':main()
