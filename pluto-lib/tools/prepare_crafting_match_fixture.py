#!/usr/bin/env python3
"""Extend the exact recipe-body fixture for read-only crafting-match admission.

Native shaped/shapeless bodies and Ingredient.test are real. Special endpoints,
external native ingredient providers and HEAD wrappers are explicitly modeled.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import prepare_recipe_placement_fixture as base
from prepare_recipe_preview_fixture import fragment
import prepare_advancement_projection_fixture as common
ROOT=Path(__file__).resolve().parents[1]
NAME='pluto-crafting-match-tests'

def verify(output):
    evidence=base.verify(output,expected_name=NAME)
    host=Path(evidence['official_source'])
    source=(host/'crates/horizon-client/src/crafting.rs').read_text()
    find=fragment(source,'    pub fn find_with(\n','    /// `Recipe.matches` then `assemble`')
    match=fragment(source,'    pub fn recipe_matches(&self, id: &str, grid: &[Option<ItemStack>], size: usize) -> bool {','    /// `Recipe.placementInfo`')
    if 'Self::assemble_if_matches(&ctx, &input, recipe)' not in find or 'Self::assemble_if_matches(&ctx, &input, recipe)' not in match:raise ValueError('Native find/known-recipe match bypassed the reviewed boundary')
    return evidence

def prepare(host,output):
    base.prepare(host,output)
    host=host.resolve();native=ROOT/'mixins/pluto_native_creative'
    evidence=json.loads((output/'fixture-sources.json').read_text());records=evidence['fragments']
    def take(file,start,end,dest,rename=None):
        text=fragment((host/file).read_text(),start,end);row=dict(source=file,start=start,end=end,destination=dest,sha256=common.digest(text.encode()))
        if rename:row.update(rename_from=rename[0],rename_to=rename[1]);text=text.replace(*rename,1)
        records.append(row);return text
    crafting='crates/horizon-client/src/crafting.rs'
    extra='\nimpl Ingredient {\n'+take(crafting,'    pub fn test(&self, stack: &ItemStack) -> bool {','/// `PlacementInfo`','crafting.rs')
    extra+='\nimpl Template {\n'+take(crafting,'    pub fn create(&self) -> ItemStack {','    /// `TransmuteRecipe.createWithOriginalComponents`','crafting.rs')+'}\n'
    extra+=take(crafting,"pub struct Input<'a> {",'/// What recipes need to know about an item.','crafting.rs')
    extra+=take(crafting,'fn matches_all(ingredients: &[Ingredient], stacks: &[&ItemStack]) -> bool {','/// A resource id with its default namespace','crafting.rs')
    extra+=take(crafting,"pub struct Context<'a> {","impl Context<'_> {","crafting.rs")
    # These endpoints are intentionally modeled, not native special-recipe proof.
    extra+='''
pub static SPECIAL_CALLS:std::sync::atomic::AtomicUsize=std::sync::atomic::AtomicUsize::new(0);
impl Special {
 fn matches(&self,ctx:&Context,_:&Input)->bool{let _=ctx;SPECIAL_CALLS.fetch_add(1,std::sync::atomic::Ordering::SeqCst);false}
 fn assemble(&self,_:&Context,_:&Input)->Option<ItemStack>{None}
}
pub fn native_ingredient_test(_kind:&str,_data:&serde_json::Map<String,Value>,_stack:&ItemStack,_accept:&mut bool){}
impl Recipes {
'''
    extra+=take(crafting,'    pub fn find_with(\n','    /// `Recipe.matches` then `assemble`','crafting.rs')
    extra+=take(crafting,'    pub fn recipe_matches(&self, id: &str, grid: &[Option<ItemStack>], size: usize) -> bool {','    /// `Recipe.placementInfo`','crafting.rs')
    extra+=take(crafting,'    fn assemble_if_matches(\n','    /// `recipe.matches(craftSlots.asCraftInput())`','crafting.rs',('fn assemble_if_matches(','pub fn native_assemble_if_matches('))
    extra+='''
pub fn assemble_if_matches(ctx:&Context,input:&Input,recipe:&Recipe)->Option<Option<ItemStack>>{match pluto_crafting_match_veto(input,recipe){std::ops::ControlFlow::Break(value)=>value,std::ops::ControlFlow::Continue(())=>Self::native_assemble_if_matches(ctx,input,recipe)}}
}
include!("matching-head.rs");
#[path="matching.rs"] pub mod pluto_matching;
'''
    with (output/'crafting.rs').open('a') as f:f.write(extra)
    for src,dst in [('crafting-match.rs','matching.rs'),('crafting-match-veto.rs','matching-head.rs'),('component-view.rs','component-view.rs')]:shutil.copyfile(native/src,output/dst)
    shutil.copyfile(ROOT/'tests/native/crafting_match.rs',output/'lib.rs')
    original=ROOT.parent/'tconstruct/tests/rust/crafting-match-policy'
    for src,dst in [('callback.rs','matching-callback.rs'),('reference_gate.rs','matching-reference-gate.rs'),('reference_plan.rs','matching-reference-plan.rs')]:shutil.copyfile(original/src,output/dst)
    for source,dest in [('fake-material-recipe.rs','matching-reference-gate.rs'),('source-fake-material-crafting.rs','matching-reference-plan.rs')]:
        exact=subprocess.check_output(['git','-C',str(ROOT.parent),'show',base.REFERENCE+':tconstruct/mixins/tconstruct/'+source],text=True)
        if exact not in (output/dest).read_text():raise ValueError('Exact consumer reference changed')
    adv='crates/horizon-client/src/advancements.rs'
    item=take(adv,'#[derive(Clone, Debug, Default)]\npub struct ItemView','/// An entity as predicates see it.','item-view.rs')
    (output/'item-view.rs').write_text(item)
    manifest=(output/'Cargo.toml').read_text().replace('name="pluto-recipe-placement-tests"','name="'+NAME+'"')
    (output/'Cargo.toml').write_text(manifest)
    evidence['official_sha256'][adv]=common.digest((host/adv).read_bytes())
    evidence['files_sha256']={str(p.relative_to(output)):common.digest(p.read_bytes()) for p in output.rglob('*') if p.is_file() and p.name not in ('Cargo.lock','fixture-sources.json')}
    evidence['scope']='Exact native shaped/shapeless matching, cropped Input, Ingredient.test, Template.create and component-view; special endpoints, external ingredient providers and HEAD wrappers modeled. Not gameplay/consumption/complete private identity proof.'
    common.metadata(output,False);(output/'fixture-sources.json').write_text(json.dumps(evidence,indent=2)+'\n');verify(output);print(output)
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--horizon-source',type=Path);parser.add_argument('--output',type=Path,required=True);parser.add_argument('--verify-lock',action='store_true');args=parser.parse_args()
    if args.verify_lock:verify(args.output);print('Exact crafting sources, consumer references and dependency identities verified')
    elif args.horizon_source is None:parser.error('--horizon-source required')
    else:prepare(args.horizon_source,args.output)
