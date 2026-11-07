"""Fixed standard-tool preparation. Called only inside the driver's owned Job."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import sys
import time
import tomllib
import urllib.request
import urllib.parse
import zipfile

HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'tconstruct/tools'))
import play_windows_latest as launch
import playtest_guard_rustfmt as fmt

TOOLCHAIN='1.99.1-x86_64-pc-windows-gnu'
COMPONENTS={'rustc':'43b17e9a9134ea72562bfbb23728a830f671ea9be1db6a72597a8d240e986b35',
 'cargo':'57e54617be4420dbb967ca8293f5ace7168104ac12efa9dc7c5c3bda112f8b36',
 'rust-std':'6546651238a3ac092b763511be524ca6eda6103e5d47342416923be1b3ce837c',
 'rust-mingw':'30263d7bb4fd0bf63a153afe36d7fc102f929a7d42781651c6b3a03a414ba428',
 'rustfmt-preview':fmt.ARCHIVE_SHA256}
WINLIBS_URL='https://github.com/brechtsanders/winlibs_mingw/releases/download/16.2.0posix-23.1.3-14.0.0-msvcrt-r3/winlibs-x86_64-posix-seh-gcc-16.2.0-mingw-w64msvcrt-14.0.0-r3.zip'
WINLIBS_SHA='62af32ea58acdea1318ea5dd8eb7017eaee694dd40b269d52a59c769dcfefb82'
MAX_ZIP=2*1024**3;MAX_EXPANDED=8*1024**3;MAX_MEMBERS=100000;MAX_FILE=1024**3


def record(path,limit=2*1024**3):
    p=Path(path);info=p.lstat();launch.plain(info)
    if not stat.S_ISREG(info.st_mode) or info.st_size>limit:raise ValueError('File type/size budget')
    digest,size=launch.file_hash(p,info)
    return {'sha256':digest,'bytes':size}


def environment(original,work):
    env=launch.build_environment(original,work/'rustup/toolchains'/TOOLCHAIN,work/'home')
    # These affect standard rustup only; absolute selected tools are used later.
    for key in list(env):
        if key.upper().startswith('RUSTUP_'):del env[key]
    env.update(RUSTUP_HOME=str(work/'rustup'),RUSTUP_DIST_SERVER='https://static.rust-lang.org',
      RUSTUP_UPDATE_ROOT='https://static.rust-lang.org/rustup',RUSTUP_AUTO_INSTALL='0',
      RUSTUP_TERM_PROGRESS_WHEN='never',RUSTUP_TERM_COLOR='never',CARGO_TERM_COLOR='never')
    return env


def install_argv(rustup):
    return [str(rustup),'toolchain','install',TOOLCHAIN,'--profile','minimal',
            '--component','rustfmt','--no-self-update']


def admit_manifest(toolchain,installed):
    path=toolchain/'lib/rustlib/multirust-channel-manifest.toml'
    raw=fmt.bounded_file(path,4*1024**2);manifest=tomllib.loads(raw.decode('utf-8'))
    if manifest.get('date')!='2026-10-15' or manifest.get('manifest-version')!='2':raise ValueError('Official distribution manifest differs')
    for name,digest in COMPONENTS.items():
        target=manifest['pkg'][name]['target'][launch.TARGET]
        distname='rustfmt' if name=='rustfmt-preview' else name
        url=f'https://static.rust-lang.org/dist/2026-10-15/{distname}-1.99.1-{launch.TARGET}.tar.gz'
        if target.get('available') is not True or target.get('hash')!=digest or target.get('url')!=url:
            raise ValueError('Official component pin differs: '+name)
    want=sorted(('rustfmt' if name=='rustfmt-preview' else name)+'-'+launch.TARGET for name in COMPONENTS)
    if sorted(installed.splitlines())!=want:raise ValueError('Installed component set differs')
    return {'manifest':record(path,4*1024**2),'components':want,'component_gzip_sha256':COMPONENTS}


def download(url,path,digest,limit):
    if (url,digest,limit) not in [(fmt.URL,fmt.ARCHIVE_SHA256,fmt.ARCHIVE_LIMIT),(WINLIBS_URL,WINLIBS_SHA,MAX_ZIP)]:
        raise ValueError('Unknown download')
    class Redirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self,req,fp,code,msg,headers,newurl):
            parsed=urllib.parse.urlparse(newurl)
            if url!=WINLIBS_URL or parsed.scheme!='https' or parsed.hostname!='release-assets.githubusercontent.com':
                raise ValueError('Unexpected download redirect')
            return super().redirect_request(req,fp,code,msg,headers,newurl)
    start=time.monotonic();h=hashlib.sha256();size=0
    with urllib.request.build_opener(Redirect()).open(url,timeout=30) as response,Path(path).open('xb') as out:
        if response.status!=200:raise ValueError('Download HTTP failure')
        declared=response.headers.get('Content-Length')
        if declared is not None and not 0<int(declared)<=limit:raise ValueError('Download size budget')
        while data:=response.read(min(65536,limit-size+1)):
            size+=len(data)
            if size>limit or time.monotonic()-start>1100:raise ValueError('Download budget')
            out.write(data);h.update(data)
        out.flush();os.fsync(out.fileno())
    if not size or h.hexdigest()!=digest:raise ValueError('Official download digest differs')
    value=record(path,limit)
    if value!={'sha256':digest,'bytes':size}:raise ValueError('Download readback drift')
    return value


def unpack_winlibs(path,toolchain):
    archive=record(path,MAX_ZIP)
    if archive['sha256']!=WINLIBS_SHA:raise ValueError('WinLibs digest differs')
    destination=toolchain/'mingw64'
    if destination.exists() or destination.is_symlink():raise ValueError('WinLibs destination not fresh')
    total=0;seen=set();files=0
    with zipfile.ZipFile(path) as z:
        entries=z.infolist()
        if not entries or len(entries)>MAX_MEMBERS:raise ValueError('WinLibs entry budget')
        for e in entries:
            parts=launch.bundles.safe_name(e.filename.rstrip('/'));key='/'.join(parts).casefold()
            if parts[0]!='mingw64' or key in seen or e.flag_bits&1:raise ValueError('WinLibs path/duplicate/encryption')
            seen.add(key);mode=e.external_attr>>16
            if stat.S_IFMT(mode) not in (0,stat.S_IFREG,stat.S_IFDIR):raise ValueError('WinLibs special entry')
            if not 0<=e.file_size<=MAX_FILE:raise ValueError('WinLibs member size')
            total+=e.file_size
            if total>MAX_EXPANDED:raise ValueError('WinLibs expansion budget')
        for e in entries:
            p=toolchain.joinpath(*launch.bundles.safe_name(e.filename.rstrip('/')))
            if e.is_dir():p.mkdir(parents=True,exist_ok=True);continue
            p.parent.mkdir(parents=True,exist_ok=True);launch.directory(p.parent);count=0
            with z.open(e) as src,p.open('xb') as dst:
                while data:=src.read(min(65536,e.file_size-count+1)):
                    count+=len(data)
                    if count>e.file_size:raise ValueError('WinLibs member grew')
                    dst.write(data)
            if count!=e.file_size:raise ValueError('WinLibs member truncated')
            files+=1
    return {'archive':archive,'files':files,'expanded_bytes':total,'tree_sha256':launch.tree_digest(destination)}


def main():
    if os.name!='nt':raise ValueError('Prepared Windows phase only')
    work=Path(sys.argv[2]);toolchain=work/'rustup/toolchains'/TOOLCHAIN
    if sys.argv[1]=='archives':
        result={'formatter':download(fmt.URL,work/'rustfmt.tar.gz',fmt.ARCHIVE_SHA256,fmt.ARCHIVE_LIMIT),
                'winlibs':download(WINLIBS_URL,work/'winlibs.zip',WINLIBS_SHA,MAX_ZIP)}
    elif sys.argv[1]=='winlibs':result=unpack_winlibs(work/'winlibs.zip',toolchain)
    else:raise ValueError('Unknown fixed preparation phase')
    print(json.dumps(result,sort_keys=True),flush=True)

if __name__=='__main__':main()
