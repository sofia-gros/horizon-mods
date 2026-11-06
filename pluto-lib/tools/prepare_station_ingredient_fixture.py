#!/usr/bin/env python3
"""Exact native Ingredient parsing and station boundary; modeled HEAD/provider hooks."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import prepare_recipe_placement_fixture as base
from prepare_recipe_preview_fixture import fragment
import prepare_advancement_projection_fixture as common
ROOT=Path(__file__).resolve().parents[1]
NAME='pluto-station-ingredient-tests'

def verify(output):
    return base.verify(output,expected_name=NAME)

def prepare(host,output):
    base.prepare(host,output)
    host=host.resolve();native=ROOT/'mixins/pluto_native_creative'
    evidence=json.loads((output/'fixture-sources.json').read_text());records=evidence['fragments']
    def take(file,start,end,destination,rename=None):
        text=fragment((host/file).read_text(),start,end)
        row=dict(source=file,start=start,end=end,destination=destination,sha256=common.digest(text.encode()))
        if rename:row.update(rename_from=rename[0],rename_to=rename[1]);text=text.replace(*rename,1)
        records.append(row);return text
    crafting='crates/horizon-client/src/crafting.rs';stations='crates/horizon-client/src/stations.rs'
    extra='\nuse horizon_items::items::Items;\n'
    extra+=take(crafting,'fn dedup_keep_order(items: &mut Vec<String>) {','/// A set of accepted items','crafting.rs')
    extra+=take(crafting,'impl Ingredient {','/// `PlacementInfo`:','crafting.rs')
    extra+='''
// Explicit modeled unrelated provider endpoints; not external-provider proof.
pub fn native_ingredient(_kind:&str,_data:&serde_json::Map<String,Value>,_items:&mut Vec<String>){}
pub fn native_ingredient_test(_kind:&str,_data:&serde_json::Map<String,Value>,_stack:&ItemStack,_accept:&mut bool){}
'''
    with (output/'crafting.rs').open('a') as f:f.write(extra)
    source='use crate::crafting::Ingredient;\nuse horizon_items::items::Items;\nuse serde_json::Value;\n'
    source+=take(stations,'fn ingredient(v: &Value, items: &Items) -> Ingredient {','impl StationRecipes {','stations.rs',('fn ingredient(','pub fn native_ingredient('))
    source+='''
pub fn ingredient(value:&Value,items:&Items)->Ingredient{match pluto_station_ingredient(value,items){std::ops::ControlFlow::Break(result)=>result,std::ops::ControlFlow::Continue(())=>native_ingredient(value,items)}}
include!("station-head.rs");
#[path="station-adapter.rs"] pub mod pluto_ingredient;
'''
    (output/'stations.rs').write_text(source)
    for src,dst in [('station-ingredient.rs','station-adapter.rs'),('station-ingredient-head.rs','station-head.rs')]:shutil.copyfile(native/src,output/dst)
    shutil.copyfile(ROOT/'tests/native/station_ingredient.rs',output/'lib.rs')
    policy=ROOT.parent/'tconstruct/tests/rust/station-ingredient-policy'
    for src,dst in [('callback.rs','station-callback.rs'),('reference.rs','station-reference.rs')]:shutil.copyfile(policy/src,output/dst)
    original=subprocess.check_output(['git','-C',str(ROOT.parent),'show',base.REFERENCE+':tconstruct/mixins/tconstruct/source-stonecutting-ingredient.rs'],text=True)
    if original not in (output/'station-reference.rs').read_text():raise ValueError('Exact consumer parser reference changed')
    (output/'Cargo.toml').write_text((output/'Cargo.toml').read_text().replace('name="pluto-recipe-placement-tests"','name="'+NAME+'"'))
    evidence['official_sha256'][stations]=common.digest((host/stations).read_bytes())
    evidence['consumer_station_parser_sha256']=common.digest(original.encode())
    evidence['files_sha256']={str(p.relative_to(output)):common.digest(p.read_bytes()) for p in output.rglob('*') if p.is_file() and p.name not in ('Cargo.lock','fixture-sources.json')}
    evidence['scope']='Exact native Ingredient parse/of/test and station ingredient body with real Items; HEAD wrapper and unrelated native custom-ingredient endpoints modeled. Not full StationRecipes loading, gameplay or external provider proof.'
    common.metadata(output,False);(output/'fixture-sources.json').write_text(json.dumps(evidence,indent=2)+'\n');verify(output);print(output)
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--horizon-source',type=Path);parser.add_argument('--output',type=Path,required=True);parser.add_argument('--verify-lock',action='store_true');args=parser.parse_args()
    if args.verify_lock:verify(args.output);print('Station parser source/lock identities verified')
    elif args.horizon_source is None:parser.error('--horizon-source required')
    else:prepare(args.horizon_source,args.output)
