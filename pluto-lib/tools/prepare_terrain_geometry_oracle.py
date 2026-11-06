#!/usr/bin/env python3
"""Extract exact official position/UV expressions for a std-only differential test.

This is not execution of the native fluid tessellator: family/visibility/light/
flow/pass decisions are excluded. Only reviewed geometry expression spans are
retained verbatim, with a small explicit argument wrapper around each span.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import tomllib
ROOT=Path(__file__).resolve().parents[1]
NATIVE=ROOT/'mixins/pluto_native_creative'
SOURCE='crates/horizon-client-render/src/mesher/fluid.rs'
def digest(raw):return hashlib.sha256(raw).hexdigest()
def unique(source,needle):
    if source.count(needle)!=1:raise ValueError('Official oracle anchor ambiguous/missing: '+needle)
    return source.index(needle)
def through(source,start,end):
    offset=unique(source,start);stop=source.index(end,offset)+len(end)
    return source[offset:stop]
def array_after(source,anchor):
    offset=unique(source,anchor);start=source.index('[',source.index('quad(',offset));depth=0
    for i in range(start,len(source)):
        if source[i]=='[':depth+=1
        elif source[i]==']':
            depth-=1
            if depth==0:return source[start:i+1]
    raise ValueError('Unclosed reviewed array')
def extract(source):
    spans={
        'origin':through(source,'    let (fx, fy, fz) =',';'),
        'top':through(source,'        let mut corners = [','        ];'),
        'bottom_y':through(source,'        let y0 = fy + bottom;',';'),
        'bottom':array_after(source,'        let y0 = fy + bottom;'),
        'side_match':through(source,'        let (h0, h1, [x0, z0], [x1, z1]) = match d {','        };'),
        'side_uv':through(source,'        let (v0, v1) =',';'),
        'side':array_after(source,'        let (v0, v1) ='),
        'back':'[v[0], v[3], v[2], v[1]]',
        'rotate':'corners.rotate_left(1);',
    }
    unique(source,spans['back']);unique(source,spans['rotate'])
    return spans

def prepare(host,output):
    meta=json.loads((ROOT/'tools/terrain-geometry-source.json').read_text())
    raw=(host/SOURCE).read_bytes()
    if digest(raw)!=meta['official_source_sha256']:raise ValueError('Official geometry source drift')
    source=raw.decode();spans=extract(source)
    if {k:digest(v.encode()) for k,v in spans.items()}!=meta['span_sha256']:raise ValueError('Reviewed geometry spans changed')
    if output.exists():raise ValueError('Output must not exist')
    output.mkdir(parents=True)
    header='// Extracted from official Horizon (MIT), copyright (c) 2026 Ludovic Marie and the Horizon contributors.\n// Full original license: HORIZON-LICENSE. No native meshing policy is executed here.\n#![allow(non_snake_case)]\nuse crate::geometry::Face as Direction;\npub type Quad=[([f32;3],[f32;2]);4];\n'
    origin='let [x,y,z]=origin.map(i32::from);\n'+spans['origin']+'\n'
    heights='let [nw,ne,sw,se]=heights;\n'
    oracle=header+'pub fn top(origin:[u8;3],heights:[f32;4],uv:[[f32;2];4],rotate:bool)->Quad {\n'+origin+heights+spans['top']+'\nif rotate {'+spans['rotate']+'}\ncorners\n}\n'
    oracle+='pub fn down(origin:[u8;3],bottom:f32)->Quad {\n'+origin+spans['bottom_y']+'\n'+spans['bottom']+'\n}\n'
    oracle+='pub fn side(origin:[u8;3],heights:[f32;4],bottom:f32,EPSILON:f32,d:Direction)->Quad {\n'+origin+heights+spans['side_match']+'\n'+spans['side_uv']+'\n'+spans['side']+'\n}\n'
    oracle+='pub fn back(v:Quad)->Quad {'+spans['back']+'}\n'
    (output/'oracle.rs').write_text(oracle)
    shutil.copyfile(NATIVE/'terrain-cell-geometry.rs',output/'geometry.rs')
    shutil.copyfile(ROOT/'tests/native/terrain-geometry/tests.rs',output/'tests.rs')
    shutil.copyfile(ROOT/'tests/native/terrain-geometry/HORIZON-LICENSE',output/'HORIZON-LICENSE')
    (output/'lib.rs').write_text('#![forbid(unsafe_code)]\n#[path="geometry.rs"] mod geometry;\n#[path="oracle.rs"] mod oracle;\n#[cfg(test)] mod tests;\n')
    evidence={'horizon_revision':meta['horizon_revision'],'official_source':SOURCE,'official_source_sha256':digest(raw),'span_sha256':meta['span_sha256'],'fixture_sha256':{p.name:digest(p.read_bytes()) for p in output.iterdir() if p.is_file()},'scope':'Exact official coordinate/UV/winding expressions only, wrapped in explicit test arguments. No native tessellation, loader, shading/culling, flow, draw pass or GPU execution.'}
    (output/'sources.json').write_text(json.dumps(evidence,indent=2)+'\n');print(output)
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--horizon-source',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();prepare(a.horizon_source,a.output)
