#!/usr/bin/env python3
"""Excluded material-aware real-Horizon CPU overlay; source prepared, native unrun. No cargo/plan/apply is run here."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import tomllib
import zipfile
import zlib
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
REVISION = '624c4f6265c02296fbe8a80f029dd0d987fb94c0'
PRODUCTION_HEAD = '43f174273cd5f1b9cf3acd1d26abab8838025510'
BUNDLE_SHA256 = 'cd6de18cacbe7c40218ec817671bf3842aef3be0d586c613bb788babafbd3a76'
PREFIX = 'mixins/pluto_native_creative/'
MODULE = 'terrain-material-cpu-fixture.rs'
ENTRY = '''\n# Validation-only module, never ordinary packaging.\n[[module]]\nid = "terrain-material-cpu-fixture"\nfile = "crates/horizon-client-render/src/mesher.rs"\nname = "pluto_terrain_material_cpu"\nsource = "terrain-material-cpu-fixture.rs"\n'''
NAMES = ['minecraft:air','minecraft:bedrock','minecraft:dirt','minecraft:stone','minecraft:sand','minecraft:oak_planks','minecraft:bricks','pluto_fixture:cell']
def sha(data): return hashlib.sha256(data).hexdigest()
def encoded(value): return (json.dumps(value,sort_keys=True,separators=(',',':'))+'\n').encode()
def png(color,size=16,second=None):
    def chunk(kind,data): return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    height=size*(2 if second is not None else 1)
    raw=b''.join(b'\0'+bytes(color if y<size else second)*size for y in range(height))
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',size,height,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b'')
def assets(version,kind="ordinary"):
    if kind not in RESOURCE_KINDS: raise ValueError("unknown resource variant")
    ambiguous = kind == "ambiguous"
    """All geometry, textures and facts are original synthetic test data."""
    files={}
    files['horizon-resources.json']=encoded({'data_version':version})
    files['facts.json']=encoded({'states':[[name,i==0,False,name,name] for i,name in enumerate(NAMES)],'defaults':{name:i for i,name in enumerate(NAMES)}})
    files['block-tags.json']=encoded({name:json.dumps({'values':NAMES[1:7]}) for name in ['minecraft:blocks_motion_in_heightmap','minecraft:blocks_motion_in_heightmap_no_leaves','minecraft:blocks_fluid_flow']})
    files['model-states.json']=encoded({name:{'schema':{},'states':[{}]} for name in ['minecraft:stone','pluto_fixture:cell']})
    for ns,name,texture in [('minecraft','stone','minecraft:block/stone'),('pluto_fixture','cell','pluto_fixture:block/cell')]:
        files[f'assets/{ns}/blockstates/{name}.json']=encoded({'variants':{'':{'model':f'{ns}:block/{name}'}}})
        element={'from':[0,0,0],'to':[16,16,16],'faces':{d:{'texture':'#all','cullface':d} for d in ['down','up','north','south','west','east']}}
        files[f'assets/{ns}/models/block/{name}.json']=encoded({'ambientocclusion':True,'textures':{'all':texture,'particle':texture},'elements':[element]*(2 if ambiguous and name=='cell' else 1)})
        files[f'assets/{ns}/textures/block/{name}.png']=png([66,120,190,160] if name=='cell' else [130,130,130,255],second=[80,130,200,160] if name=='cell' else None)
        if name=='cell':files[f'assets/{ns}/textures/block/{name}.png.mcmeta']=encoded({'animation':{'frametime':2,'frames':[0,1],'interpolate':True}})
    # A distinct baked standalone model is a preparation input, not a texture
    # substituted into a live mesher. The invalid variant differs in native shade.
    model=json.loads(files['assets/pluto_fixture/models/block/cell.json'])
    model['textures']={'all':'pluto_fixture:block/flow','particle':'pluto_fixture:block/flow'}
    if kind=='incompatible':model['elements'][0]['shade']=False
    if kind=='opaque':
        for name in ['assets/pluto_fixture/models/block/cell.json']:
            src=json.loads(files[name])
            for element in src['elements']:
                for face in element['faces'].values():face['uv']=[4,4,12,12]
            files[name]=encoded(src)
        for element in model['elements']:
            for face in element['faces'].values():face['uv']=[4,4,12,12]
        files['assets/pluto_fixture/textures/block/cell.png']=png([66,120,190,255],second=[80,130,200,255])
    if kind=='fusion':
        model['loader']='fusion:model';model['type']='fusion:connecting'
    files['assets/pluto_fixture/models/block/flow.json']=encoded(model)
    alpha=255 if kind=='opaque' else 160
    files['assets/pluto_fixture/textures/block/flow.png']=png([190,80,40,alpha],second=[180,70,30,alpha])
    files['assets/pluto_fixture/textures/block/flow.png.mcmeta']=encoded({'animation':{'frametime':3,'frames':[0,1]}})
    if kind=='fusion':
        # One native full-layout sheet, forty-eight distinct original 16px tiles.
        # No animation claims in this case; load() must produce real rule IDs.
        files['assets/pluto_fixture/textures/block/flow.png']=sheet_png()
        files['assets/pluto_fixture/textures/block/flow.png.mcmeta']=encoded({'fusion':{'type':'connecting','layout':'full'}})
    # Real MeshTables requires these atlas keys even though no native fluid state
    # is present. Original one-color PNGs do not represent vanilla water/lava art.
    for i,name in enumerate(['water_still','water_flow','water_overlay','lava_still','lava_flow']):
        files[f'assets/minecraft/textures/block/{name}.png']=png([20+i,50+i,100+i,180])
    for name in ['grass','foliage','dry_foliage']:
        files[f'assets/minecraft/textures/colormap/{name}.png']=png([80,150,60,255],1)
    files['data/minecraft/worldgen/biome/plains.json']=encoded({'temperature':0.8,'downfall':0.4,'has_precipitation':True,'effects':{'water_color':4159204,'water_fog_color':329011,'fog_color':12638463,'sky_color':7907327}})
    return files

RESOURCE_KINDS=('ordinary','ambiguous','opaque','incompatible','fusion')
def sheet_png():
    def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    # Fusion full layout is 8x6 tiles (official Layout::grid), not a model.
    width,height=128,96
    raw=b''.join(b'\0'+b''.join(bytes([30+(x//16)*20,40+(y//16)*20,100,160]) for x in range(width)) for y in range(height))
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b'')

def baseline(official):
    meta=json.loads((ROOT/'tools/terrain-cell-source.json').read_text())
    text=(official/meta['file']).read_text()
    if sha(text.encode())!=meta['file_sha256']: raise ValueError('official mesher source differs')
    if text.count(meta['start'])!=1:raise ValueError('ambiguous native baseline start')
    start=text.index(meta['start']); original=text[start:text.index(meta['end'],start)].rstrip()
    if sha(original.encode())!=meta['function_sha256']: raise ValueError('official baseline extraction differs')
    native=original.replace('    pub fn mesh_reusing(','fn native_baseline(',1).replace('&mut self,',"mesher: &mut Mesher<'_>,",1)
    native=re.sub(r'\bself\b','mesher',native)
    template=(HERE/'fixture.rs').read_text()
    if template.count('// INSERT EXACT NATIVE BASELINE')!=1: raise ValueError('baseline marker differs')
    generated=template.replace('// INSERT EXACT NATIVE BASELINE',native)
    # Deliberately no formatter/compiler: exact mechanical receiver/name rewrite.
    return generated.encode(),meta

def prepare(official,bundle,output):
    if output.exists(): raise ValueError('output must not exist')
    official=official.resolve();bundle=bundle.resolve();output=output.resolve()
    marker=(official/'.horizon-revision').read_text().strip()
    if marker!=REVISION: raise ValueError('wrong official source revision')
    if sha(bundle.read_bytes())!=BUNDLE_SHA256: raise ValueError('not the exact successful 43f production bundle')
    generated,meta=baseline(official)
    version_source=official/'crates/horizon-core/src/version.rs'
    values=re.findall(r'pub const DATA_VERSION: i32 = (\d+);',version_source.read_text())
    if len(values)!=1: raise ValueError('unrecognized official data constant')
    original={}
    with zipfile.ZipFile(bundle) as archive:
        for item in archive.infolist():
            if not item.filename.startswith(PREFIX): continue
            rel=Path(item.filename).relative_to(PREFIX)
            if rel.is_absolute() or '..' in rel.parts or item.is_dir(): raise ValueError('invalid package entry')
            if str(rel) in original: raise ValueError('duplicate package entry')
            original[str(rel)]=archive.read(item)
    manifest=tomllib.loads(original['horizon-mixin.toml'].decode())
    if manifest['compatibility']['horizon_revision']!=REVISION: raise ValueError('bundle target differs')
    for entry in meta['modules']:
        if sha(original['pluto_native_creative/'+entry['source']])!=entry['sha256']: raise ValueError('production terrain source differs')
    if sha(original['pluto_native_creative/terrain-cell-head.rs'])!=meta['helper_sha256']: raise ValueError('production helper differs')
    if any(m.get('name')=='pluto_terrain_material_cpu' for m in manifest.get('module',[])): raise ValueError('fixture already installed')
    output.mkdir(parents=True)
    modroot=output/'mods/pluto_native_creative'
    for relative,data in original.items():
        path=modroot/relative;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
    (modroot/'horizon-mixin.toml').write_bytes(original['horizon-mixin.toml']+ENTRY.encode())
    (modroot/MODULE).write_bytes(generated)
    resource_hashes={}
    for sub in RESOURCE_KINDS:
        for name,data in assets(int(values[0]),sub).items():
            relative=f'resources/{sub}/{name}';path=output/relative;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data);resource_hashes[relative]=sha(data)
    native_paths=['crates/horizon-client-render/src/model_library.rs','crates/horizon-client-render/src/fusion.rs','crates/horizon-optifine/src/fusion/layout.rs','crates/horizon-optifine/src/fusion/texture.rs','Cargo.toml','Cargo.lock','crates/horizon-client-render/src/mesher.rs','crates/horizon-client-render/src/assets.rs','crates/horizon-client-render/src/models/catalog.rs','crates/horizon-client-render/src/sprites.rs','crates/horizon-client-render/src/biome.rs','crates/horizon-world/src/block_render.rs','crates/horizon-items/src/blocks.rs','crates/horizon-core/src/version.rs']
    evidence={'plan_counts':{k:len(manifest.get(v,[]))+(1 if k=='modules' else 0) for k,v in [('modules','module'),('injections','injection'),('states','state'),('crates','crate')]},'production_head':PRODUCTION_HEAD,'preparer_sha256':sha(Path(__file__).read_bytes()),'native_input_sha256':{p:sha((official/p).read_bytes()) for p in native_paths},'official':str(official),'horizon_revision':REVISION,'official_mesher_sha256':meta['file_sha256'],'baseline_function_sha256':meta['function_sha256'],'version_source_sha256':sha(version_source.read_bytes()),'bundle':str(bundle),'bundle_sha256':sha(bundle.read_bytes()),'production_entries':{p:sha(v) for p,v in original.items()},'fixture_template_sha256':sha((HERE/'fixture.rs').read_bytes()),'generated_fixture_sha256':sha(generated),'resource_sha256':resource_hashes,'scope':'Actual official Assets loader, StateModels baker, FaceVisibility, MeshTables, Biomes and installed Mesher. Original synthetic facts/models/textures; no modeled native owners, no production vanilla asset or GPU proof. BlockFacts are explicit synthetic actual native structs. RenderAssets are assembled from the actual native StateModels/Sprites outputs, not full RenderAssets::load/gallery.'}
    (output/'fixture-sources.json').write_text(json.dumps(evidence,indent=2)+'\n')
    verify(output)
    return evidence

def verify(output):
    e=json.loads((output/'fixture-sources.json').read_text());bundle=Path(e['bundle']);official=Path(e['official'])
    if e['production_head']!=PRODUCTION_HEAD or e['bundle_sha256']!=BUNDLE_SHA256: raise ValueError('production cutoff changed')
    if sha(Path(__file__).read_bytes())!=e['preparer_sha256']:raise ValueError('preparer changed')
    if sha(bundle.read_bytes())!=e['bundle_sha256']: raise ValueError('fixture bundle changed')
    if sha((HERE/'fixture.rs').read_bytes())!=e['fixture_template_sha256']: raise ValueError('fixture template changed')
    for name,digest in e['native_input_sha256'].items():
        if sha((official/name).read_bytes())!=digest:raise ValueError('official fixture input changed: '+name)
    generated,_=baseline(official)
    if sha(generated)!=e['generated_fixture_sha256']: raise ValueError('regenerated native baseline differs')
    modroot=output/'mods/pluto_native_creative'
    with zipfile.ZipFile(bundle) as archive:
        for name,digest in e['production_entries'].items():
            raw=archive.read(PREFIX+name)
            if sha(raw)!=digest: raise ValueError('bundle payload changed')
            expected=raw+ENTRY.encode() if name=='horizon-mixin.toml' else raw
            if (modroot/name).read_bytes()!=expected: raise ValueError('overlay changed production bytes: '+name)
    if (modroot/MODULE).read_bytes()!=generated: raise ValueError('fixture source changed')
    actual={str(p.relative_to(modroot)) for p in modroot.rglob('*') if p.is_file()}
    if actual!=set(e['production_entries'])|{MODULE}:raise ValueError('unreviewed extra overlay files')
    for name,digest in e['resource_sha256'].items():
        if sha((output/name).read_bytes())!=digest: raise ValueError('synthetic resource changed: '+name)
    actual_resources={str(p.relative_to(output)) for p in (output/'resources').rglob('*') if p.is_file()}
    if actual_resources!=set(e['resource_sha256']):raise ValueError('unreviewed resource file set')
    expected={str(p.relative_to(output)) for p in modroot.rglob('*') if p.is_file()}|actual_resources|{'fixture-sources.json'}
    actual_files={str(p.relative_to(output)) for p in output.rglob('*') if p.is_file()}
    if actual_files!=expected:raise ValueError('unreviewed overlay file set')
    if any(p.is_symlink() for p in output.rglob('*')):raise ValueError('overlay symlinks are unsupported')
    return e

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--official',type=Path);p.add_argument('--bundle',type=Path);p.add_argument('--output',type=Path,required=True);p.add_argument('--verify',action='store_true');a=p.parse_args()
    try:
        if a.verify:verify(a.output)
        elif a.official is None or a.bundle is None:p.error('--official and --bundle are required')
        else:prepare(a.official,a.bundle,a.output)
    except (ValueError,OSError,KeyError) as error:p.exit(1,str(error)+'\n')
    print('Exact production bytes, native baseline and original synthetic resources verified.')
