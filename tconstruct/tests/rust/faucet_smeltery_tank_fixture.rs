extern crate self as horizon_client;
pub mod containers { pub enum BlockEntity { Chest(Vec<Option<horizon_items::items::ItemStack>>) } }
#[path="__MIXINS__/source-casting-tank.rs"] mod tconstruct_source_casting_tank;
#[path="__MIXINS__/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path="__MIXINS__/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path="__MIXINS__/source-smeltery-state.rs"] mod tconstruct_source_smeltery_state;
use std::collections::HashMap;
use horizon_world::nbt::Tag;
type Pos=(i32,i32,i32);
#[path="__MIXINS__/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path="__MIXINS__/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path="__MIXINS__/source-fluid-display.rs"] mod tconstruct_source_fluid_display;
#[path="__PLUTO__"] mod pluto_stack_fluid;
struct Info{name:String} struct Facts; impl Facts{fn get(&self,id:usize)->Info{Info{name:match id{1=>"tconstruct:seared_drain",2=>"tconstruct:seared_fuel_tank",3=>"tconstruct:smeltery_controller",4=>"tconstruct:seared_duct",5=>"tconstruct:scorched_drain",6=>"tconstruct:foundry_controller",7=>"tconstruct:seared_fuel_gauge",8=>"tconstruct:seared_ingot_tank",9=>"tconstruct:seared_ingot_gauge",10=>"tconstruct:scorched_fuel_tank",11=>"tconstruct:scorched_fuel_gauge",12=>"tconstruct:scorched_ingot_tank",13=>"tconstruct:scorched_ingot_gauge",14=>"tconstruct:seared_melter",15=>"tconstruct:seared_casting_tank",_=>"minecraft:air"}.into()}}} struct Res;impl Res{fn facts(&self)->Facts{Facts}}
struct Chunks{loaded:HashMap<(i32,i32),()>,owners:HashMap<Pos,Pos>} impl Chunks{fn contains_key(&self,p:&(i32,i32))->bool{self.loaded.contains_key(p)}fn __horizon_mixins_tconstruct_smeltery_owners(&self)->&HashMap<Pos,Pos>{&self.owners}}
struct World{blocks:HashMap<Pos,usize>,block_entities:HashMap<Pos,containers::BlockEntity>,chunks:Chunks,tags:HashMap<(Pos,String),Tag>,fail:bool,formed:bool} impl World{fn block(&self,x:i32,y:i32,z:i32)->usize{*self.blocks.get(&(x,y,z)).unwrap_or(&0)}}
struct Game<'a>{res:Res,remote:Option<()>,lan:Option<()>,lan_acting_uuid:Option<()>,world:World,process_fault:u8,process_calls:usize,signals:usize,slot_updates:usize,_life:std::marker::PhantomData<&'a()>}
impl Game<'_> { fn container_changed(&mut self, _: Pos) { self.slot_updates += 1; } }
mod tconstruct_source_smeltery_world{use super::*;pub(super) fn detect(g:&Game<'_>,_:Pos)->Option<([i32;3],[i32;3],u32)>{g.world.formed.then_some(([-2,-1,-1],[0,1,1],4000))}}
mod tconstruct_source_faucet_tank {
    use super::Tag;
    use super::tconstruct_source_melter_state::Tank;
    // __MELTER_PATCH__
    pub(super) fn tank_kind(name: &str) -> Option<(&'static str, u32)> {
        super::tconstruct_source_smeltery_fuel::capacity(name).map(|capacity|
            (if capacity == 4000 { "tconstruct:fuel_tank" } else { "tconstruct:ingot_tank" }, capacity))
    }
}
mod tconstruct_source_melter_store{use super::*;pub(super) fn read(w:&World,p:Pos,k:&str)->Option<Tag>{w.tags.get(&(p,k.into())).cloned()}pub(super) fn write_many(w:&mut World,v:&[(Pos,&str,Tag)])->bool{if w.fail||v.iter().any(|(p,k,_)|!w.tags.contains_key(&(*p,(*k).into()))){return false}for(p,k,t)in v{w.tags.insert((*p,(*k).into()),t.clone());}true}}
mod runtime{
use super::*;use super::tconstruct_source_melter_store as store;use super::tconstruct_source_smeltery_state::Smeltery;use super::tconstruct_source_melter_state::{Faucet,Tank};use super::containers::BlockEntity;
fn notify_casting_comparator(game: &mut Game<'_>, _: Pos, before: i32, after: i32) {
    if before != after { game.signals += 1; }
}
fn process_casting_tank(game: &mut Game<'_>, state: &mut super::tconstruct_source_casting_tank::CastingTank) -> bool {
    game.process_calls += 1;
    let changed = state.process(&|_| true, &|_| true, None, false);
    match game.process_fault {
        1 => { let BlockEntity::Chest(slots) = game.world.block_entities.get_mut(&(0,0,0)).unwrap(); slots[0] = None; },
        2 => { game.world.blocks.insert((-2,0,0), 4); },
        3 => { game.world.formed = false; },
        _ => (),
    }
    changed
}
// __SMELTERY_HELPERS__
// __PRODUCTION_ROUTE__
#[test]fn reserves_atomically_and_preserves_opaque_controller(){let(mut g,p,s,t,c)=super::fixture();let before=g.world.tags.get(&(c,"tconstruct:smeltery".into())).unwrap().get("heat_codes").cloned();assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(true));let raw=g.world.tags.get(&(c,"tconstruct:smeltery".into())).unwrap();assert_eq!(raw.get("heat_codes").cloned(),before);let state=Smeltery::from_nbt(raw).unwrap();let out=Tank::from_nbt(&store::read(&g.world,t,"tconstruct:fuel_tank").unwrap(),4000).unwrap();let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert_eq!((state.tank.total(),out.amount,f.pending.amount),(90,10,80));let old=g.world.tags.clone();g.world.fail=true;assert_eq!(smeltery_to_tank(&mut g,p,s,t,f,false),Some(false));assert_eq!(old,g.world.tags);}
#[test]fn tagged_fluid_and_unloaded_controller_wait(){let(mut g,p,s,t,c)=super::fixture();let raw=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap();let mut state=Smeltery::from_nbt(raw).unwrap();state.tank.fluids[0].tag=Some(Tag::compound([("test",Tag::Int(1))]));if let Tag::Compound(fields)=raw {fields.insert("tank".into(),state.tank.to_nbt());}let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);g.world.chunks.loaded.remove(&(-1,-1));assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}
#[test]fn potion_layers_conserve_identity_and_rollback(){let(mut g,p,s,t,c)=super::fixture();let first=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);let second=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);let raw=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap();let mut state=Smeltery::from_nbt(raw).unwrap();state.tank.fluids.clear();assert!(state.tank.insert_tagged("tconstruct:potion",Some(&first),180));assert!(state.tank.insert_tagged("tconstruct:potion",Some(&second),90));if let Tag::Compound(fields)=raw{fields.insert("tank".into(),state.tank.to_nbt());}g.lan=Some(());assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(true));let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let out=Tank::from_nbt(&store::read(&g.world,t,"tconstruct:fuel_tank").unwrap(),4000).unwrap();let state=Smeltery::from_nbt(&store::read(&g.world,c,"tconstruct:smeltery").unwrap()).unwrap();assert_eq!(state.tank.amount_tagged("tconstruct:potion",Some(&first)),90);assert_eq!(state.tank.amount_tagged("tconstruct:potion",Some(&second)),90);assert_eq!((f.pending.amount,out.amount),(80,10));assert_eq!(f.pending.tag,Some(first.clone()));assert_eq!(out.tag,Some(first));let before=g.world.tags.clone();g.world.fail=true;assert_eq!(smeltery_to_tank(&mut g,p,s,t,f,false),Some(false));assert_eq!(before,g.world.tags);g.world.fail=false;for _ in 0..25{let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();if !f.active{break}assert_eq!(smeltery_to_tank(&mut g,p,s,t,f,false),Some(true));}let state=Smeltery::from_nbt(&store::read(&g.world,c,"tconstruct:smeltery").unwrap()).unwrap();let out=Tank::from_nbt(&store::read(&g.world,t,"tconstruct:fuel_tank").unwrap(),4000).unwrap();let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert_eq!(state.tank.amount_tagged("tconstruct:potion",Some(&second)),90);assert_eq!(out.amount,180);assert_eq!(f.pending.amount,0);assert!(!f.active);}
#[test]fn potion_custom_effects_pass_only_nonnetwork(){let(mut g,p,s,t,c)=super::fixture();let tag=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into())),("CustomPotionColor",Tag::Int(0x123456)),("CustomPotionEffects",Tag::List(vec![Tag::compound([("Id",Tag::Byte(1)),("Duration",Tag::Int(120))])]))]);let raw=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap();let mut state=Smeltery::from_nbt(raw).unwrap();state.tank.fluids.clear();assert!(state.tank.insert_tagged("tconstruct:potion",Some(&tag),180));if let Tag::Compound(fields)=raw{fields.insert("tank".into(),state.tank.to_nbt());}assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(true));let out=Tank::from_nbt(&store::read(&g.world,t,"tconstruct:fuel_tank").unwrap(),4000).unwrap();assert_eq!(out.tag,Some(tag.clone()));let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert_eq!(f.pending.tag,Some(tag));g.lan=Some(());let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,f,false),Some(false));assert_eq!(before,g.world.tags);}
#[test]fn potion_network_complex_and_invalid_refuse(){for tag in [Tag::compound([("Potion",Tag::String("minecraft:swiftness".into())),("CustomPotionColor",Tag::Int(3))]),Tag::compound([("Potion",Tag::String(horizon_world::nbt::JavaString(vec![0xd800])))])]{let(mut g,p,s,t,c)=super::fixture();let raw=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap();let mut state=Smeltery::from_nbt(raw).unwrap();state.tank.fluids.clear();assert!(state.tank.insert_tagged("tconstruct:potion",Some(&tag),180));if let Tag::Compound(fields)=raw{fields.insert("tank".into(),state.tank.to_nbt());}g.lan=Some(());let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}}
#[test]fn invalid_pending_or_output_tag_waits(){for pending in [false,true] {let(mut g,p,s,t,_)=super::fixture();let tag=Tag::compound([("Potion",Tag::String("unknown:custom".into()))]);if pending {let mut f=Faucet::new();f.active=true;assert!(f.pending.insert_tagged("tconstruct:potion",Some(&tag),20));g.world.tags.insert((p,"tconstruct:faucet".into()),f.to_nbt());}else{let mut out=Tank::new(4000);assert!(out.insert_tagged("tconstruct:potion",Some(&tag),10));g.world.tags.insert((t,"tconstruct:fuel_tank".into()),out.to_nbt());}let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,f,false),Some(false));assert_eq!(before,g.world.tags);}}
#[test]fn stale_controller_unknown_tank_and_duct_fail_closed(){let(mut g,p,s,t,c)=super::fixture();g.world.formed=false;let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);g.world.formed=true;g.world.blocks.insert(s,4);assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),None);g.world.blocks.insert(s,1);if let Tag::Compound(raw)=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap(){if let Some(Tag::Compound(tank))=raw.get_mut(&"tank".into()){tank.insert("unknown".into(),Tag::Int(1));}}let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}

fn drain_input_fixture(source_id: usize, foundry: bool) -> (Game<'static>, Pos, Pos, Pos, Pos, &'static str, u32) {
    let (mut game, pos, drain, source, controller) = super::fixture();
    game.world.blocks.insert(source, source_id);
    if foundry { game.world.blocks.insert(drain, 5); game.world.blocks.insert(controller, 6); }
    let name = game.res.facts().get(source_id).name;
    let (key, capacity) = super::tconstruct_source_faucet_tank::tank_kind(&name).unwrap();
    let mut input = Tank::new(capacity);
    assert!(input.insert("minecraft:water", 180));
    game.world.tags.insert((source, key.into()), input.to_nbt());
    let raw = game.world.tags.get_mut(&(controller, "tconstruct:smeltery".into())).unwrap();
    let mut state = Smeltery::from_nbt(raw).unwrap();
    state.tank.fluids.clear();
    assert!(state.tank.insert("x:unrelated", 50));
    assert!(state.tank.insert("minecraft:water", 25));
    if let Tag::Compound(fields) = raw { fields.insert("tank".into(), state.tank.to_nbt()); }
    (game, pos, source, drain, controller, key, capacity)
}

#[test]
fn drain_input_all_eight_sources_preserve_layers_and_controller_metadata() {
    for source_id in [2,7,8,9,10,11,12,13] {
        for foundry in [false, true] {
            let (mut game, pos, source, target, controller, key, capacity) = drain_input_fixture(source_id, foundry);
            assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(true));
            let raw = store::read(&game.world, controller, "tconstruct:smeltery").unwrap();
            let state = Smeltery::from_nbt(&raw).unwrap();
            let input = Tank::from_nbt(&store::read(&game.world, source, key).unwrap(), capacity).unwrap();
            let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
            assert_eq!((input.amount, state.tank.total(), faucet.pending.amount), (90,85,80));
            assert_eq!(state.tank.fluids[0].id, "x:unrelated");
            assert_eq!(state.tank.amount("x:unrelated"), 50);
            assert_eq!(state.tank.amount("minecraft:water"), 35);
            assert_eq!(raw.get("heat_codes"), Some(&Tag::Int(123)));
        }
    }
}

#[test]
fn drain_input_capacity_and_failed_save_never_overfill_or_partially_commit() {
    let (mut game, pos, source, target, controller, key, capacity) = drain_input_fixture(2, false);
    let mut state = Smeltery::from_nbt(&store::read(&game.world, controller, "tconstruct:smeltery").unwrap()).unwrap();
    state.tank.fluids.clear();
    assert!(state.tank.insert("minecraft:water", 3995));
    let raw = store::read(&game.world, controller, "tconstruct:smeltery").unwrap();
    game.world.tags.insert((controller, "tconstruct:smeltery".into()), patch_smeltery_controller_tank(&raw, &state).unwrap());
    let before = game.world.tags.clone();
    game.world.fail = true;
    assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(false));
    assert_eq!(game.world.tags, before);
    game.world.fail = false;
    assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let state = Smeltery::from_nbt(&store::read(&game.world, controller, "tconstruct:smeltery").unwrap()).unwrap();
    let input = Tank::from_nbt(&store::read(&game.world, source, key).unwrap(), capacity).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((input.amount, state.tank.total(), faucet.pending.amount), (175,4000,0));
}

#[test]
fn drain_input_stale_structure_capacity_opaque_source_and_duct_wait() {
    for fault in 0..5 {
        let (mut game, pos, source, target, controller, key, _) = drain_input_fixture(2, false);
        if fault == 0 { game.world.formed = false; }
        if fault == 1 {
            let raw = game.world.tags.get_mut(&(controller, "tconstruct:smeltery".into())).unwrap();
            let mut state = Smeltery::from_nbt(raw).unwrap(); state.tank.capacity = 3999;
            if let Tag::Compound(fields) = raw { fields.insert("tank".into(), state.tank.to_nbt()); }
        }
        if fault == 2 {
            if let Tag::Compound(fields) = game.world.tags.get_mut(&(source, key.into())).unwrap() { fields.insert("future".into(), Tag::Int(1)); }
        }
        if fault == 3 { game.world.chunks.loaded.remove(&(-1,-1)); }
        if fault == 4 { game.world.blocks.insert(target, 4); }
        let before = game.world.tags.clone();
        assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), if fault == 4 { None } else { Some(false) });
        assert_eq!(game.world.tags, before);
    }
}

#[test]
fn drain_input_pending_survives_unreadable_source_and_finishes_after_removal() {
    let (mut game, pos, source, target, controller, _, _) = drain_input_fixture(2, false);
    assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    game.world.block_entities.remove(&source);
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    let before = game.world.tags.clone();
    assert_eq!(tank_to_smeltery(&mut game, pos, source, target, faucet, false), Some(false));
    assert_eq!(game.world.tags, before);
    game.world.blocks.remove(&source);
    for _ in 0..9 {
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(tank_to_smeltery(&mut game, pos, source, target, faucet, false), Some(true));
    }
    let state = Smeltery::from_nbt(&store::read(&game.world, controller, "tconstruct:smeltery").unwrap()).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!(state.tank.amount("minecraft:water"), 115);
    assert_eq!(faucet.pending.amount, 0);
    assert!(!faucet.active);
}

#[test]
fn drain_input_potion_tags_form_distinct_layers_without_changing_selection() {
    let (mut game, pos, source, target, controller, key, capacity) = drain_input_fixture(2, false);
    let first = Tag::compound([("Potion", Tag::String("minecraft:healing".into()))]);
    let second = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
    let raw = store::read(&game.world, controller, "tconstruct:smeltery").unwrap();
    let mut state = Smeltery::from_nbt(&raw).unwrap();
    state.tank.fluids.clear(); assert!(state.tank.insert_tagged("tconstruct:potion", Some(&first), 25));
    game.world.tags.insert((controller, "tconstruct:smeltery".into()), patch_smeltery_controller_tank(&raw, &state).unwrap());
    let mut input = Tank::new(capacity); assert!(input.insert_tagged("tconstruct:potion", Some(&second), 180));
    game.world.tags.insert((source, key.into()), input.to_nbt());
    assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let state = Smeltery::from_nbt(&store::read(&game.world, controller, "tconstruct:smeltery").unwrap()).unwrap();
    assert_eq!(state.tank.fluids[0].tag, Some(first.clone()));
    assert_eq!(state.tank.amount_tagged("tconstruct:potion", Some(&first)), 25);
    assert_eq!(state.tank.amount_tagged("tconstruct:potion", Some(&second)), 10);
}


fn melter_drain_fixture() -> (Game<'static>, Pos, Pos, Pos, Pos) {
    let (mut game, pos, source, target, controller, _, _) = drain_input_fixture(2, false);
    let mut melter = super::tconstruct_source_melter_state::Melter::new();
    assert!(melter.tank.insert("minecraft:water", 180));
    melter.slots[0] = Some(horizon_items::items::ItemStack::new("minecraft:copper_ingot", 2));
    melter.fuel = 30; melter.temperature = 1000; melter.progress[0] = 7;
    let mut raw = melter.to_nbt();
    if let Tag::Compound(fields) = &mut raw {
        fields.insert("future".into(), Tag::String("preserve".into()));
        if let Some(Tag::List(slots)) = fields.get_mut(&"slots".into()) {
            if let Tag::Compound(item) = &mut slots[0] { item.insert("future_item".into(), Tag::Int(9)); }
        }
    }
    game.world.blocks.insert(source, 14);
    game.world.block_entities.insert(source, BlockEntity::Chest(melter.slots.to_vec()));
    game.world.tags.insert((source, "tconstruct:melter".into()), raw);
    (game, pos, source, target, controller)
}

#[test]
fn melter_drain_patches_only_tank_and_keeps_live_inventory_untouched() {
    let (mut game, pos, source, target, controller) = melter_drain_fixture();
    let before = store::read(&game.world, source, "tconstruct:melter").unwrap();
    let BlockEntity::Chest(live) = game.world.block_entities.get(&source).unwrap();
    let before_live = live.clone();
    assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let mut expected_tank = Tank::new(super::tconstruct_source_melter_state::MELTER_CAPACITY);
    assert!(expected_tank.insert("minecraft:water", 90));
    let expected = super::tconstruct_source_faucet_tank::patch_melter_tank(&before, &expected_tank).unwrap();
    assert_eq!(store::read(&game.world, source, "tconstruct:melter"), Some(expected));
    let BlockEntity::Chest(live) = game.world.block_entities.get(&source).unwrap();
    assert_eq!(*live, before_live);
    let state = Smeltery::from_nbt(&store::read(&game.world, controller, "tconstruct:smeltery").unwrap()).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((state.tank.amount("minecraft:water"), faucet.pending.amount), (35,80));
}

#[test]
fn melter_drain_failed_save_and_noncanonical_tank_do_not_mutate() {
    for fault in 0..3 {
        let (mut game, pos, source, target, _) = melter_drain_fixture();
        game.world.fail = fault == 0;
        if fault == 1 {
            if let Tag::Compound(fields) = game.world.tags.get_mut(&(source, "tconstruct:melter".into())).unwrap() {
                if let Some(Tag::Compound(tank)) = fields.get_mut(&"tank".into()) { tank.insert("future".into(), Tag::Int(1)); }
            }
        }
        if fault == 2 { game.world.block_entities.remove(&source); }
        let before = game.world.tags.clone();
        assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(false));
        assert_eq!(game.world.tags, before);
    }
}

#[test]
fn melter_drain_finishes_all_packets_without_changing_fuel_or_items() {
    let (mut game, pos, source, target, controller) = melter_drain_fixture();
    let raw = store::read(&game.world, source, "tconstruct:melter").unwrap();
    let slots = raw.get("slots").cloned();
    assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    for _ in 0..25 {
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        if !faucet.active { break; }
        assert_eq!(tank_to_smeltery(&mut game, pos, source, target, faucet, false), Some(true));
    }
    let raw = store::read(&game.world, source, "tconstruct:melter").unwrap();
    assert_eq!(raw.get("slots").cloned(), slots);
    assert_eq!(raw.get("fuel"), Some(&Tag::Long(30)));
    assert_eq!(raw.get("temperature"), Some(&Tag::Long(1000)));
    let input = Tank::from_nbt(raw.get("tank").unwrap(), super::tconstruct_source_melter_state::MELTER_CAPACITY).unwrap();
    let state = Smeltery::from_nbt(&store::read(&game.world, controller, "tconstruct:smeltery").unwrap()).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((input.amount, state.tank.amount("minecraft:water"), faucet.pending.amount), (0,205,0));
    assert!(!faucet.active);
}

fn casting_drain_fixture(foundry: bool) -> (Game<'static>, Pos, Pos, Pos, Pos) {
    let (mut game, pos, source, target, controller, _, _) = drain_input_fixture(2, foundry);
    let mut state = super::tconstruct_source_casting_tank::CastingTank::new();
    assert!(state.tank.insert("minecraft:water", 3000));
    state.slots[0] = Some(horizon_items::items::ItemStack::new("minecraft:water_bucket", 1));
    state.redstone = true;
    game.world.blocks.insert(source, 15);
    game.world.block_entities.insert(source, BlockEntity::Chest(state.slots.to_vec()));
    game.world.tags.insert((source, "tconstruct:casting_tank".into()), state.to_nbt());
    (game, pos, source, target, controller)
}

#[test]
fn casting_drain_reprocesses_source_container_and_keeps_layers() {
    for foundry in [false, true] {
        let (mut game, pos, source, target, controller) = casting_drain_fixture(foundry);
        assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(true));
        let input = casting_tank_at(&game, source).unwrap();
        let raw = store::read(&game.world, controller, "tconstruct:smeltery").unwrap();
        let state = Smeltery::from_nbt(&raw).unwrap();
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!((input.tank.amount, state.tank.amount("minecraft:water"), faucet.pending.amount), (3910,35,80));
        assert!(input.redstone);
        assert!(input.slots[0].is_none());
        assert_eq!(input.slots[1].as_ref().unwrap().item.as_ref(), "minecraft:bucket");
        assert_eq!(input.to_nbt(), store::read(&game.world, source, "tconstruct:casting_tank").unwrap());
        assert_eq!(state.tank.fluids[0].id, "x:unrelated");
        assert_eq!(state.tank.amount("x:unrelated"), 50);
        assert_eq!(raw.get("heat_codes"), Some(&Tag::Int(123)));
        assert_eq!((game.process_calls, game.signals, game.slot_updates), (1,1,2));
        assert_eq!(tank_to_smeltery(&mut game, pos, source, target, faucet, false), Some(true));
        assert_eq!(game.process_calls, 1);
    }
}

#[test]
fn casting_drain_failed_save_and_changed_snapshots_do_not_publish_staged_slots() {
    for fault in 0..4 {
        let (mut game, pos, source, target, _) = casting_drain_fixture(false);
        game.world.fail = fault == 0;
        game.process_fault = fault;
        let before = game.world.tags.clone();
        assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(false));
        assert_eq!(game.world.tags, before);
        let BlockEntity::Chest(slots) = game.world.block_entities.get(&source).unwrap();
        assert_eq!(slots[0].as_ref().map(|item| item.item.as_ref()), if fault == 1 { None } else { Some("minecraft:water_bucket") });
        assert!(slots[1].is_none());
        assert_eq!((game.signals, game.slot_updates), (0,0));
    }
}

#[test]
fn casting_drain_unknown_or_mismatched_source_and_full_destination_wait() {
    for fault in 0..3 {
        let (mut game, pos, source, target, controller) = casting_drain_fixture(false);
        if fault == 0 {
            if let Tag::Compound(fields) = game.world.tags.get_mut(&(source, "tconstruct:casting_tank".into())).unwrap() { fields.insert("future".into(), Tag::Int(1)); }
        }
        if fault == 1 {
            let BlockEntity::Chest(slots) = game.world.block_entities.get_mut(&source).unwrap(); slots[0] = None;
        }
        if fault == 2 {
            let raw = store::read(&game.world, controller, "tconstruct:smeltery").unwrap();
            let mut state = Smeltery::from_nbt(&raw).unwrap();
            assert!(state.tank.insert("x:unrelated", 3925));
            game.world.tags.insert((controller, "tconstruct:smeltery".into()), patch_smeltery_controller_tank(&raw, &state).unwrap());
        }
        let before = game.world.tags.clone();
        assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(fault == 2));
        assert_eq!(game.world.tags, before);
        assert_eq!((game.process_calls, game.signals, game.slot_updates), (0,0,0));
    }
}

#[test]
fn casting_drain_finishes_water_packet_before_reserving_refilled_potion() {
    let (mut game, pos, source, target, controller) = casting_drain_fixture(false);
    let mut input = super::tconstruct_source_casting_tank::CastingTank::new();
    assert!(input.tank.insert("minecraft:water", 90));
    input.slots[0] = Some(horizon_items::items::ItemStack::new("minecraft:potion", 1).with_potion("minecraft:healing"));
    game.world.block_entities.insert(source, BlockEntity::Chest(input.slots.to_vec()));
    game.world.tags.insert((source, "tconstruct:casting_tank".into()), input.to_nbt());
    game.lan = Some(());
    assert_eq!(tank_to_smeltery(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let input = casting_tank_at(&game, source).unwrap();
    assert_eq!(input.tank.fluid.as_deref(), Some("tconstruct:potion"));
    let potion = input.tank.tag.clone().unwrap();
    for _ in 0..8 {
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(tank_to_smeltery(&mut game, pos, source, target, faucet, false), Some(true));
    }
    assert_eq!(game.process_calls, 1);
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!(faucet.pending.amount, 0);
    assert_eq!(tank_to_smeltery(&mut game, pos, source, target, faucet, false), Some(true));
    let state = Smeltery::from_nbt(&store::read(&game.world, controller, "tconstruct:smeltery").unwrap()).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!(state.tank.amount("minecraft:water"), 115);
    assert_eq!(state.tank.amount_tagged("tconstruct:potion", Some(&potion)), 10);
    assert_eq!((faucet.pending.amount, faucet.pending.tag), (80,Some(potion)));
}


}
fn fixture()->(Game<'static>,Pos,Pos,Pos,Pos){use tconstruct_source_melter_state::{Tank,Faucet};use tconstruct_source_smeltery_state::Smeltery;let(p,s,t,c)=((0,1,0),(-2,0,0),(0,0,0),(-1,0,-1));let mut state=Smeltery::new();state.tank.capacity=4000;state.tank.insert("x:copper",180);let mut raw=state.to_nbt();if let Tag::Compound(fields)=&mut raw{fields.insert("heat_codes".into(),Tag::Int(123));}let tags=HashMap::from([((c,"tconstruct:smeltery".into()),raw),((p,"tconstruct:faucet".into()),Faucet::new().to_nbt()),((t,"tconstruct:fuel_tank".into()),Tank::new(4000).to_nbt())]);(Game{res:Res,remote:None,lan:None,lan_acting_uuid:None,world:World{blocks:HashMap::from([(s,1),(t,2),(c,3)]),block_entities:HashMap::from([(c,containers::BlockEntity::Chest(vec![])),(t,containers::BlockEntity::Chest(vec![]))]),chunks:Chunks{loaded:HashMap::from([((-1,0),()),((0,0),()),((-1,-1),())]),owners:HashMap::from([(s,c)])},tags,fail:false,formed:true},process_fault:0,process_calls:0,signals:0,slot_updates:0,_life:std::marker::PhantomData},p,s,t,c)}
