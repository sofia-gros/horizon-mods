import copy
import importlib.util
import json
from pathlib import Path
import re
import tomllib
import unittest

ROOT=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('ordinary_compose',ROOT/'compose.py')
C=importlib.util.module_from_spec(spec);spec.loader.exec_module(C)
I=json.loads((ROOT/'inputs.json').read_text())
B=json.loads((ROOT/'before.json').read_text())
P=json.loads((ROOT/'proposals.json').read_text())
BOOT=(ROOT/'combined-bootstrap.rs').read_text()

class Tests(unittest.TestCase):
    def test_exact_sixteen_path_postimage_and_inverse_receipt(self):
        a,r=C.compose(I,B,P,BOOT)
        self.assertEqual(len(a),16);self.assertEqual(r,json.loads((ROOT/'receipt.json').read_text()))
        self.assertTrue(all(r['inverse_checks'].values()))
        self.assertFalse(r['ordinary_installed']);self.assertFalse(r['native_executed'])

    def test_only_five_delegated_hooks_are_removed(self):
        a,_=C.compose(I,B,P,BOOT)
        old=tomllib.loads(B[C.T+'horizon-mixin.toml']);new=tomllib.loads(a[C.T+'horizon-mixin.toml'])
        oldids={h['id'] for h in old['injection']};newids={h['id'] for h in new['injection']}
        self.assertEqual(oldids-newids,set(C.REMOVED))
        self.assertEqual(newids-oldids,{'crafting-shared-empty-startup','crafting-shared-sources-startup'})
        self.assertEqual([h for h in new['injection'] if h['id'] in oldids], [h for h in old['injection'] if h['id'] not in C.REMOVED])

    def test_old_modules_states_and_prior_adoptions_remain_exact(self):
        a,_=C.compose(I,B,P,BOOT)
        for prefix,added in [(C.T,4),(C.P,2)]:
            old=tomllib.loads(B[prefix+'horizon-mixin.toml']);new=tomllib.loads(a[prefix+'horizon-mixin.toml'])
            self.assertEqual(new['module'][:-added],old['module'])
            self.assertEqual({k:v for k,v in new.items() if k not in ['module','injection']},{k:v for k,v in old.items() if k not in ['module','injection']})

    def test_pluto_only_click_formals_change_and_remainders_are_added(self):
        a,_=C.compose(I,B,P,BOOT)
        old=tomllib.loads(B[C.P+'horizon-mixin.toml'])['injection'];new=tomllib.loads(a[C.P+'horizon-mixin.toml'])['injection']
        self.assertEqual(len(new),len(old)+1)
        for x,y in zip(old,new):
            if x['id']=='recipe-placement-click':
                self.assertEqual(y,{**x,'bindings':x['bindings']+[{'name':'use_max','ty':'bool','value':'use_max'}]})
            else:self.assertEqual(x,y)
        self.assertEqual(new[-1]['target'],'horizon_client_lib::crafting::Recipes::remainders')

    def test_ctor_owner_is_actual_library_and_both_hooks_are_non_cancel(self):
        a,_=C.compose(I,B,P,BOOT);hooks=tomllib.loads(a[C.T+'horizon-mixin.toml'])['injection'][-2:]
        self.assertEqual([h['target'] for h in hooks],['horizon_client_lib::crafting::Recipes::empty','horizon_client_lib::crafting::Recipes::from_sources'])
        for h in hooks:self.assertEqual((h['kind'],h['cancellable'],h['bindings']),('HEAD',False,[]))

    def test_combined_bootstrap_is_one_cached_result_and_no_rollback(self):
        self.assertEqual(BOOT.count('static REGISTERED:'),1)
        init=BOOT.split('fn initialize(',1)[1].split('fn require_registration(',1)[0]
        self.assertLess(init.index('stored()'),init.index('pair()'))
        self.assertIn('stored().map_err(InitializationError::Stored)?;',init)
        self.assertIn('state.get_or_init(|| initialize(stored, pair))',BOOT)
        self.assertIn('panic!("TConstruct crafting initialization failed:',BOOT)
        self.assertNotIn('unregister',BOOT)
        self.assertEqual(len(re.findall(r'#\[test\]\s*fn ',BOOT)),5)

    def test_ctor_helpers_call_same_combined_initializer(self):
        a,_=C.compose(I,B,P,BOOT)
        for suffix in ['empty','sources']:
            helper=a[C.T+'source-crafting-'+suffix+'-startup.rs']
            self.assertEqual(helper.count('tconstruct_crafting_bootstrap::ensure()'),1)
            self.assertNotIn('Recipes::',helper)

    def test_stored_api_and_native_policy_are_exact_successful_candidates(self):
        a,_=C.compose(I,B,P,BOOT)
        for path,key in [(C.P+'inventory-api/src/crafting/matching.rs','stored/matching.rs'),(C.P+'crafting-match.rs','stored/native-adapter.rs'),(C.T+'source-crafting-deny-policies.rs','stored/consumer.rs')]:
            self.assertEqual(a[path],P[key])
        self.assertEqual(I['api153_finite16_run'],42247376045)

    def test_pair_keeps_native_value_provider_and_separate_requests(self):
        a,_=C.compose(I,B,P,BOOT)
        for path,key in [(C.P+'crafting-derivations.rs','pair/native-derivation.rs'),(C.P+'crafting-match-veto.rs','pair/matching-head.rs'),(C.P+'crafting-pure-remainders.rs','pair/remainders-head.rs'),(C.T+'source-overslime-derivation.rs','pair/consumer.rs')]:
            self.assertEqual(a[path],P[key])
        self.assertIn('tconstruct_source_overslime_crafting::plan(grid)',a[C.T+'source-overslime-derivation.rs'])

    def test_legacy_bodies_are_not_deleted_or_reconstructed(self):
        a,r=C.compose(I,B,P,BOOT)
        for n in ['anvil-material-recipe.rs','survival-melter-recipe.rs','source-overslime-craft-assemble.rs','source-overslime-craft-remainders.rs','source-overslime.rs','source-overslime-crafting.rs','source-crafting-recipe-guard.rs']:
            self.assertNotIn(C.T+n,a);self.assertEqual(r['preserved_legacy_files'][C.T+n],C.sha(B[C.T+n]))
        self.assertEqual(a[C.T+'source-station-transaction.rs'][len(P['station/consumer-prefix.rs']):],B[C.T+'source-crafting-recipe-guard.rs'])

    def test_station_change_is_one_registration_line_and_exact_dispatcher(self):
        a,_=C.compose(I,B,P,BOOT);s=a[C.T+'source-crafting-station.rs']
        self.assertEqual(s.count(C.INSERT),1)
        self.assertEqual(s.replace(C.INSERT,''),B[C.T+'source-crafting-station.rs'])
        self.assertEqual(a[C.P+'recipe-transactions.rs'],P['station/native-transaction.rs'])
        self.assertEqual(a[C.P+'recipe-placement-click.rs'],P['station/single-head.rs'])

    def test_all_three_result_targets_have_one_owner(self):
        a,_=C.compose(I,B,P,BOOT);hooks=[]
        for prefix in [C.T,C.P]:hooks.extend(tomllib.loads(a[prefix+'horizon-mixin.toml'])['injection'])
        for target,id in [('horizon_client_lib::crafting::Recipes::assemble_if_matches','crafting-match-veto'),('horizon_client_lib::crafting::Recipes::remainders','crafting-pure-remainders'),("horizon_client::game::recipe_book::Game<'_>::place_recipe",'recipe-placement-click')]:
            selected=[h for h in hooks if h['target']==target]
            self.assertEqual(len(selected),1);self.assertEqual(selected[0]['id'],id)

    def test_unknown_or_changed_sources_and_cutoff_fail_closed(self):
        for key in ['base','ordinary_installed','native_executed','removed_hook_ids']:
            i=copy.deepcopy(I);i[key]={'base':'0'*40,'ordinary_installed':True,'native_executed':True,'removed_hook_ids':[]}[key]
            with self.assertRaises(ValueError):C.compose(i,B,P,BOOT)
        for which in ['before','proposals','bootstrap']:
            b=copy.deepcopy(B);p=copy.deepcopy(P);boot=BOOT
            if which=='before':b[next(iter(b))]+='\n'
            elif which=='proposals':p['unknown']=''
            else:boot+='\n'
            with self.assertRaises(ValueError):C.compose(I,b,p,boot)

if __name__=='__main__':unittest.main()
