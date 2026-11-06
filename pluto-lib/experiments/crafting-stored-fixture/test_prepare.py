"""Pure checks, mocked children only; never prepare a real host or invoke Cargo."""
import copy,importlib.util,json,os,subprocess,tempfile,tomllib,unittest
from pathlib import Path
from unittest.mock import Mock,patch
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('stored_prepare',HERE/'prepare.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

class Checks(unittest.TestCase):
    def test_frozen_153_and_16_inventories(self):
        d=m.inputs();self.assertEqual(len(d['api_base_sha256']),33);self.assertEqual(len(d['candidate_sha256']),11)
        self.assertEqual(len(set(d['api_expected_tests'])),153);self.assertEqual(len(d['fixture_expected_tests']),16)
        self.assertFalse(set(d['api_existing_tests'])&set(d['api_added_tests']))
    def test_single_matching_overlay(self):
        d=m.inputs();self.assertEqual(set(d['api_base_sha256']),set(d['api_candidate_sha256']))
        self.assertEqual([n for n in d['api_base_sha256'] if d['api_base_sha256'][n]!=d['api_candidate_sha256'][n]],['src/crafting/matching.rs'])
    def test_api_manifest_preserved_and_native_manifest_one_path(self):
        workspace,fixture=m.manifests(Path('/official'));a=tomllib.loads(workspace);f=tomllib.loads(fixture)
        self.assertEqual(a['workspace']['members'],['inventory-api','fixture'])
        self.assertEqual(a['workspace']['default-members'],['inventory-api','fixture'])
        self.assertEqual(f['dependencies']['pluto-inventory-api'],{'path':'../inventory-api'})
        self.assertNotIn('patch',f);self.assertNotIn('features',f)
        for n in ('horizon-items','horizon-client-core','horizon-world'):self.assertEqual(f['dependencies'][n],{'path':'/official/crates/'+n})
    def test_exact_recursive_files(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'src').mkdir();(p/'src/a.rs').write_bytes(b'x');m.exact(p,{'src/a.rs':m.sha(b'x')})
            (p/'extra').mkdir()
            with self.assertRaises(ValueError):m.exact(p,{'src/a.rs':m.sha(b'x')})
    def test_extra_hidden_config_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'a').write_bytes(b'x');(p/'.cargo').mkdir();(p/'.cargo/config.toml').write_bytes(b'')
            with self.assertRaises(ValueError):m.exact(p,{'a':m.sha(b'x')})
    def test_symlink_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'a').symlink_to('/nonexistent')
            with self.assertRaises(ValueError):m.exact(p,{'a':m.sha(b'x')})
    def test_mode_and_link_are_sealed(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'a').write_bytes(b'x');(p/'link').symlink_to('a');before=m.tree(p);(p/'a').chmod(0o755)
            self.assertNotEqual(before['a']['mode'],m.tree(p)['a']['mode']);self.assertEqual(before['link'],{'kind':'link','target':'a'})
    def model(self):
        host,out=Path('/official'),Path('/fixture')
        old={'package':[{'name':n,'version':'0.1.0'} for n in m.NATIVE]+[{'name':'serde_json','version':'1.0.149','source':'registry+official','checksum':'fixture-only'}]}
        new={'package':copy.deepcopy(old['package'])+[{'name':m.API_NAME,'version':'0.1.0'},{'name':m.NAME,'version':'0.0.0'}]}
        pk=[]
        for p in new['package']:
            path=out/('inventory-api' if p['name']==m.API_NAME else 'fixture')/'Cargo.toml' if p['name'] in (m.NAME,m.API_NAME) else host/'crates'/p['name']/'Cargo.toml'
            pk.append(dict(p,id='id:'+p['name'],source=p.get('source'),manifest_path=str(path)))
        meta={'version':1,'workspace_root':str(out),'packages':pk,'workspace_members':['id:'+m.API_NAME,'id:'+m.NAME],'workspace_default_members':['id:'+m.API_NAME,'id:'+m.NAME]}
        return host,out,meta,old,new
    def test_exact_dependency_model_accepted(self):m.validate_dependencies(*self.model())
    def test_foreign_registry_version_rejected(self):
        a=self.model();a[-1]['package'][4]['version']='2.0.0'
        with self.assertRaises(ValueError):m.validate_dependencies(*a)
    def test_duplicate_api_package_rejected(self):
        a=self.model();p=copy.deepcopy(a[2]['packages'][-2]);p['id']='another';a[2]['packages'].append(p)
        with self.assertRaises(ValueError):m.validate_dependencies(*a)
    def test_second_api_path_rejected(self):
        a=self.model();a[2]['packages'][-2]['manifest_path']='/another/inventory-api/Cargo.toml'
        with self.assertRaises(ValueError):m.validate_dependencies(*a)
    def test_registry_api_substitution_rejected(self):
        a=self.model();a[2]['packages'][-2]['source']='registry+official'
        with self.assertRaises(ValueError):m.validate_dependencies(*a)
    def test_native_path_escape_rejected(self):
        a=self.model();a[2]['packages'][0]['manifest_path']='/other/crates/horizon-core/Cargo.toml'
        with self.assertRaises(ValueError):m.validate_dependencies(*a)
    def test_extra_workspace_member_rejected(self):
        a=self.model();a[2]['workspace_members'].append('unexpected')
        with self.assertRaises(ValueError):m.validate_dependencies(*a)
    def test_wrong_workspace_root_rejected(self):
        a=self.model();a[2]['workspace_root']='/other'
        with self.assertRaises(ValueError):m.validate_dependencies(*a)
    def test_wrong_default_members_rejected(self):
        for members in ([], ['id:'+m.API_NAME], ['id:'+m.API_NAME]*2, ['id:'+m.API_NAME,'id:'+m.NAME,'unexpected']):
            a=self.model();a[2]['workspace_default_members']=members
            with self.assertRaises(ValueError):m.validate_dependencies(*a)
    def test_wrong_metadata_schema_rejected(self):
        a=self.model();a[2]['version']=2
        with self.assertRaises(ValueError):m.validate_dependencies(*a)
    def test_uncontrolled_subprocess_rejected_before_spawn(self):
        with patch.dict(os.environ,{},clear=True),patch.object(m.subprocess,'Popen') as popen:
            with self.assertRaises(ValueError):m.run(['git','status'])
            popen.assert_not_called()
    def test_child_inherits_outer_group_and_stdout_is_exact(self):
        p=Mock();p.communicate.return_value=(' output\n','');p.returncode=0;p.poll.return_value=0
        with patch.dict(os.environ,{'PLUTO_FIXTURE_CONTROLLER':'crafting-stored-v1'}),patch.object(m.os,'getpgrp',return_value=12),patch.object(m.os,'getpid',return_value=12),patch.object(m.subprocess,'Popen',return_value=p) as popen:
            self.assertEqual(m.run(['git','show']),' output\n');self.assertFalse(popen.call_args.kwargs['start_new_session']);p.wait.assert_called_once_with(timeout=2)
    def test_timeout_kills_and_reaps_direct_child(self):
        p=Mock();p.communicate.side_effect=subprocess.TimeoutExpired(['git'],60);p.poll.return_value=None
        with patch.dict(os.environ,{'PLUTO_FIXTURE_CONTROLLER':'crafting-stored-v1'}),patch.object(m.os,'getpgrp',return_value=12),patch.object(m.os,'getpid',return_value=12),patch.object(m.subprocess,'Popen',return_value=p):
            with self.assertRaises(subprocess.TimeoutExpired):m.run(['git'])
        p.kill.assert_called_once();p.wait.assert_called_once_with(timeout=2)
    def test_scope_and_no_automatic_test_execution(self):
        s=(HERE/'prepare.py').read_text();self.assertNotIn("['cargo','test'",s)
        self.assertNotIn('start_new_session=True',s)
        self.assertIn("'--untracked-files=all','--ignored=matching'",s)
        self.assertIn("args[:2]+['--locked']+args[2:]",s)
        self.assertFalse(m.inputs()['actual_rust_executed'])

if __name__=='__main__':unittest.main()
