"""Source/generator checks only; no native Mesher execution implied."""
import importlib.util
import json
from pathlib import Path
import struct
import tomllib
import unittest
import zlib
ROOT=Path(__file__).resolve().parents[1]
HERE=ROOT/'experiments/terrain-cell-cpu'
spec=importlib.util.spec_from_file_location('terrain_cpu_fixture',HERE/'prepare.py');fixture=importlib.util.module_from_spec(spec);spec.loader.exec_module(fixture)
class CpuOverlayChecks(unittest.TestCase):
    def test_resources_are_bounded_original_and_have_native_registry_shape(self):
        files=fixture.assets(1234)
        self.assertLess(sum(map(len,files.values())),64*1024)
        facts=json.loads(files['facts.json']);self.assertEqual(len(facts['states']),8)
        self.assertTrue(all(isinstance(row[4],str) for row in facts['states']))
        self.assertEqual(facts['defaults']['pluto_fixture:cell'],7)
        self.assertEqual(json.loads(files['horizon-resources.json'])['data_version'],1234)
    def test_png_animation_has_two_real_frames_and_valid_chunks(self):
        raw=fixture.assets(1234)['assets/pluto_fixture/textures/block/cell.png']
        self.assertTrue(raw.startswith(b'\x89PNG\r\n\x1a\n'));pos=8;pixels=b''
        while pos<len(raw):
            n=struct.unpack('>I',raw[pos:pos+4])[0];kind=raw[pos+4:pos+8];data=raw[pos+8:pos+8+n]
            self.assertEqual(zlib.crc32(kind+data)&0xffffffff,struct.unpack('>I',raw[pos+8+n:pos+12+n])[0])
            if kind==b'IHDR':self.assertEqual(struct.unpack('>II',data[:8]),(16,32))
            if kind==b'IDAT':pixels+=data
            pos+=12+n
        decoded=zlib.decompress(pixels);self.assertEqual(len(decoded),32*(1+16*4));self.assertNotEqual(decoded[1:5],decoded[16*65+1:16*65+5])
    def test_ambiguous_fixture_changes_model_only(self):
        a=fixture.assets(1234);b=fixture.assets(1234,True)
        changed=[name for name in a if a[name]!=b[name]]
        self.assertEqual(changed,['assets/pluto_fixture/models/block/cell.json'])
        self.assertEqual(len(json.loads(b[changed[0]])['elements']),2)
    def test_fixture_is_not_installed_or_skipping(self):
        native=tomllib.loads((ROOT/'mixins/pluto_native_creative/horizon-mixin.toml').read_text())
        self.assertFalse(any(m.get('name')=='pluto_terrain_cell_cpu' for m in native['module']))
        source=(HERE/'fixture.rs').read_text();self.assertNotIn('#[ignore',source)
        self.assertIn('expect("explicit generated resources are required")',source)
        for name in ['Assets','Mesher','RenderAssets','MeshTables','MeshScratch']:self.assertNotIn('struct '+name,source)
        self.assertEqual(source.count('// INSERT EXACT NATIVE BASELINE'),1)
        self.assertIn('scratch_equal(&sa, &sb, mode)',source)
    def test_runner_rejects_zero_test_and_missing_timing_passes(self):
        spec=importlib.util.spec_from_file_location('terrain_cpu_runner',HERE/'run.py');runner=importlib.util.module_from_spec(spec);spec.loader.exec_module(runner)
        for text in ['test result: ok. 0 passed; 0 failed;', 'PLUTO_TERRAIN_MODE no-provider complete;\ntest result: ok. 1 passed; 0 failed;']:
            with self.assertRaises(ValueError):runner.checked_log(text,'no-provider')
        entry={'case':'no-provider','median_ratio':1.0,'baseline_ns_per_section':[1]*7,'adapter_ns_per_section':[1]*7}
        text='PLUTO_TERRAIN_CPU '+json.dumps(entry)+'\nPLUTO_TERRAIN_MODE no-provider complete;\ntest result: ok. 1 passed; 0 failed;'
        self.assertEqual(len(runner.checked_log(text,'no-provider')),1)
    def test_fixture_plan_rejects_wrong_module_or_added_hook(self):
        import copy
        spec=importlib.util.spec_from_file_location('terrain_cpu_plan',HERE/'check_plan.py');guard=importlib.util.module_from_spec(spec);spec.loader.exec_module(guard)
        evidence={'native_input_sha256':{'native.rs':'sha'}}
        plan={'schema':6,'inputs':evidence['native_input_sha256'],'modules':[{'mod_id':'pluto_native_creative','module_id':'terrain-cell-cpu-fixture','file':'crates/horizon-client-render/src/mesher.rs','name':'pluto_terrain_cell_cpu','fingerprint':'fp','source':'reviewed'}]}
        guard.check(plan,evidence,'reviewed','fp')
        for change in [lambda p:p['modules'][0].update(source='changed'),lambda p:p['modules'][0].update(mod_id='foreign'),lambda p:p.update(injections=[{'injection_id':'terrain-cell-cpu-extra'}])]:
            candidate=copy.deepcopy(plan);change(candidate)
            with self.assertRaises(ValueError):guard.check(candidate,evidence,'reviewed','fp')
    def test_budget_replay_expects_valid_claim_to_resume(self):
        source=(HERE/'fixture.rs').read_text()
        self.assertIn('let claimed_replay = resume_valid_claim && kind == "single";',source)
        self.assertIn('if claimed_replay {',source)
        self.assertIn('"valid_claim_resumed":resume_valid_claim',source)
        self.assertNotIn('compare(&f, &chunk("dense"), true, false, true)',source)
if __name__=='__main__':unittest.main()
