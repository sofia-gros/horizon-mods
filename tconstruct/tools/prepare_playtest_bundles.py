#!/usr/bin/env python3
"""Bounded exact package receipts for the same-checkout Windows playtest.

No source fetching, revision edits, native planner, compilation or guard bypass.
Both existing packagers run verbatim; each package retains its separate limits.
"""
import hashlib
import io
import json
from pathlib import Path
import stat
import tempfile
import tomllib
import zipfile

from playtest_pluto_inputs import directory, plain

HORIZON='41f822b659789780f58d32c63ddfb6887ea1b8bf'
MIXINS='5a64af938ec86b00d53d16a408aabc0038fb3e57'
MAX_ENTRIES=512
MAX_UNPACKED=64*1024**2
MAX_ARCHIVE=64*1024**2
MAX_SUPPORT_ENTRIES=10000


def sha(raw): return hashlib.sha256(raw).hexdigest()
def map_seal(value): return sha(json.dumps(value,sort_keys=True).encode())
def unique_object(pairs):
    out={}
    for key,value in pairs:
        if key in out: raise ValueError('Duplicate package header key')
        out[key]=value
    return out


def safe_name(name):
    parts=name.split('/')
    if not name or any(ord(c)<32 or c in '<>\"|?*' for c in name) or '\\' in name or any(not x or x in ('.','..') or ':' in x or x[-1:] in (' ','.') for x in parts):
        raise ValueError('Unsafe package member path')
    reserved={'CON','PRN','AUX','NUL',*[f'COM{i}' for i in range(1,10)],*[f'LPT{i}' for i in range(1,10)]}
    if any(x.split('.')[0].upper() in reserved for x in parts):raise ValueError('Windows reserved member path')
    return parts


def members(raw,limit):
    if len(raw)>MAX_ARCHIVE:raise ValueError('Archive byte budget exceeded')
    result={};folded=set();total=0
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        entries=archive.infolist()
        if not entries or len(entries)>limit:raise ValueError('Archive member budget exceeded')
        for entry in entries:
            safe_name(entry.filename)
            if entry.is_dir() or entry.external_attr>>16 != stat.S_IFREG|0o644 or entry.flag_bits&1:
                raise ValueError('Package member must be plain unencrypted mode0644 file')
            if entry.filename.casefold() in folded:raise ValueError('Duplicate/case-colliding package member')
            folded.add(entry.filename.casefold());total+=entry.file_size
            if entry.file_size<0 or total>MAX_UNPACKED:raise ValueError('Unpacked package budget exceeded')
            value=archive.read(entry)
            if len(value)!=entry.file_size:raise ValueError('Package member length mismatch')
            result[entry.filename]=value
    return result,total


def inspect(raw,identity):
    files,total=members(raw,MAX_ENTRIES)
    header=json.loads(files.get('horizon-native.json',b''),object_pairs_hook=unique_object)
    expected={'schema':1,'id':identity,'game':'26.3','horizon':HORIZON,'mixins':MIXINS}
    if any(header.get(key)!=value for key,value in expected.items()):raise ValueError('Native package identity/revisions differ; fix source explicitly')
    prefix='mixins/'+identity+'/'
    if prefix+'horizon-mixin.toml' not in files:raise ValueError('Missing unified native manifest')
    inner=tomllib.loads(files[prefix+'horizon-mixin.toml'].decode())
    if inner.get('mod',{}).get('id')!=identity or inner.get('mod',{}).get('version')!=header.get('version') or inner.get('compatibility',{}).get('horizon_revision')!=HORIZON:
        raise ValueError('Header and native manifest disagree')
    if any(name!='horizon-native.json' and not name.startswith(prefix) for name in files):raise ValueError('Unexpected package root')
    support=None
    if identity=='tconstruct_native':
        name=prefix+'data/tconstruct-support.zip'
        if name not in files:raise ValueError('Missing TConstruct support archive')
        nested,size=members(files[name],MAX_SUPPORT_ENTRIES)
        allowed=('tconstruct/content/','tconstruct/assets/','tconstruct/third_party/','tconstruct/docs/','tconstruct/tools/','pluto-lib/pluto_lib/')
        if any(not n.startswith(allowed) for n in nested):raise ValueError('Unexpected nested support path')
        support={'members':len(nested),'unpacked_bytes':size,'member_sha256':{n:sha(b) for n,b in sorted(nested.items())}}
    receipt={'id':identity,'archive_sha256':sha(raw),'archive_bytes':len(raw),'members':len(files),'unpacked_bytes':total,
             'member_sha256':{n:sha(b) for n,b in sorted(files.items())},'header':header}
    if support is not None:receipt['support']=support
    return files,receipt


def combine(packages):
    result={};folded=set()
    for files in packages:
        for name,raw in files.items():
            if name=='horizon-native.json':continue
            if not name.startswith('mixins/'):raise ValueError('Unexpected combined package root')
            relative=name[len('mixins/'):]
            if relative.casefold() in folded:raise ValueError('Combined native member collision')
            folded.add(relative.casefold());result[relative]=raw
    values={n:sha(b) for n,b in sorted(result.items())}
    return result,{'members':len(values),'member_sha256':values,'members_seal_sha256':map_seal(values)}


def disk_members(root):
    root=directory(root);result={};total=0
    def walk(current):
        nonlocal total
        if len(current.relative_to(root).parts)>32:raise ValueError('Retained package directory depth exceeded')
        for p in sorted(current.iterdir(),key=lambda x:x.name):
            info=p.lstat();plain(info)
            if stat.S_ISDIR(info.st_mode):walk(p)
            else:
                total+=info.st_size
                if len(result)>=2*MAX_ENTRIES+2 or info.st_size>MAX_ARCHIVE or total>2*(MAX_ARCHIVE+MAX_UNPACKED):raise ValueError('Retained package read budget exceeded')
                raw=p.read_bytes()
                if len(raw)!=info.st_size:raise ValueError('Retained package changed during read')
                result[p.relative_to(root).as_posix()]=raw
    walk(root)
    return result


def verify_prepared(tc_mods,pluto_root,receipt):
    values={}
    for root in (tc_mods,pluto_root.parent):
        for name,raw in disk_members(root).items():
            if name in values:raise ValueError('Prepared bundle collision')
            values[name]=sha(raw)
    expected=receipt['combined']
    if values!=expected['member_sha256'] or map_seal(values)!=expected['members_seal_sha256'] or len(values)!=expected['members']:
        raise ValueError('Prepared native members differ from same-checkout package seal')
    for name,record in zip(('tconstruct.hnmod','plutolib.hnmod'),receipt['packages'],strict=True):
        path=tc_mods.parent/name;plain(path.lstat())
        if path.stat().st_size!=record['archive_bytes'] or sha(path.read_bytes())!=record['archive_sha256']:
            raise ValueError('Prepared native archive changed')
    return receipt


def prepare(repo,home,run,python,cwd,env,external_pluto=None):
    """Run same-source packagers, validate, and retain immutable per-content output.

    The caller seals all Mod/package sources before/after; run supplies the
    caller's isolated reviewed environment. Only this fresh temporary package
    attempt is automatically cleaned; failed builds and user files are not.
    """
    root=home/'packages-latest';root.mkdir(parents=True,exist_ok=True);directory(root)
    with tempfile.TemporaryDirectory(prefix='prepare-',dir=root) as temporary:
        temp=Path(temporary);tc=temp/'tconstruct.hnmod';pl=temp/'plutolib.hnmod'
        run([python,repo/'tconstruct/tools/package_hnmod.py','--output',tc],cwd=cwd,env=env)
        run([python,repo/'pluto-lib/tools/package_native.py',pl],cwd=cwd,env=env)
        for path in (tc,pl):
            plain(path.lstat())
            if path.stat().st_size>MAX_ARCHIVE:raise ValueError('Generated package exceeds archive read budget')
        tc_raw=tc.read_bytes();pl_raw=pl.read_bytes()
        tc_files,tc_receipt=inspect(tc_raw,'tconstruct_native');pl_files,pl_receipt=inspect(pl_raw,'pluto_native_creative')
        merged,combined=combine([tc_files,pl_files])
        pluto_files={n[len('mixins/pluto_native_creative/'):]:b for n,b in pl_files.items() if n!='horizon-native.json'}
        if external_pluto is not None and disk_members(external_pluto)!=pluto_files:
            raise ValueError('External Pluto bundle differs from the complete same-checkout package')
        receipt={'schema':1,'packages':[tc_receipt,pl_receipt],'combined':combined}
        key=map_seal(receipt);dest=root/key
        # Keep the bundles separate; the combined 600+ files are not one hnmod.
        expected={'tconstruct.hnmod':tc_raw,'plutolib.hnmod':pl_raw}
        for name,raw in merged.items():
            group='tc-mods/' if name.startswith('tconstruct_native/') else 'pluto-mods/'
            expected[group+name]=raw
        if dest.exists():
            if disk_members(dest)!=expected:raise ValueError('Retained generated package bytes changed')
        else:
            dest.mkdir()
            for name,raw in sorted(expected.items()):
                out=dest/name;out.parent.mkdir(parents=True,exist_ok=True)
                with out.open('xb') as stream:stream.write(raw)
        return dest/'tc-mods',dest/'pluto-mods/pluto_native_creative',receipt
