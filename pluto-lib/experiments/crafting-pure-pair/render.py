"""Pure source composition only; no subprocess, native execution or output writes."""
import hashlib,re
BASE='17f44dc36fc2df28547328aa8554286ae965b208'
OFFICIAL='41f822b659789780f58d32c63ddfb6887ea1b8bf'
def sha(s):return hashlib.sha256(s.encode()).hexdigest()
def function(source,name):
    needle='pub(super) fn '+name+'('
    if source.count(needle)!=1:raise ValueError('Function boundary not unique')
    start=source.index(needle);body=source.index('{',start);depth=0
    for end in range(body,len(source)):
        if source[end]=='{':depth+=1
        elif source[end]=='}':
            depth-=1
            if depth==0:return source[start:end+1]+'\n'
    raise ValueError('Unclosed function')
def validate(i,o,c):
    if i['base']!=BASE or i['official']!=OFFICIAL:raise ValueError('Wrong cutoff')
    if any(i[k] for k in ['actual_ordinary_activation','native_executed','old_hook_removed']):raise ValueError('Unproved activation')
    for values,hashes in [(o,i['oracle_sha256']),(c,i['candidate_sha256'])]:
        if set(values)!=set(hashes) or any(sha(v)!=hashes[n] for n,v in values.items()):raise ValueError('Source bytes/set drift')
    replacement='crate::crafting::pluto_derivations::after_matching(\n        crate::crafting::pluto_matching::evaluate(input, recipe),\n        input,\n        recipe,\n    )'
    if replacement not in c['matching-head.rs']:raise ValueError('Veto outcome is not prior to derivation')
def verify_native(i,o,crafting):
    if sha(crafting)!=i['official_crafting_sha256']:raise ValueError('Native whole source drift')
    for n,v in o.items():
        if n.startswith('official/') and crafting.count(v)!=1:raise ValueError('Native source fragment drift')
def compose(i,o,c):
    validate(i,o,c)
    imports='''use crate::{items::ItemStack,item_data::StackData};
use horizon_items::firework::FireworkShape;
use horizon_world::{nbt::{Tag,JavaString},component_patch::Patch};
use serde_json::Value;
use std::{collections::HashSet,sync::Arc};
'''
    crafting=imports+o['official/ingredient-declarations']+'impl Ingredient {\n'+o['official/ingredient-of']+'}\n'
    for n in ['complete-template','recipe-declarations','complete-input']:crafting+=o['official/'+n]
    crafting+='pub mod pluto_derivations { include!("native-derivation.rs"); #[cfg(test)] mod tests { include!("regression_tests.rs"); } }\n'
    crafting+='mod pair_consumer { include!("consumer.rs"); }\n'
    for m,f,call,params,args,ret in [
        ('legacy_assembly','old-assembly.rs','tconstruct_overslime_craft_assemble',"input:&Input<'_>,recipe:&Recipe",'input,recipe','Option<Option<ItemStack>>'),
        ('legacy_remainders','old-remainders.rs','tconstruct_overslime_craft_remainders','recipe:&str,grid:&[Option<ItemStack>],size:usize','recipe,grid,size','Vec<Option<ItemStack>>')]:
        crafting+='mod '+m+' {\nuse super::{Input,Recipe,ItemStack};\n'+o[f]+'\npub(super) fn call('+params+')->std::ops::ControlFlow<'+ret+'> {'+call+'('+args+')}\n}\n'
    lib='''pub use horizon_items::{items,item_data};
pub mod crafting;
pub mod tconstruct_source_overslime { include!("overslime-arithmetic.rs"); }
pub mod tconstruct_source_overslime_crafting { include!("overslime-crafting.rs"); }
'''
    files={'lib.rs':lib,'crafting.rs':crafting,'overslime-arithmetic.rs':function(o['overslime.rs'],'amount')+'\n'+function(o['overslime.rs'],'refill'),'overslime-crafting.rs':o['overslime-crafting.rs'],**{n:c[n] for n in ['native-derivation.rs','consumer.rs','regression_tests.rs']}}
    names=['crafting::pluto_derivations::tests::'+n for n in re.findall(r'#\[test\]\s*fn (\w+)',c['regression_tests.rs'])]
    if len(names)!=16 or len(set(names))!=16:raise ValueError('Finite inventory differs')
    return files,{'files':{n:{'sha256':sha(v),'bytes':len(v.encode())} for n,v in files.items()},'tests':names,'rust_executed':False,'actual_installed_head':False,'same_invocation_pair_guarantee':False}
