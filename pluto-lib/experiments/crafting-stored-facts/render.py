"""Pure finite-policy fixture composer. No subprocess, metadata, build or writes."""
import hashlib
import re

BASE = 'e128d61478971e247b1e3e2e09516cf99652f9fc'
HORIZON = '41f822b659789780f58d32c63ddfb6887ea1b8bf'
N = 'pluto-lib/mixins/pluto_native_creative/'
T = 'tconstruct/mixins/tconstruct/'

def digest(s):
    return hashlib.sha256(s.encode()).hexdigest()

def between(s, a, b):
    if s.count(a) != 1 or s.count(b) != 1:
        raise ValueError('non-unique source boundaries')
    start=s.index(a); end=s.index(b,start)
    return s[start:end]

def validate(inputs, oracles, candidates):
    if inputs['base'] != BASE or inputs['horizon'] != HORIZON:
        raise ValueError('wrong source cutoff')
    if any(inputs[k] for k in ('runtime_installed','native_executed','old_hooks_removed')):
        raise ValueError('unproven activation/execution')
    if set(oracles) != set(inputs['oracle_sha256']) or set(candidates) != set(inputs['candidate_sha256']):
        raise ValueError('unexpected source set')
    for values, hashes in [(oracles,inputs['oracle_sha256']),(candidates,inputs['candidate_sha256'])]:
        for k,v in values.items():
            if digest(v) != hashes[k]: raise ValueError('source drift: '+k)
    original_tests=oracles[N+'inventory-api/src/crafting/matching.rs'].split('#[cfg(test)]\n',1)[1]
    if original_tests not in candidates['matching.rs']:
        raise ValueError('legacy test body changed')
    if candidates['matching.rs'].count('static POLICIES:') != 1:
        raise ValueError('registry ownership changed')

def verify_native(inputs, oracles, crafting, advancements):
    if digest(crafting) != inputs['crafting_whole_sha256'] or digest(advancements) != inputs['advancements_whole_sha256']:
        raise ValueError('native whole source drift')
    for name,(start,end) in inputs['native_section_bounds'].items():
        if between(crafting,start,end) != oracles['official-crafting/'+name]:
            raise ValueError('native fragment drift: '+name)
    view=oracles['official-item-view-fragment']
    if advancements.count(view) != 1: raise ValueError('native ItemView drift')

def compose(inputs, oracles, candidates):
    validate(inputs,oracles,candidates)
    crafting='''use crate::{items::ItemStack, item_data::StackData};
use horizon_items::firework::FireworkShape;
use horizon_world::{nbt::{Tag, JavaString}, component_patch::Patch};
use serde_json::Value;
use std::{collections::HashSet, sync::Arc};
'''
    crafting+=oracles['official-crafting/ingredient-declarations']
    crafting+='impl Ingredient {\n'+oracles['official-crafting/ingredient-of']+'}\n'
    for name in ('complete-template','recipe-declarations','complete-input'):
        crafting+=oracles['official-crafting/'+name]
    placement=oracles[N+'recipe-placement.rs']
    bridge=between(placement,'impl Ingredient for NativeIngredient {','/// Mirrors native lookup order')
    crafting+='mod ingredient_bridge {\nuse super::{Ingredient as NativeIngredient,Shape};\nuse pluto_inventory_api::recipe_book::placement::{Ingredient,IngredientShape};\n'+bridge+'}\n'
    crafting+='pub mod pluto_matching { include!("native-adapter.rs"); }\n'
    crafting+='mod candidate_policy { include!("consumer.rs"); }\n'
    crafting+='mod tconstruct_survival_empty_tank { include!("empty-tank.rs"); }\n'
    for name,filename,helper in [('old_anvil','anvil-material-recipe.rs','tconstruct_anvil_material_recipe'),('old_melter','survival-melter-recipe.rs','tconstruct_survival_melter_recipe')]:
        crafting+='mod '+name+' {\nuse super::{Input,Recipe,ItemStack,tconstruct_survival_empty_tank};\n'
        crafting+=oracles[T+filename]
        crafting+='\npub(super) fn denies(input:&Input<\'_>, recipe:&Recipe)->bool { matches!('+helper+'(input,recipe),std::ops::ControlFlow::Break(_)) }\n}\n'
    crafting+='#[cfg(test)] mod differential_tests { include!("differential_tests.rs"); }\n'
    storage=oracles[N+'stack-storage.rs']; prefix=storage[:storage.index('/// Stage the native NBT representation')]
    lib='''pub use horizon_items::{items,item_data};
pub use pluto_inventory_api as pluto_inventory;
pub mod crafting;
pub mod pluto_component_view { include!("component-view.rs"); }
pub mod pluto_stack_storage { include!("retained-wire-query.rs"); }
pub mod advancements { include!("item-view.rs"); }
'''
    files={'lib.rs':lib,'crafting.rs':crafting,'item-view.rs':oracles['official-item-view-fragment'],
        'component-view.rs':oracles[N+'component-view.rs'],'retained-wire-query.rs':prefix,
        'empty-tank.rs':oracles[T+'survival-empty-tank.rs'],
        **{n:candidates[n] for n in ('native-adapter.rs','consumer.rs','differential_tests.rs')}}
    # One actual API path dependency must use candidate matching.rs; never include
    # that crate as a second source module in this harness.
    names={
        'differential12':['crafting::differential_tests::'+n for n in re.findall(r'#\[test\]\s*fn ([a-zA-Z0-9_]+)',candidates['differential_tests.rs'])],
        'original4':['pluto_component_view::tests::'+n for n in re.findall(r'#\[test\]\s*fn ([a-zA-Z0-9_]+)',oracles[N+'component-view.rs'])]+['crafting::tconstruct_survival_empty_tank::tests::'+n for n in re.findall(r'#\[test\]\s*fn ([a-zA-Z0-9_]+)',oracles[T+'survival-empty-tank.rs'])],
    }
    if [len(names[k]) for k in ('differential12','original4')] != [12,4] or len(set(sum(names.values(),[])))!=16:
        raise ValueError('unexpected finite test inventory')
    return files,{'status':'source_composed_only','files':{n:{'bytes':len(s.encode()),'sha256':digest(s)} for n,s in files.items()},'expected_tests':names,'rust_executed':False,'actual_game_proved':False,'startup_proved':False}
