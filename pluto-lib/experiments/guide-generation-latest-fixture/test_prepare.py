"""Pure source/wiring checks. No native checkout, metadata or Rust execution."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest
from unittest import mock

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('generation_prepare',HERE/'prepare.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

class Checks(unittest.TestCase):
    def test_exact_native_pin_and_test_scope(self):
        v=json.loads((HERE/'inputs.json').read_text())
        self.assertEqual(v['official_revision'],m.HORIZON)
        self.assertEqual((len(v['generation_names']),len(v['inventory_names']),v['expected_filtered']),(6,21,15))
        self.assertEqual(len(set(v['inventory_names'])),21)
        self.assertTrue(set(v['generation_names'])<=set(v['inventory_names']))
        self.assertEqual(len(v['api_sha256']),29)
        self.assertEqual(v['official_sha256']['Cargo.lock'],'0bc4234f3001d44df9d46fbb06df331d3f53b1af6b613ec9082baaec3d93c62f')
    def test_no_nested_process_or_metadata_execution(self):
        source=(HERE/'prepare.py').read_text()
        for token in ['import subprocess','Popen(','check_output(','cargo metadata','os.system(']:self.assertNotIn(token,source)
    def test_relative_old_and_new_fixture_paths_survive(self):
        self.assertEqual(m.COPIES['native/guide-generation-query/native.rs'],'tconstruct/tests/guide-generation-query/native.rs')
        self.assertEqual(m.COPIES['native/guide-text-migration/native.rs'],'tconstruct/tests/guide-text-migration/native.rs')
        self.assertIn('native/guide-text-migration/guide_font_provider.json',m.COPIES)
        self.assertIn('#[path="native/guide-generation-query/native.rs"]',m.LIB)
        self.assertNotIn('guide_layout_tests',m.LIB)
    def test_actual_core_render_api_are_shared_not_mocked(self):
        for text in ['horizon_client_core::text','horizon_client_render::font','pluto_inventory_api as pluto_inventory','extern crate self as horizon_client']:
            self.assertIn(text,m.LIB)
        for text in ['struct Text','struct Font','struct Game','include_str!']:
            self.assertNotIn(text,m.LIB)
    def test_complete_file_scan_rejects_links(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'one').write_text('a');self.assertEqual(set(m.files(p)),{'one'})
            (p/'link').symlink_to(p/'one')
            with self.assertRaises(ValueError):m.files(p)
    def test_regular_input_rejects_missing_link_and_escape(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'sub').mkdir();(p/'one').write_text('a')
            self.assertEqual(m.read(p,'one'),b'a')
            (p/'sub/link').symlink_to(p/'one')
            for name in ['missing','sub/link','../outside']:
                with self.assertRaises(ValueError):m.read(p,name)
    def test_manifest_has_three_exact_official_patches(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'Cargo.toml').write_text('[patch.crates-io]\n'+''.join(n+'={path="third_party/'+n+'"}\n' for n in ['wgpu-hal','naga','libmimalloc-sys']))
            v=tomllib.loads(m.manifest(p))
            self.assertEqual(set(v['patch']['crates-io']),{'wgpu-hal','naga','libmimalloc-sys'})
            self.assertEqual(v['workspace'],{'exclude':['inventory-api'],'default-members':['.']})
            self.assertEqual(v['dependencies']['pluto-inventory-api'],{'path':'inventory-api'})
            self.assertEqual(v['dependencies']['horizon-client-render']['default-features'],False)
            self.assertEqual(v['dependencies']['horizon-client-core']['path'],str(p/'crates/horizon-client-core'))
    def test_manifest_rejects_unreviewed_or_escaping_patch(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)
            for extra in ['','naga={path="../escape"}\nlibmimalloc-sys={path="ok"}\n']:
                (p/'Cargo.toml').write_text('[patch.crates-io]\nwgpu-hal={path="ok"}\n'+extra)
                with self.assertRaises(ValueError):m.manifest(p)
    def test_output_must_be_fresh_before_any_copy(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);host=p/'host';host.mkdir();output=p/'output';output.mkdir();(output/'sentinel').write_text('kept')
            with mock.patch.object(m,'config',return_value={}),mock.patch.object(m,'check_official_inputs',side_effect=AssertionError('must not read yet')):
                with self.assertRaises(ValueError):m.prepare(host,output)
            self.assertEqual((output/'sentinel').read_text(),'kept')
    def test_overlapping_official_output_is_rejected_before_reads(self):
        with tempfile.TemporaryDirectory() as d:
            host=Path(d)
            with mock.patch.object(m,'config',return_value={}),mock.patch.object(m,'check_official_inputs',side_effect=AssertionError('must not read yet')):
                with self.assertRaises(ValueError):m.prepare(host,host/'new')
    def test_selected_official_drift_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'Cargo.lock').write_text('original');v={'official_sha256':{'Cargo.lock':m.sha(b'original')}}
            m.check_official_inputs(p,v);(p/'Cargo.lock').write_text('changed')
            with self.assertRaises(ValueError):m.check_official_inputs(p,v)
    def test_verified_fixture_set_is_derived_not_receipt_authority(self):
        source=(HERE/'prepare.py').read_text()
        self.assertIn("source_names=set(COPIES)",source)
        self.assertIn("set(evidence['source_sha256'])==source_names",source)
        self.assertIn("set(actual)==expected",source)
        self.assertIn("files(output/'inventory-api')==value['api_sha256']",source)
    def test_lock_and_metadata_checks_reject_duplicate_or_foreign_identity(self):
        source=(HERE/'prepare.py').read_text()
        for text in ['not lock-allowed-LOCAL','LOCAL<=lock','duplicate lock package','metadata outside adapted lock','duplicate metadata package','wrong fixture workspace','external local dependency','wrong actual native dependency']:
            self.assertIn(text,source)
    def test_native_source_and_proof_scope_remain_separate(self):
        v=json.loads((HERE/'inputs.json').read_text())
        self.assertEqual(v['source_publication'],'d1941c7702cf9ce45c4f245962e83148c8a26806')
        self.assertEqual(v['prior_font12'],'49a2401eb3957e0766c8e68adbc45d69882aa793')
        self.assertIn('No workflow/runner', (HERE/'README.md').read_text())


    @staticmethod
    def workspace():
        return {'packages':[{'name':'pluto-guide-generation-tests','id':'fixture'},
                            {'name':'pluto-inventory-api','id':'api'}],
                'workspace_members':['fixture'],'workspace_default_members':['fixture'],
                'workspace_root':'/fixture'}
    def test_exact_root_member_keeps_api_as_dependency(self):
        metadata=self.workspace();before=copy.deepcopy(metadata)
        m.verify_workspace(Path('/fixture'),metadata)
        self.assertEqual(metadata,before)
    def test_extra_duplicate_missing_or_api_member_rejects(self):
        for members in [[],['api'],['fixture','api'],['fixture','foreign'],['fixture','fixture']]:
            metadata=self.workspace();metadata['workspace_members']=members
            with self.assertRaises(ValueError):m.verify_workspace(Path('/fixture'),metadata)
        metadata=self.workspace();metadata['packages'].append(dict(metadata['packages'][0]))
        with self.assertRaises(ValueError):m.verify_workspace(Path('/fixture'),metadata)
    def test_default_member_and_root_must_match_fixture(self):
        for defaults in [[],['api'],['fixture','api'],['fixture','fixture']]:
            metadata=self.workspace();metadata['workspace_default_members']=defaults
            with self.assertRaises(ValueError):m.verify_workspace(Path('/fixture'),metadata)
        metadata=self.workspace();metadata['workspace_root']='/other'
        with self.assertRaises(ValueError):m.verify_workspace(Path('/fixture'),metadata)

    def test_frozen_current_source_inputs_and_full_api_set(self):
        value=json.loads((HERE/'inputs.json').read_text())
        for name,digest in value['preparer_sha256'].items():
            self.assertEqual(m.sha((HERE/name).read_bytes()),digest)
        def source(name):
            path=m.ROOT/name
            if path.is_file():return path.read_bytes()
            return subprocess.check_output(['git','-C',str(m.ROOT),'show','HEAD:'+name])
        for name,digest in value['source_sha256'].items():self.assertEqual(m.sha(source(name)),digest,name)
        tracked=subprocess.check_output(['git','-C',str(m.ROOT),'ls-tree','-r','--name-only','HEAD',m.API],text=True).splitlines()
        expected={p[len(m.API)+1:]:m.sha(source(p)) for p in tracked}
        self.assertEqual(expected,value['api_sha256'])

if __name__=='__main__':unittest.main()
