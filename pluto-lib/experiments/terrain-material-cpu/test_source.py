#!/usr/bin/env python3
"""Source-only checks. No planner, formatter, Cargo, rustc or native execution."""
import ast
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import struct
import tomllib
import unittest
import zlib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
OFFICIAL = Path(os.environ.get('PLUTO_OFFICIAL_SOURCE', ROOT.parents[1]/'.devtools/latest-official-export-20261105/horizon'))
def load(name):
    spec=importlib.util.spec_from_file_location(name,HERE/(name+'.py'))
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
P=load('prepare');C=load('check_plan')
def pixels(raw):
    assert raw[:8]==b'\x89PNG\r\n\x1a\n'
    pos=8;data=b'';size=None
    while pos<len(raw):
        n=int.from_bytes(raw[pos:pos+4],'big');kind=raw[pos+4:pos+8];payload=raw[pos+8:pos+8+n]
        assert zlib.crc32(kind+payload)&0xffffffff==int.from_bytes(raw[pos+8+n:pos+12+n],'big')
        if kind==b'IHDR':size=struct.unpack('>IIBBBBB',payload)
        if kind==b'IDAT':data+=payload
        pos+=12+n
    w,h,depth,kind,*_=size;assert (depth,kind)==(8,6)
    data=zlib.decompress(data);assert len(data)==h*(1+w*4)
    assert all(data[y*(1+w*4)]==0 for y in range(h))
    return w,h,b''.join(data[y*(1+w*4)+1:(y+1)*(1+w*4)] for y in range(h))

class Source(unittest.TestCase):
    def test_frozen_sources(self):
        cfg=json.loads((HERE/'inputs.json').read_text())
        for path,digest in cfg['experiment_sha256'].items():
            self.assertEqual(P.sha((HERE/path).read_bytes()),digest,path)
        for path,digest in cfg['production_sha256'].items():
            self.assertEqual(P.sha((ROOT/path).read_bytes()),digest,path)
        self.assertEqual(cfg['production_head'],P.PRODUCTION_HEAD)
        self.assertEqual(cfg['bundle_sha256'],P.BUNDLE_SHA256)
    def test_exact_native_baseline_inverse(self):
        generated,meta=P.baseline(OFFICIAL)
        native=(OFFICIAL/meta['file']).read_text();a=native.index(meta['start'])
        original=native[a:native.index(meta['end'],a)].rstrip()
        template=(HERE/'fixture.rs').read_text();before,after=template.split('// INSERT EXACT NATIVE BASELINE')
        text=generated.decode();self.assertTrue(text.startswith(before));self.assertTrue(text.endswith(after))
        body=text[len(before):-len(after)]
        inverse=re.sub(r'\bmesher\b','self',body).replace('fn native_baseline(','    pub fn mesh_reusing(',1).replace("self: &mut Mesher<'_>,",'&mut self,',1)
        self.assertEqual(inverse,original)
        self.assertEqual(P.sha(original.encode()),meta['function_sha256'])
    def test_preparation_cannot_launch_tools(self):
        tree=ast.parse((HERE/'prepare.py').read_text())
        imports=[n.module for n in ast.walk(tree) if isinstance(n,ast.ImportFrom)]
        imports += [a.name for n in ast.walk(tree) if isinstance(n,ast.Import) for a in n.names]
        self.assertFalse(set(imports)&{'subprocess','os','shutil'})
    def test_one_test_only_module(self):
        m=tomllib.loads(P.ENTRY)
        self.assertEqual(set(m),{'module'});self.assertEqual(len(m['module']),1)
        self.assertEqual(m['module'][0]['file'],'crates/horizon-client-render/src/mesher.rs')
        self.assertIn('#[cfg(test)]\nmod tests {',(HERE/'fixture.rs').read_text())
        self.assertNotIn('#![',(HERE/'fixture.rs').read_text())
    def test_resource_variants_are_bounded_original_inputs(self):
        all_sets=[P.assets(0,k) for k in P.RESOURCE_KINDS]
        self.assertLess(sum(sum(map(len,s.values())) for s in all_sets),65536)
        for resources in all_sets:
            self.assertTrue(all(not Path(p).is_absolute() and '..' not in Path(p).parts for p in resources))
            self.assertTrue(all(p.startswith(('assets/','data/')) or p.endswith('.json') for p in resources))
        with self.assertRaises(ValueError):P.assets(0,'unknown')
    def test_material_texture_and_real_model_are_distinct(self):
        r=P.assets(0)
        self.assertNotEqual(r['assets/pluto_fixture/textures/block/cell.png'],r['assets/pluto_fixture/textures/block/flow.png'])
        model=json.loads(r['assets/pluto_fixture/models/block/flow.json'])
        self.assertEqual(model['textures']['all'],'pluto_fixture:block/flow')
        w,h,data=pixels(r['assets/pluto_fixture/textures/block/flow.png'])
        self.assertEqual((w,h),(16,32));self.assertEqual(set(data[3::4]),{160})
    def test_opaque_range_and_incompatible_shade(self):
        r=P.assets(0,'opaque')
        for name in ('cell','flow'):
            model=json.loads(r[f'assets/pluto_fixture/models/block/{name}.json'])
            self.assertTrue(all(f['uv']==[4,4,12,12] for e in model['elements'] for f in e['faces'].values()))
            self.assertEqual(set(pixels(r[f'assets/pluto_fixture/textures/block/{name}.png'])[2][3::4]),{255})
        r=P.assets(0,'incompatible');flow=json.loads(r['assets/pluto_fixture/models/block/flow.json'])
        self.assertIs(flow['elements'][0]['shade'],False)
    def test_fusion_real_full_sheet_contract(self):
        r=P.assets(0,'fusion');flow=json.loads(r['assets/pluto_fixture/models/block/flow.json'])
        self.assertEqual(flow['loader'],'fusion:model');self.assertEqual(flow['type'],'fusion:connecting')
        meta=json.loads(r['assets/pluto_fixture/textures/block/flow.png.mcmeta'])
        self.assertEqual(meta,{'fusion':{'type':'connecting','layout':'full'}})
        self.assertEqual(pixels(r['assets/pluto_fixture/textures/block/flow.png'])[:2],(128,96))
        self.assertIn('Self::Full => (8, 6)',(OFFICIAL/'crates/horizon-optifine/src/fusion/layout.rs').read_text())
    def test_fallback_oracle_and_replay_observations_retained(self):
        s=(HERE/'fixture.rs').read_text()
        for field in ('solid','translucent','sides','packed','solid_tints','visibility','block_entities','lights','sprite_layers','bounds','keep'):
            self.assertIn('a.'+field,s)
        for value in ('a.padded.blocks','a.padded.light','a.solid_rows','a.leaf.interior','resume_valid_claim && kind == "single"'):
            self.assertIn(value,s)
        self.assertIn('native_baseline(&mut baseline',s)
    def test_native_cases_have_no_silent_skip(self):
        s=(HERE/'fixture.rs').read_text()
        self.assertEqual(s.count('#[test]'),1);self.assertNotIn('#[ignore]',s)
        self.assertIn('mode == "no-provider" || mode == "registered"',s)
        self.assertIn('material_cases(&root)',s)
        self.assertIn('explicit generated resources are required',s)
        self.assertIn('FusionTextures::load(root, assets.registry',s)
        self.assertNotIn('NonZeroU16::new',s)
    def test_plan_rejects_foreign_extra_and_changed_fixture(self):
        module={'mod_id':'pluto_native_creative','module_id':'terrain-material-cpu-fixture',
                'file':'crates/horizon-client-render/src/mesher.rs','name':'pluto_terrain_material_cpu',
                'fingerprint':'f','source':'s'}
        plan={'schema':6,'modules':[module],'states':[],'injections':[],'crates':[],'inputs':{}}
        evidence={'native_input_sha256':{},'plan_counts':{'modules':1,'states':0,'injections':0,'crates':0}}
        C.check(plan,evidence,'s','f')
        for key,value in [('mod_id','foreign'),('source','changed')]:
            bad=json.loads(json.dumps(plan));bad['modules'][0][key]=value
            with self.assertRaises(ValueError):C.check(bad,evidence,'s','f')
        bad=json.loads(json.dumps(plan));bad['modules'].append(module)
        with self.assertRaises(ValueError):C.check(bad,evidence,'s','f')
    def test_runtime_and_scope_are_not_replaced(self):
        s=(HERE/'README.md').read_text()
        for phrase in ('native unrun','no GPU','not TConstruct activation','43f17427'):
            self.assertIn(phrase,s)
        self.assertNotIn('import subprocess',(HERE/'prepare.py').read_text())
        self.assertIn('if actual_resources!=set(e[\'resource_sha256\'])',(HERE/'prepare.py').read_text())

if __name__=='__main__':unittest.main()
