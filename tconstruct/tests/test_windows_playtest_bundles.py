"""Pure synthetic archive tests, not real integration packaging/plan/build proof."""
import io
import json
from pathlib import Path
import stat
import sys
import tempfile
import unittest
import zipfile
from unittest.mock import patch
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
import prepare_playtest_bundles as b


def archive(files):
    out=io.BytesIO()
    with zipfile.ZipFile(out,'w') as z:
        for name,raw in files:
            x=zipfile.ZipInfo(name);x.external_attr=0o100644<<16;x.compress_type=zipfile.ZIP_DEFLATED;z.writestr(x,raw)
    return out.getvalue()


def package(identity):
    h={'schema':1,'id':identity,'version':'0.1','game':'26.3','horizon':b.HORIZON,'mixins':b.MIXINS}
    prefix='mixins/'+identity+'/'
    files=[('horizon-native.json',json.dumps(h).encode()),(prefix+'horizon-mixin.toml',f'[mod]\nid="{identity}"\nversion="0.1"\n[compatibility]\nhorizon_revision="{b.HORIZON}"\n'.encode()),(prefix+'helper.rs',b'fn helper() {}\n')]
    if identity=='tconstruct_native':files.append((prefix+'data/tconstruct-support.zip',archive([('tconstruct/tools/test.py',b'pass\n')])))
    return archive(files)


class Tests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
    def test_two_packages_keep_limits_and_complete_combined_seal(self):
        a,ra=b.inspect(package('tconstruct_native'),'tconstruct_native');c,rc=b.inspect(package('pluto_native_creative'),'pluto_native_creative')
        files,seal=b.combine([a,c]);self.assertEqual(seal['members'],5)
        self.assertEqual(seal['members_seal_sha256'],b.map_seal(seal['member_sha256']))
        self.assertEqual(ra['support']['members'],1);self.assertNotIn('horizon-native.json',files)
    def test_unsafe_windows_and_traversal_paths_rejected(self):
        for name in ('/escape','../escape','a/../b','C:/escape','a\\b','a//b','a/','NUL.txt','a/COM1','a./b','a?/b','a\0b'):
            with self.subTest(name=name):
                with self.assertRaises(ValueError):b.safe_name(name)
    def test_duplicate_case_and_mode_rejected(self):
        for names in [('a','a'),('a','A')]:
            with self.assertRaises(ValueError):b.members(archive([(x,b'') for x in names]),512)
        out=io.BytesIO()
        with zipfile.ZipFile(out,'w') as z:
            i=zipfile.ZipInfo('link');i.external_attr=(stat.S_IFLNK|0o777)<<16;z.writestr(i,b'target')
        with self.assertRaises(ValueError):b.members(out.getvalue(),512)
    def test_budgets_and_bad_header_reject(self):
        raw=package('pluto_native_creative')
        for key in ('MAX_ARCHIVE','MAX_UNPACKED','MAX_ENTRIES'):
            with patch.object(b,key,0):
                with self.assertRaises(ValueError):b.inspect(raw,'pluto_native_creative')
        with self.assertRaises(ValueError):b.inspect(raw,'tconstruct_native')
    def test_wrong_revision_and_manifest_reject(self):
        files,_=b.members(package('pluto_native_creative'),512)
        h=json.loads(files['horizon-native.json']);h['horizon']='0'*40;files['horizon-native.json']=json.dumps(h).encode()
        with self.assertRaises(ValueError):b.inspect(archive(list(files.items())),'pluto_native_creative')
        files,_=b.members(package('pluto_native_creative'),512);files['mixins/pluto_native_creative/horizon-mixin.toml']=b'[mod]\nid="wrong"\n'
        with self.assertRaises(ValueError):b.inspect(archive(list(files.items())),'pluto_native_creative')
    def test_nested_support_paths_and_budget(self):
        files,_=b.members(package('tconstruct_native'),512);files['mixins/tconstruct_native/data/tconstruct-support.zip']=archive([('elsewhere.py',b'')])
        with self.assertRaises(ValueError):b.inspect(archive(list(files.items())),'tconstruct_native')
        with patch.object(b,'MAX_SUPPORT_ENTRIES',0):
            with self.assertRaises(ValueError):b.inspect(package('tconstruct_native'),'tconstruct_native')
    def test_combined_collision_rejected(self):
        files,_=b.inspect(package('pluto_native_creative'),'pluto_native_creative')
        with self.assertRaises(ValueError):b.combine([files,files])
    def runner(self,argv,**kwargs):
        output=Path(argv[-1]);identity='tconstruct_native' if '--output' in argv else 'pluto_native_creative';output.write_bytes(package(identity))
    def test_prepare_uses_both_packagers_and_cache_exactness(self):
        home=self.root/'home';home.mkdir();repo=self.root/'repo';repo.mkdir();calls=[]
        def run(argv,**kwargs):calls.append(argv);self.runner(argv,**kwargs)
        tc,pl,receipt=b.prepare(repo,home,run,'python',self.root,{})
        self.assertEqual(len(calls),2);self.assertIn('--output',calls[0]);self.assertIn('package_native.py',str(calls[1][1]))
        tc2,pl2,r2=b.prepare(repo,home,run,'python',self.root,{})
        self.assertEqual((tc,pl,receipt),(tc2,pl2,r2))
        self.assertEqual(b.verify_prepared(tc,pl,receipt),receipt)
        (pl/'helper.rs').write_bytes(b'changed')
        with self.assertRaises(ValueError):b.verify_prepared(tc,pl,receipt)
        with self.assertRaises(ValueError):b.prepare(repo,home,run,'python',self.root,{})
    def test_prepared_tc_data_and_archive_drift_reject(self):
        home=self.root/'home';home.mkdir();repo=self.root/'repo';repo.mkdir()
        tc,pl,receipt=b.prepare(repo,home,self.runner,'python',self.root,{})
        path=tc/'tconstruct_native/data/tconstruct-support.zip';old=path.read_bytes();path.write_bytes(b'changed')
        with self.assertRaises(ValueError):b.verify_prepared(tc,pl,receipt)
        path.write_bytes(old);archive_path=tc.parent/'tconstruct.hnmod';archive_path.write_bytes(b'changed')
        with self.assertRaises(ValueError):b.verify_prepared(tc,pl,receipt)
    def test_external_bundle_requires_all_files_not_only_identity(self):
        home=self.root/'home';home.mkdir();repo=self.root/'repo';repo.mkdir();external=self.root/'external';external.mkdir()
        files,_=b.inspect(package('pluto_native_creative'),'pluto_native_creative')
        for name,raw in files.items():
            if name.startswith('mixins/pluto_native_creative/'):(external/name.split('/',2)[2]).write_bytes(raw)
        b.prepare(repo,home,self.runner,'python',self.root,{},external)
        (external/'helper.rs').write_bytes(b'wrong')
        with self.assertRaises(ValueError):b.prepare(repo,home,self.runner,'python',self.root,{},external)
    def test_fail_closed_packager_without_extracting_partial_files(self):
        home=self.root/'home';home.mkdir();repo=self.root/'repo';repo.mkdir()
        def fail(*a,**k):raise ValueError('packager refused inconsistent source')
        with self.assertRaises(ValueError):b.prepare(repo,home,fail,'python',self.root,{})
        self.assertEqual(list((home/'packages-latest').iterdir()),[])

if __name__=='__main__':unittest.main()
