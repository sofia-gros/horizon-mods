#!/usr/bin/env python3
"""Generate/verify UNAPPLIED addition diffs only; no staging, apply or compilation."""
import argparse
import difflib
import hashlib
import importlib.util
import json
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('contact_sources',HERE/'contact_sources.py')
base=importlib.util.module_from_spec(spec);spec.loader.exec_module(base)
SOURCE='crates/horizon-client/src/game.rs'
ADDITIONAL={
 SOURCE:'cdfc9247cce139c00e5a36266812c7f7f42ccbb565d67705a896520b4b62b22c',
 'crates/horizon-client/src/lib.rs':'3899ca984b7853a9c3f5e852f4eb24670be60dd2a24ab357c4ce46c6444fafcc',
 'crates/horizon-client/src/mod_api.rs':'d362b451014b1ed650a8c3fd0fc9ea2fcc1477686b88ee7b5d5cf040e22db643',
 'crates/horizon-client/src/game/commands.rs':'415bda393e631faa8e125bd115b63ad911467efce723bae5e76b71b4461902bd',
 'crates/horizon-client/src/game/native_entity_refs.rs':'9d29cf0555db6cd720455ad55d5a572f140730c83c690d464875a4af60d80c1f',
 'crates/horizon-client/src/game/lan.rs':'09ee322c6a85a8c3fea44f39757801f1f49490b816367f2d8e0b5f9c3c77c063',
}
def digest(b):return hashlib.sha256(b).hexdigest()
def read(official):
    sources=base.read(official)
    for name,expected in ADDITIONAL.items():
        raw=(official/name).read_bytes()
        if digest(raw)!=expected:raise ValueError('official source changed: '+name)
        sources[name]=raw.decode()
    return sources

def proposed(original,kind):
    if kind not in ('positive','negative'):raise ValueError('unknown probe')
    return original+'\n'+(HERE/f'game_borrow_{kind}.rs').read_text()

def patch(original,kind):
    return ''.join(difflib.unified_diff(original.splitlines(True),proposed(original,kind).splitlines(True),
        fromfile='a/'+SOURCE,tofile='b/'+SOURCE))

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--official',type=Path,required=True)
    parser.add_argument('--write-patches',action='store_true')
    args=parser.parse_args();sources=read(args.official);original=sources[SOURCE]
    for kind in ('positive','negative'):
        target=HERE/f'UNAPPLIED-game-borrow-{kind}.patch';delta=patch(original,kind)
        if args.write_patches:target.write_text(delta)
        elif target.read_text()!=delta:raise ValueError('probe differs: '+kind)
    if read(args.official)!=sources:raise ValueError('official input changed')
    print('Exact addition diffs verified. No overlay, application or native compilation performed.')
if __name__=='__main__':main()
