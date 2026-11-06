use super::*;
use super::super::{Input, Recipe, Special, Template, legacy_assembly, legacy_remainders, pair_consumer};
use horizon_items::item_data::{BundleSelected, Wire};
use horizon_world::nbt::{JavaString, Tag};
use pluto_inventory_api::crafting::matching::Outcome;
use std::sync::{Arc, atomic::{AtomicUsize,Ordering}};
const ID: &str = "tconstruct:overslime_bottle_recipe";
const CUSTOM: &str = "minecraft:custom_data";
fn tool(current:i32, cap:i32, active:bool, overworked:i32)->ItemStack {
    let mut s=ItemStack::new("tconstruct:pickaxe",1);
    s.data_mut().other.push((CUSTOM.into(),Tag::compound([
        ("tic_overslime",Tag::Int(current)),("tic_overslime_capacity",Tag::Int(cap)),
        ("tic_overslime_active",Tag::Byte(i8::from(active))),("tic_overslime_overworked",Tag::Int(overworked)),
        ("foreign",Tag::String(JavaString(vec![0xd800,0x61]))),
    ])));s
}
fn bottle(kind:&str,count:u32)->ItemStack { ItemStack::new(&format!("tconstruct:{kind}_slime_bottle"),count) }
fn grid(tool:ItemStack,bottles:Vec<ItemStack>)->Vec<Option<ItemStack>> {
    let mut g=vec![Some(tool)];g.extend(bottles.into_iter().map(Some));g.resize(9,None);g
}
fn recipe()->Recipe { Recipe::Shapeless { ingredients:vec![],result:Template{item:Some(ID.into()),count:1,damage:0,extra:None,components:None} } }
fn equal_stack(a:&ItemStack,b:&ItemStack) {
    assert_eq!(a,b);
    if let (Some(a),Some(b))=(a.data(),b.data()) {
        assert_eq!(a.bundle_selected.0,b.bundle_selected.0);
        assert_eq!(a.wire.patch(),b.wire.patch());
        assert_eq!(a.map_post_processing,b.map_post_processing);
        for (a,b) in a.container.iter().zip(&b.container) {assert_eq!(a.0,b.0);equal_stack(&a.1,&b.1);}
        for (a,b) in a.bundle_contents.iter().zip(&b.bundle_contents) {equal_stack(a,b);}
    }
}
fn equal_cells(a:&[Option<ItemStack>],b:&[Option<ItemStack>]) {
    assert_eq!(a.len(),b.len());
    for (a,b) in a.iter().zip(b) {match (a,b) {(Some(a),Some(b))=>equal_stack(a,b),(None,None)=>{},_=>panic!("cell presence")}}
}
fn compare(g:&[Option<ItemStack>],size:usize,r:&Recipe,id:&str)->(ControlFlow<Option<Option<ItemStack>>>,ControlFlow<Vec<Option<ItemStack>>>) {
    let before=g.to_vec();let input=Input::of(g,size).unwrap();let registry=Registry::default();registry.insert(pair_consumer::entry()).unwrap();
    let old=legacy_assembly::call(&input,r);let new=after_matching_from(&registry,Outcome::Native,&input,r);
    assert_eq!(old,new);
    if let (ControlFlow::Break(Some(Some(a))),ControlFlow::Break(Some(Some(b))))=(&old,&new) {equal_stack(a,b);}
    let old_rest=legacy_remainders::call(id,g,size);let new_rest=remainders_from(&registry,id,g,size);
    assert_eq!(old_rest,new_rest);
    if let (ControlFlow::Break(a),ControlFlow::Break(b))=(&old_rest,&new_rest) {equal_cells(a,b);}
    equal_cells(g,&before);(new,new_rest)
}
fn output(pair:&(ControlFlow<Option<Option<ItemStack>>>,ControlFlow<Vec<Option<ItemStack>>>))->&ItemStack {
    let ControlFlow::Break(Some(Some(s)))=&pair.0 else {panic!("missing derived output")};s
}
#[test]
fn four_bottle_types_keep_native_outputs_and_original_remainder_order() {
    for kind in ["earth","sky","ichor","ender"] {for current in [0,45,49] {
        compare(&grid(tool(current,50,true,0),vec![bottle(kind,64),bottle(kind,1)]),3,&recipe(),ID);
    }}
}
#[test]
fn holes_offsets_and_compression_change_only_remainder_coordinates() {
    let compact=grid(tool(45,50,true,0),vec![bottle("earth",2),bottle("earth",3)]);
    let mut spaced=vec![None;9];spaced[2]=compact[0].clone();spaced[4]=compact[1].clone();spaced[8]=compact[2].clone();
    let a=compare(&compact,3,&recipe(),ID);let b=compare(&spaced,3,&recipe(),ID);equal_stack(output(&a),output(&b));
    let ControlFlow::Break(rest)=b.1 else {panic!()};assert_eq!(rest[4].as_ref().unwrap().item.as_ref(),"minecraft:glass_bottle");
    assert_eq!(rest[8].as_ref().unwrap().count,1);assert!(rest[2].is_none());
}
#[test]
fn full_active_tool_returns_original_bottles_but_inactive_is_unmatched() {
    for active in [false,true] {let p=compare(&grid(tool(50,50,active,0),vec![bottle("earth",9)]),3,&recipe(),ID);
        assert_eq!(matches!(p.0,ControlFlow::Break(Some(Some(_)))),active);
    }
}
#[test]
fn overworked_and_saturated_values_keep_existing_refill_math() {
    for current in [-1,0,49,i32::MAX] {for overworked in [-1,0,1,i32::MAX] {
        compare(&grid(tool(current,50,true,overworked),vec![bottle("earth",1),bottle("earth",1)]),3,&recipe(),ID);
    }}
}
#[test]
fn mixed_bottles_multiple_tools_and_wrong_tool_counts_still_reject() {
    let mut wrong=tool(0,50,true,0);wrong.count=2;
    for g in [grid(tool(0,50,true,0),vec![bottle("earth",1),bottle("sky",1)]),grid(wrong,vec![bottle("earth",1)]),grid(tool(0,50,true,0),vec![tool(0,50,true,0),bottle("earth",1)])] {
        let p=compare(&g,3,&recipe(),ID);assert_eq!(p.0,ControlFlow::Break(None));
    }
}
#[test]
fn absent_and_malformed_stored_custom_data_preserve_original_results() {
    for tag in [None,Some(Tag::Int(7)),Some(Tag::compound([("tic_overslime_active",Tag::Byte(1))]))] {
        let mut item=ItemStack::new("example:tool",1);if let Some(tag)=tag {item.data_mut().other.push((CUSTOM.into(),tag));}
        compare(&grid(item,vec![bottle("earth",1)]),3,&recipe(),ID);
    }
}
#[test]
fn full_unknown_fields_runtime_values_and_native_clone_are_preserved() {
    let mut item=tool(0,50,true,0);item.damage=17;
    let data=item.data_mut();data.custom_name=Some(Tag::compound([("translate",Tag::String("example:tool".into()))]));
    data.other.push(("future:opaque".into(),Tag::Float(0x7fc0_1234)));data.bundle_selected=BundleSelected(7);data.map_post_processing=Some(1);
    data.container.push((2,ItemStack::new("example:nested",9)));
    let expected=item.clone();let p=compare(&grid(item,vec![bottle("earth",1)]),3,&recipe(),ID);let got=output(&p);
    assert_eq!(got.damage,17);assert_eq!(got.data().unwrap().other[1],expected.data().unwrap().other[1]);
    assert_eq!(got.data().unwrap().bundle_selected.0,7);assert_eq!(got.data().unwrap().map_post_processing,Some(1));
}
#[test]
fn top_wire_bytes_survive_clone_but_modified_output_has_no_wire_authority() {
    let mut item=tool(0,50,true,0);let raw:Arc<[u8]>=Arc::from(vec![0xa5,0,0xff]);let decoded=item.clone();item.data_mut().wire=Wire::new(decoded,raw.clone());
    assert_eq!(item.wire_patch().as_deref(),Some(&*raw));
    let p=compare(&grid(item,vec![bottle("earth",1)]),3,&recipe(),ID);let got=output(&p);
    assert_eq!(got.data().unwrap().wire.patch(),Some(&*raw));assert!(got.wire_patch().is_none());
}
#[test]
fn nested_wire_and_unused_bottle_metadata_survive_without_decode() {
    let mut item=tool(45,50,true,0);let mut nested=ItemStack::new("future:item",1);let raw:Arc<[u8]>=Arc::from(vec![0x99]);let decoded=nested.clone();nested.data_mut().wire=Wire::new(decoded,raw.clone());item.data_mut().bundle_contents.push(nested);
    let mut spare=bottle("earth",6);spare.data_mut().other.push(("future:data".into(),Tag::String(JavaString(vec![0xd800]))));
    let p=compare(&grid(item,vec![bottle("earth",1),spare]),3,&recipe(),ID);
    assert_eq!(output(&p).data().unwrap().bundle_contents[0].data().unwrap().wire.patch(),Some(&*raw));
    let ControlFlow::Break(rest)=p.1 else {panic!()};assert_eq!(rest[2].as_ref().unwrap().data().unwrap().other.len(),1);
}
#[test]
fn selector_shape_result_and_exact_remainder_id_keep_native_fallback() {
    let g=grid(tool(0,50,true,0),vec![bottle("earth",1)]);
    for r in [Recipe::Special(Special::RepairItem),Recipe::Shaped{width:1,height:1,cells:vec![None],result:Template{item:Some(ID.into()),count:1,damage:0,extra:None,components:None}},Recipe::Shapeless{ingredients:vec![],result:Template{item:Some("example:other".into()),count:1,damage:0,extra:None,components:None}}] {
        let p=compare(&g,3,&r,"minecraft:tconstruct:overslime_bottle_recipe");assert!(matches!(p.0,ControlFlow::Continue(())));assert!(matches!(p.1,ControlFlow::Continue(())));
    }
}
#[test]
fn independent_requests_repeat_the_pure_value_not_a_cached_pair() {
    let a=grid(tool(0,50,true,0),vec![bottle("earth",1)]);let b=grid(tool(45,50,true,0),vec![bottle("earth",1)]);
    let first=compare(&a,3,&recipe(),ID);let second=compare(&b,3,&recipe(),ID);assert_ne!(output(&first),output(&second));
    let again=compare(&a,3,&recipe(),ID);equal_stack(output(&first),output(&again));
}
static CALLS: AtomicUsize=AtomicUsize::new(0);
static COUNTER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn interested(_: &Recipe)->bool {CALLS.fetch_add(1,Ordering::SeqCst);true}
fn derivation(_: &[Option<ItemStack>])->Option<pluto_inventory_api::crafting::products::Pair<ItemStack>> {CALLS.fetch_add(100,Ordering::SeqCst);None}
fn counted(id:&'static str)->PureRecipe {PureRecipe{id,assembly_interest:interested,remainder_interest:|_|true,derive:derivation}}
#[test]
fn veto_prevents_even_selectors_and_output_authority() {
    let _guard=COUNTER_LOCK.lock().unwrap();
    let g=grid(tool(0,50,true,0),vec![bottle("earth",1)]);let input=Input::of(&g,3).unwrap();let registry=Registry::default();registry.insert(counted("only")).unwrap();
    CALLS.store(0,Ordering::SeqCst);assert_eq!(after_matching_from(&registry,Outcome::Denied{first:"deny"},&input,&recipe()),ControlFlow::Break(None));assert_eq!(CALLS.load(Ordering::SeqCst),0);
}
#[test]
fn duplicate_ids_never_replace_and_conflict_invokes_no_derivation() {
    let _guard=COUNTER_LOCK.lock().unwrap();
    let registry=Registry::default();registry.insert(counted("a")).unwrap();assert_eq!(registry.insert(pair_consumer::entry()),Ok(()));assert_eq!(registry.insert(counted("a")),Err(RegistrationError::DuplicateId));assert_eq!(registry.insert(counted("")),Err(RegistrationError::EmptyId));
    let g=grid(tool(0,50,true,0),vec![bottle("earth",1)]);let input=Input::of(&g,3).unwrap();CALLS.store(0,Ordering::SeqCst);
    assert_eq!(after_matching_from(&registry,Outcome::Native,&input,&recipe()),ControlFlow::Break(None));assert_eq!(CALLS.load(Ordering::SeqCst),1);
}
#[test]
fn empty_registry_does_not_claim_assembly_or_remainders() {
    let registry=Registry::default();let g=grid(tool(0,50,true,0),vec![bottle("earth",1)]);let input=Input::of(&g,3).unwrap();
    assert!(matches!(after_matching_from(&registry,Outcome::Native,&input,&recipe()),ControlFlow::Continue(())));
    assert!(matches!(remainders_from(&registry,ID,&g,3),ControlFlow::Continue(())));
}
#[test]
fn selectors_run_unlocked_and_snapshot_does_not_adopt_new_registration() {
    let registry=Registry::default();registry.insert(counted("a")).unwrap();let mut visited=vec![];
    assert!(matches!(select(registry.snapshot(),|entry| {visited.push(entry.id);registry.insert(counted("b")).unwrap();false}),Selection::Native));
    assert_eq!(visited,vec!["a"]);assert_eq!(registry.snapshot().len(),2);
}
#[test]
fn original_empty_grid_remainder_fallback_keeps_declared_side() {
    let registry=Registry::default();registry.insert(pair_consumer::entry()).unwrap();
    for size in [0,2,3] {let g=vec![None;size*size];assert_eq!(remainders_from(&registry,ID,&g,size),legacy_remainders::call(ID,&g,size));}
}
