"""Pure archive and mocked caller tests; no Git/Cargo/plan/formatter execution."""
import ast
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'tools'))
import stage_package_contract as c
import prepare_playtest_bundles as b
from test_windows_playtest_bundles import package, archive
from windows_source_layers import before_jobs_one


class Tests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name); self.mods = self.root/'tc'; self.pluto = self.root/'pluto'
        self.mods.mkdir(); self.pluto.mkdir()
        self.archives = {}; self.hashes = {}; self.all_files = []
        for identity in sorted(c.IDS):
            files,_ = b.members(package(identity),512)
            version = '0.5.0' if identity == 'tconstruct_native' else '0.1.0'
            header = json.loads(files['horizon-native.json']); header['version']=version
            files['horizon-native.json']=json.dumps(header).encode('utf-8')
            manifest='mixins/'+identity+'/horizon-mixin.toml'
            files[manifest]=files[manifest].replace(b'version="0.1"', ('version="'+version+'"').encode())
            raw = archive(list(files.items())); path = self.root/(identity+'.hnmod'); path.write_bytes(raw)
            self.archives[identity] = path; self.hashes[identity] = b.sha(raw)
            files, _ = b.inspect(raw, identity); self.all_files.append(files)
            for name, value in files.items():
                if name == 'horizon-native.json': continue
                relative = name.removeprefix('mixins/')
                out = (self.mods/relative if identity == 'tconstruct_native'
                       else self.pluto/relative.removeprefix('pluto_native_creative/'))
                out.parent.mkdir(parents=True, exist_ok=True); out.write_bytes(value)
        self.combined = self.root/'combined'; self.combined.mkdir()
        for name, value in b.combine(self.all_files)[0].items():
            out = self.combined/name; out.parent.mkdir(parents=True, exist_ok=True); out.write_bytes(value)
        self.checker = self.root/'checker.py'
        self.checker.write_text("import hashlib\ndef make_package_contract(mode, archives):\n assert mode == 'full-tconstruct'\n return {'schema':1,'mode':mode,'packages':[{'mod_id':i,'archive':str(p),'archive_sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for i,p in sorted(archives.items())]}\n", encoding='utf-8')
    def prepare(self):
        return c.prepare(self.checker, self.archives, self.hashes, self.mods, self.pluto)
    def test_complete_archive_members_and_shared_full_contract(self):
        raw, members = self.prepare(); value = json.loads(raw)
        self.assertEqual(value['mode'], 'full-tconstruct'); self.assertEqual(len(value['packages']), 2)
        self.assertEqual(members, b.combine(self.all_files)[1]['member_sha256'])
        out = self.root/'contract.json'; c.save(out, raw)
        c.verify(out, raw, members, self.archives, self.hashes, self.mods, self.pluto, self.combined)
    def test_exact_actual_recipe_helper_generates_full_contract(self):
        checker=ROOT.parent/'pluto-lib/tools/check_recipe_placement_plan.py'
        self.assertEqual(b.sha(checker.read_bytes()),'7a153e6a4863a3721ead6aa8b96b189b35a2df7bc7464f44950a1184e9cd0496')
        raw,members=c.prepare(checker,self.archives,self.hashes,self.mods,self.pluto)
        value=json.loads(raw);self.assertEqual(value['mode'],'full-tconstruct')
        self.assertEqual(value['horizon'],b.HORIZON);self.assertEqual(value['mixins'],b.MIXINS)
        self.assertEqual({p['mod_id']:p['version'] for p in value['packages']},{'tconstruct_native':'0.5.0','pluto_native_creative':'0.1.0'})
        self.assertEqual({p['mod_id']:p['archive_sha256'] for p in value['packages']},self.hashes)
        self.assertEqual(members,b.combine(self.all_files)[1]['member_sha256'])
    def test_archive_digest_mismatch_precedes_shared_helper(self):
        self.checker.write_text("raise AssertionError('must not import')\n",encoding='utf-8')
        for identity in self.hashes:
            with self.subTest(identity=identity):
                hashes = {**self.hashes, identity:'0'*64}
                with self.assertRaisesRegex(ValueError, 'caller-verified'):
                    c.prepare(self.checker, self.archives, hashes, self.mods, self.pluto)
    def test_missing_extra_duplicate_relative_and_unknown_hash_reject(self):
        a = self.archives; h = self.hashes
        for archives, hashes in [({k:v for k,v in a.items() if k!='tconstruct_native'},h),
                                  ({**a,'extra':self.root/'x'},h),
                                  ({k:a['tconstruct_native'] for k in a},h),
                                  ({**a,'tconstruct_native':Path('relative.hnmod')},h),
                                  (a,{**h,'tconstruct_native':'ABC'}), (a,{})]:
            with self.subTest(archives=archives,hashes=hashes):
                with self.assertRaises(ValueError):c.archive_members(archives,hashes)
    def test_links_and_budgets_reject_without_reading_payload(self):
        p = self.archives['tconstruct_native']; alias=self.root/'alias'; alias.symlink_to(p)
        with self.assertRaises(ValueError):c.bounded_file(alias,b.MAX_ARCHIVE)
        directory_alias=self.root/'directory-alias'; directory_alias.symlink_to(self.root,target_is_directory=True)
        with self.assertRaises(ValueError):c.bounded_file(directory_alias/p.name,b.MAX_ARCHIVE)
        with self.assertRaises(ValueError):c.bounded_file(p,0)
        with patch.object(Path,'open',side_effect=AssertionError('must not read')):
            with self.assertRaises(ValueError):c.bounded_file(p,0)
    def test_missing_extra_or_changed_expanded_member_reject(self):
        p=self.pluto/'helper.rs'; raw=p.read_bytes()
        p.write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'Expanded'):self.prepare()
        p.unlink()
        with self.assertRaisesRegex(ValueError,'Expanded'):self.prepare()
        p.write_bytes(raw); extra=self.mods/'extra'; extra.write_bytes(b'')
        with self.assertRaisesRegex(ValueError,'Expanded'):self.prepare()
    def test_missing_extra_or_changed_copied_member_reject(self):
        raw,members=self.prepare(); out=self.root/'contract.json';c.save(out,raw)
        p=self.combined/'pluto_native_creative/helper.rs'; old=p.read_bytes()
        for value in (None,b'changed'):
            if value is None:p.unlink()
            else:p.write_bytes(value)
            with self.assertRaisesRegex(ValueError,'Copied'):
                c.verify(out,raw,members,self.archives,self.hashes,self.mods,self.pluto,self.combined)
        p.write_bytes(old);(self.combined/'extra').write_bytes(b'')
        with self.assertRaisesRegex(ValueError,'Copied'):
            c.verify(out,raw,members,self.archives,self.hashes,self.mods,self.pluto,self.combined)
    def test_save_is_fresh_and_contract_drift_rejects(self):
        raw,members=self.prepare(); out=self.root/'contract.json';c.save(out,raw)
        with self.assertRaises(FileExistsError):c.save(out,raw)
        out.write_bytes(raw+b' ')
        with self.assertRaisesRegex(ValueError,'contract changed'):
            c.verify(out,raw,members,self.archives,self.hashes,self.mods,self.pluto,self.combined)
    def test_archive_drift_after_contract_and_before_apply_rejects(self):
        raw,members=self.prepare(); out=self.root/'contract.json';c.save(out,raw)
        self.archives['pluto_native_creative'].write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError,'caller-verified'):
            c.verify(out,raw,members,self.archives,self.hashes,self.mods,self.pluto,self.combined)
    def test_shared_helper_failure_or_wrong_identity_never_saves(self):
        for body in ("raise ValueError('helper rejected')", "return {'mode':'pluto-only','packages':[]}"):
            self.checker.write_text('def make_package_contract(mode,archives):\n '+body+'\n',encoding='utf-8')
            with self.assertRaises(ValueError):self.prepare()
            self.assertFalse((self.root/'contract.json').exists())
    def test_stage_requires_four_inputs_and_guard_third_argument_before_apply(self):
        source=(ROOT/'tools/stage.py').read_text(encoding='utf-8'); tree=ast.parse(source)
        args={n.args[0].value:n for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='add_argument'}
        for key in ('tconstruct-archive','pluto-archive','tconstruct-archive-sha256','pluto-archive-sha256'):
            self.assertTrue(next(k.value.value for k in args['--'+key].keywords if k.arg=='required'))
        self.assertLess(source.index('package_contract.prepare('),source.index("['cargo', 'build'"))
        sequence=[n for n in ast.walk(tree) if isinstance(n,ast.With) and any(isinstance(i.context_expr,ast.Call) and isinstance(i.context_expr.func,ast.Attribute) and i.context_expr.func.attr=='TemporaryDirectory' for i in n.items)]
        self.assertEqual(len(sequence),1);body=sequence[0].body
        calls=[n.value for n in body if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call)]
        order=[]
        for call in calls:
            if isinstance(call.func,ast.Attribute):
                if isinstance(call.func.value,ast.Name) and call.func.value.id=='package_contract':order.append(call.func.attr)
                elif call.func.attr=='run':
                    values=ast.unparse(call.args[0]); order.append('guard' if 'check_native_plan.py' in values else ('apply' if "'apply'" in values else 'plan'))
                    if 'check_native_plan.py' in values:self.assertEqual(ast.unparse(call.args[0].elts[-1]),'str(contract_path)')
        self.assertEqual(order,['verify','plan','verify','guard','verify','apply'])
    def test_launcher_passes_receipt_hashes_and_same_prepared_archive_paths(self):
        source=(ROOT/'tools/play_windows_latest.py').read_text(encoding='utf-8')
        for token in ("'--tconstruct-archive',tc_mods.parent/'tconstruct.hnmod'", "'--pluto-archive',tc_mods.parent/'plutolib.hnmod'", "'--tconstruct-archive-sha256',package_receipt['packages'][0]['archive_sha256']", "'--pluto-archive-sha256',package_receipt['packages'][1]['archive_sha256']"):
            self.assertEqual(source.count(token),1)
        self.assertIn('bundles.verify_prepared(tc_mods,bundle,package_receipt)',source)
    def test_production_changes_inverse_to_exact_c1b54_and_stage_pin(self):
        inv=json.loads((ROOT/'experiments/windows-stage-package-contract/inverse.json').read_text(encoding='utf-8'))
        for rel,meta in inv['files'].items():
            raw=before_jobs_one(rel.removeprefix('tconstruct/'),(ROOT.parent/rel).read_bytes());self.assertEqual(b.sha(raw),meta['after_sha256']);text=raw.decode('utf-8')
            for before,after in reversed(meta['edits']):self.assertEqual(text.count(after),1);text=text.replace(after,before,1)
            self.assertEqual(b.sha(text.encode('utf-8')),meta['before_sha256'])
        import play_windows_latest as launcher
        self.assertEqual(launcher.STAGE_SHA,b.sha((ROOT/'tools/stage.py').read_bytes()))


if __name__ == '__main__':unittest.main()
