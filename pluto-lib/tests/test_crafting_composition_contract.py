"""Synthetic guard regressions only, not an observed plan/packager/native proof."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('placement_contract',ROOT/'tools/check_recipe_placement_plan.py')
g=importlib.util.module_from_spec(spec);spec.loader.exec_module(g)

class ContractTests(unittest.TestCase):
    def setup_package(self, root, full=True):
        owners={g.MOD_ID:'0.1.0',g.TC:'0.5.0'} if full else {g.MOD_ID:'0.1.0'}
        p={'schema':6,'injections':[],'modules':[],'states':[],'crates':[],'bridges':[],'data':[]}
        c={'schema':1,'mode':'full-tconstruct' if full else 'standalone-pluto','horizon':g.HORIZON,'mixins':g.MIXINS,'packages':[]}
        for owner,version in owners.items():
            name='mixins/'+owner+'/horizon-mixin.toml'
            m=(f'[mod]\nid="{owner}"\nversion="{version}"\n[compatibility]\nhorizon_revision="{g.HORIZON}"\n'
               '[[module]]\nid="fixture"\nfile="native.rs"\nname="fixture"\nsource="fixture.rs"\n').encode()
            path=root/(owner+'.hnmod')
            with zipfile.ZipFile(path,'w') as z:
                z.writestr(name,m);z.writestr('horizon-native.json',json.dumps({'id':owner,'version':version}));z.writestr('mixins/'+owner+'/fixture.rs','// fixture')
            p['modules'].append({'mod_id':owner,'mod_version':version,'module_id':'fixture','source':'// fixture'})
            c['packages'].append({'mod_id':owner,'version':version,'archive':str(path),'archive_sha256':g.digest(path.read_bytes()),'manifest':name,'manifest_sha256':g.digest(m)})
        return p,c
    def test_explicit_modes_with_exact_synthetic_archives(self):
        with tempfile.TemporaryDirectory() as d:
            for full in [False,True]:
                sub=Path(d)/str(full);sub.mkdir();p,c=self.setup_package(sub,full)
                self.assertEqual(g.package_mode(p,c),c['mode'])
    def test_recorded_archive_contract_roundtrips_without_mode_inference(self):
        with tempfile.TemporaryDirectory() as d:
            p,c=self.setup_package(Path(d))
            archives={x['mod_id']:x['archive'] for x in c['packages']}
            made=g.make_package_contract('full-tconstruct',archives)
            self.assertEqual(made,c);self.assertEqual(g.package_mode(p,made),'full-tconstruct')
            for mode,files in [('auto',archives),('standalone-pluto',archives),('full-tconstruct',{g.MOD_ID:archives[g.MOD_ID]})]:
                with self.assertRaises(ValueError):g.make_package_contract(mode,files)
    def test_package_payload_cannot_differ_from_plan(self):
        with tempfile.TemporaryDirectory() as d:
            p,c=self.setup_package(Path(d));p['modules'][0]['source']='// changed'
            with self.assertRaisesRegex(ValueError,'body differs'):g.package_mode(p,c)
    def test_missing_and_unknown_contract_reject(self):
        for c in [None,{}, {'schema':2}, {'schema':1,'mode':'auto','horizon':g.HORIZON,'mixins':g.MIXINS,'packages':[]}]:
            with self.assertRaises(ValueError):g.package_mode({},c)
    def test_null_package_pins_never_mean_success(self):
        with tempfile.TemporaryDirectory() as d:
            p,c=self.setup_package(Path(d));c['packages'][0]['archive_sha256']=None
            with self.assertRaisesRegex(ValueError,'Unobserved'):g.package_mode(p,c)
    def test_full_declared_tc_cannot_disappear(self):
        with tempfile.TemporaryDirectory() as d:
            p,c=self.setup_package(Path(d));p['modules']=p['modules'][:1]
            with self.assertRaisesRegex(ValueError,'omitted'):g.package_mode(p,c)
    def test_standalone_never_admits_partial_tc(self):
        with tempfile.TemporaryDirectory() as d:
            p,c=self.setup_package(Path(d),False);p['modules'].append({'mod_id':g.TC})
            with self.assertRaisesRegex(ValueError,'owner'):g.package_mode(p,c)
    def test_duplicate_packages_and_mode_mismatch_reject(self):
        with tempfile.TemporaryDirectory() as d:
            p,c=self.setup_package(Path(d));c['packages'][1]=copy.deepcopy(c['packages'][0])
            with self.assertRaises(ValueError):g.package_mode(p,c)
    def test_undeclared_or_missing_manifest_identity_reject(self):
        with tempfile.TemporaryDirectory() as d:
            p,c=self.setup_package(Path(d));p['modules'][0]['module_id']='other'
            with self.assertRaises(ValueError):g.package_mode(p,c)
    def test_archive_and_manifest_digest_drift_reject(self):
        with tempfile.TemporaryDirectory() as d:
            p,c=self.setup_package(Path(d))
            for field in ['archive_sha256','manifest_sha256']:
                broken=copy.deepcopy(c);broken['packages'][0][field]='0'*64
                with self.subTest(field=field),self.assertRaises(ValueError):g.package_mode(p,broken)
    def test_unknown_plan_owner_or_version_reject(self):
        with tempfile.TemporaryDirectory() as d:
            p,c=self.setup_package(Path(d))
            for key,value in [('mod_id','other'),('mod_version','99')]:
                b=copy.deepcopy(p);b['modules'][0][key]=value
                with self.assertRaises(ValueError):g.package_mode(b,c)
    def test_unknown_extra_manifest_is_not_ignored(self):
        with tempfile.TemporaryDirectory() as d:
            p,c=self.setup_package(Path(d));item=c['packages'][0];a=Path(item['archive'])
            with zipfile.ZipFile(a,'a') as z:z.writestr('mixins/hidden/horizon-mixin.toml','[mod]\nid="hidden"')
            item['archive_sha256']=g.digest(a.read_bytes())
            with self.assertRaisesRegex(ValueError,'manifest set'):g.package_mode(p,c)
    def test_observed_ctor_ir_is_exact_and_missing_observations_still_reject(self):
        m=json.loads((ROOT/'tools/crafting-match-boundaries.json').read_bytes())['tconstruct_boundaries']
        expected={
            'empty':([42,0],'1e84c586bff87f44d88440bc32d0bb8f72a140325977f61a302e893dbcc4729c'),
            'from_sources':([42,4],'10979e222d48a73a212516bb5f05b977d4ee9fedf4a4c840a27eb2ec10b8dd7b'),
        }
        self.assertEqual(len(m),2)
        for row in m:
            self.assertEqual((row['node'],row['function_fingerprint']),expected[row['target'].split('::')[-1]])
            p={'schema':6,'injections':[dict(row,mod_id=g.TC)]}
            g.check_order(p,[row])
            for key in ('node','function_fingerprint'):
                missing=copy.deepcopy(row);missing[key]=None
                with self.subTest(target=row['target'],missing=key),self.assertRaisesRegex(ValueError,'Unobserved'):
                    g.check_order(p,[missing])
                drift=copy.deepcopy(p);drift['injections'][0][key]=[42,99] if key=='node' else '0'*64
                with self.subTest(target=row['target'],drift=key),self.assertRaises(ValueError):
                    g.check_order(drift,[row])
    def test_non_cancellable_only_exact_constructor(self):
        m={'target':next(iter(g.BOOTSTRAP)),'injection_id':next(iter(g.BOOTSTRAP.values())),
           'mod_id':g.TC,'file':'native.rs','kind':'HEAD','cancellable':False,'priority':100,'bindings':[],
           'node':[1,1],'function_fingerprint':'synthetic','contract':{'output':'()','value':None,'operation':None}}
        g.check_order({'schema':6,'injections':[dict(m)]},[m])
        for key,v in [('target','other'),('bindings',[{}]),('contract',{'output':'bool'}),('cancellable',True)]:
            x=dict(m);x[key]=v
            with self.assertRaises(ValueError):g.check_order({'schema':6,'injections':[dict(x)]},[x])
    def closure(self):
        module={'mod_id':g.MOD_ID,'mod_version':'0.1.0','module_id':'fixture','file':'native.rs','name':'fixture','visibility':'pub'}
        meta=dict(module,source_sha256=g.digest(b'// fixture'))
        s={'mod_id':g.TC,'state_id':'session','initial':'default'}
        lane={'target':'guest','helper_source':'// guest','mod_id':g.TC}
        api={'package':'pluto-inventory-api','files':{'src/lib.rs':'fixed'}}
        c={'schema':1,'exclusive_module_files':['native.rs'],'inventory_crate':api,'modes':{'full-tconstruct':{'native_files':{},'hook_targets':[],'modules':[meta],'states':[s],'retained_injections':[lane]}}}
        p={'modules':[dict(module,source='// fixture')],'states':[s],'bridges':[],'crates':[api],'injections':[lane]}
        return p,c
    def test_exact_synthetic_dependency_closure(self):
        p,c=self.closure();g.verify_closure(p,Path('/unused'),'full-tconstruct',c)
        # Official schema 6 omits visibility for the exact pub(crate) variant.
        c['modes']['full-tconstruct']['modules'][0]['visibility']=None
        del p['modules'][0]['visibility']
        g.verify_closure(p,Path('/unused'),'full-tconstruct',c)
        p['modules'][0]['visibility']='pub'
        with self.assertRaisesRegex(ValueError,'module identity'):
            g.verify_closure(p,Path('/unused'),'full-tconstruct',c)
    def test_changed_or_duplicate_dependency_module_reject(self):
        p,c=self.closure()
        for change in ['source','duplicate','owner','sibling']:
            b=copy.deepcopy(p)
            if change=='duplicate':b['modules']*=2
            elif change=='sibling':b['modules'].append(dict(b['modules'][0],name='new'))
            else:b['modules'][0]['source' if change=='source' else 'mod_id']='other'
            with self.subTest(change=change),self.assertRaises(ValueError):g.verify_closure(b,Path('/unused'),'full-tconstruct',c)
    def test_unknown_constructor_entry_hook_rejects(self):
        p,c=self.closure();p['injections'].append({'file':'native.rs','target':'Recipes::load','kind':'HEAD'})
        with self.assertRaisesRegex(ValueError,'entry hook'):g.verify_closure(p,Path('/unused'),'full-tconstruct',c)
    def test_api_registry_identity_and_full_fileset_required(self):
        p,c=self.closure()
        for change in ['duplicate','source']:
            b=copy.deepcopy(p)
            if change=='duplicate':b['crates']*=2
            else:b['crates'][0]['files']['extra']='changed'
            with self.assertRaises(ValueError):g.verify_closure(b,Path('/unused'),'full-tconstruct',c)
    def test_extra_protected_state_or_bridge_reject(self):
        p,c=self.closure()
        for kind in ['states','bridges']:
            b=copy.deepcopy(p);b[kind].append({'file':'native.rs'})
            with self.assertRaises(ValueError):g.verify_closure(b,Path('/unused'),'full-tconstruct',c)
    def test_session_and_guest_handler_do_not_silently_change(self):
        p,c=self.closure()
        for section in ['states','injections']:
            b=copy.deepcopy(p);b[section][0]['unexpected']='change'
            with self.assertRaises(ValueError):g.verify_closure(b,Path('/unused'),'full-tconstruct',c)
    def test_placement_definition_is_retained(self):
        m=g.entries();self.assertEqual(len(m),2)
        self.assertEqual(m[0]['target'],'horizon_client_lib::crafting::Recipes::placement')
        self.assertEqual(m[1]['bindings'][-1],{'name':'use_max','ty':'bool','value':'use_max'})
    def test_legacy_station_transaction_body_is_exact_suffix(self):
        repo=ROOT.parent;old=(repo/'tconstruct/mixins/tconstruct/source-crafting-recipe-guard.rs').read_text()
        new=(repo/'tconstruct/mixins/tconstruct/source-station-transaction.rs').read_text()
        self.assertTrue(new.endswith(old));self.assertEqual(new.count(old),1)
    def test_bootstrap_policies_keep_legacy_whole_overslime_sources(self):
        repo=ROOT.parent;before=json.loads((ROOT/'experiments/crafting-ordinary-intake/before.json').read_text())
        for name in ['source-overslime.rs','source-overslime-crafting.rs','source-overslime-craft-assemble.rs','source-overslime-craft-remainders.rs']:
            p='tconstruct/mixins/tconstruct/'+name
            self.assertEqual((repo/p).read_text(),before[p])
        source=(repo/'tconstruct/mixins/tconstruct/source-crafting-bootstrap.rs').read_text()
        self.assertLess(source.index('stored().map_err'),source.index('pair().map_err'))
        self.assertIn('get_or_init',source)

if __name__=='__main__':unittest.main()
