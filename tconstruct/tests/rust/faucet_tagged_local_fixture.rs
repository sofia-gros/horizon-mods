extern crate self as horizon_client;
pub mod containers{pub enum BlockEntity{Chest(Vec<Option<horizon_items::items::ItemStack>>)}}
#[path="__MIXINS__/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path="__MIXINS__/source-faucet-tank.rs"] mod tconstruct_source_faucet_tank;
#[path="__MIXINS__/source-casting-tank.rs"] mod tconstruct_source_casting_tank;
use std::collections::HashMap;use horizon_world::nbt::Tag;use horizon_items::items::ItemStack;type Pos=(i32,i32,i32);
#[path="__MIXINS__/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path="__MIXINS__/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path="__MIXINS__/source-fluid-display.rs"] mod tconstruct_source_fluid_display;
#[path="__MIXINS__/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path="__PLUTO__"] mod pluto_stack_fluid;
struct Info{name:String}struct Facts;impl Facts{fn get(&self,id:usize)->Info{Info{name:match id{1=>"tconstruct:seared_fuel_tank",2=>"tconstruct:seared_casting_tank",3=>"tconstruct:seared_melter",4=>"tconstruct:seared_lantern",5=>"tconstruct:scorched_lantern",_=>"minecraft:air"}.into()}}}struct Res;impl Res{fn facts(&self)->Facts{Facts}}
struct World{blocks:HashMap<Pos,usize>,block_entities:HashMap<Pos,containers::BlockEntity>,chunks:HashMap<(i32,i32),()>,tags:HashMap<(Pos,String),Tag>,fail:bool}impl World{fn block(&self,x:i32,y:i32,z:i32)->usize{*self.blocks.get(&(x,y,z)).unwrap_or(&0)}}
struct Game<'a>{res:Res,world:World,remote:Option<()>,lan:Option<()>,lan_acting_uuid:Option<()>,notified:bool,process_fault:u8,process_calls:usize,fault_after:usize,_life:std::marker::PhantomData<&'a()>}impl Game<'_>{fn ensure_datapacks(&mut self){}fn container_changed(&mut self,_:Pos){self.notified=true}}
mod tconstruct_source_melter_store{use super::*;pub(super) fn read(w:&World,p:Pos,k:&str)->Option<Tag>{w.tags.get(&(p,k.into())).cloned()}pub(super) fn write_many(w:&mut World,v:&[(Pos,&str,Tag)])->bool{if w.fail||v.iter().any(|(p,k,_)|!w.tags.contains_key(&(*p,(*k).into()))){return false}for(p,k,t)in v{w.tags.insert((*p,(*k).into()),t.clone());}true}}
mod tconstruct_source_machine_runtime{use super::*;use super::containers::BlockEntity;use super::tconstruct_source_melter_store as store;use super::tconstruct_source_melter_state::{Faucet,Tank};use super::tconstruct_source_casting_tank::CastingTank;
pub(super) fn kind(n:&str)->Option<&'static str>{(n=="tconstruct:seared_fuel_tank").then_some("tconstruct:fuel_tank")}
fn casting_tank_at(g:&Game<'_>,p:Pos)->Option<CastingTank>{let mut c=CastingTank::from_nbt(&store::read(&g.world,p,"tconstruct:casting_tank")?)?;let BlockEntity::Chest(slots)=g.world.block_entities.get(&p)?;if slots.len()<2{return None}c.slots=[slots[0].clone(),slots[1].clone()];c.lossless().then_some(c)}
fn process_casting_tank(game: &mut Game<'_>, state: &mut CastingTank) -> bool {
    let changed = state.process(&|_| true, &|_| true, None, game.lan.is_none() && game.lan_acting_uuid.is_none());
    game.process_calls += 1;
    if game.process_calls == game.fault_after {
        match game.process_fault {
            1 => { let BlockEntity::Chest(slots) = game.world.block_entities.get_mut(&(-1,1,0)).unwrap(); slots[0] = Some(ItemStack::new("minecraft:stone",1)); },
            2 => { let BlockEntity::Chest(slots) = game.world.block_entities.get_mut(&(0,0,0)).unwrap(); slots[0] = Some(ItemStack::new("minecraft:stone",1)); },
            3 => { game.world.blocks.insert((0,1,0), 1); },
            4 => { game.world.chunks.clear(); },
            5 => { game.world.block_entities.remove(&(-1,1,0)); },
            _ => {},
        }
    }
    changed
}fn notify_casting_comparator(_: &mut Game<'_>,_:Pos,_:i32,_:i32){}
// __PRODUCTION_ROUTE__
#[test]fn test_tagged_local_potion_fills_bottle(){let(mut g,p,s,t)=super::fixture();let effect=Tag::compound([("Id",Tag::Byte(1)),("Duration",Tag::Int(120)),("Amplifier",Tag::Byte(1))]);let tag=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into())),("CustomPotionColor",Tag::Int(0x123456)),("CustomPotionEffects",Tag::List(vec![effect]))]);let mut src=super::tconstruct_source_melter_state::Tank::new(4000);assert!(src.insert_tagged("tconstruct:potion",Some(&tag),250));g.world.tags.insert((s,"tconstruct:fuel_tank".into()),src.to_nbt());let mut out=CastingTank::new();out.slots[0]=Some(ItemStack::new("minecraft:glass_bottle",1));g.world.tags.insert((t,"tconstruct:casting_tank".into()),out.to_nbt());let BlockEntity::Chest(slots)=g.world.block_entities.get_mut(&t).unwrap();slots[0]=out.slots[0].clone();assert_eq!(tank_to_casting_tank(&mut g,p,s,t,Faucet::new(),true),Some(true));let mut tested_failure=false;for _ in 0..35 {let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let current=CastingTank::from_nbt(&store::read(&g.world,t,"tconstruct:casting_tank").unwrap()).unwrap();if current.tank.amount==240 {let before=g.world.tags.clone();g.world.fail=true;assert_eq!(tank_to_casting_tank(&mut g,p,s,t,f.clone(),false),Some(false));assert_eq!(before,g.world.tags);let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap();assert_eq!(slots[0].as_ref().unwrap().item.as_ref(),"minecraft:glass_bottle");assert!(slots[1].is_none());g.world.fail=false;tested_failure=true;}assert_eq!(tank_to_casting_tank(&mut g,p,s,t,f,false),Some(true));let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let src=super::tconstruct_source_melter_state::Tank::from_nbt(&store::read(&g.world,s,"tconstruct:fuel_tank").unwrap(),4000).unwrap();let current=CastingTank::from_nbt(&store::read(&g.world,t,"tconstruct:casting_tank").unwrap()).unwrap();let bottled=if current.slots[1].is_some(){250}else{0};assert_eq!(src.amount+f.pending.amount+current.tank.amount+bottled,250);if bottled>0{break}}assert!(tested_failure);let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap();let result=slots[1].as_ref().unwrap();assert_eq!(result.item.as_ref(),"minecraft:potion");assert_eq!(result.potion(),Some("minecraft:swiftness"));let extra=&result.data().unwrap().potion_extra;assert!(extra.iter().any(|(k,v)|k=="custom_color" && *v==Tag::Int(0x123456)));assert!(extra.iter().any(|(k,_)|k=="custom_effects"));let output=CastingTank::from_nbt(&store::read(&g.world,t,"tconstruct:casting_tank").unwrap()).unwrap();assert_eq!(output.tank.amount,0);}
#[test]fn test_tagged_network_refuses_without_mutation(){let(mut g,p,s,t)=super::fixture();let tag=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into())),("CustomPotionColor",Tag::Int(0x123456))]);let mut src=super::tconstruct_source_melter_state::Tank::new(4000);assert!(src.insert_tagged("tconstruct:potion",Some(&tag),250));g.world.tags.insert((s,"tconstruct:fuel_tank".into()),src.to_nbt());for variant in 0..3 {g.remote=(variant==0).then_some(());g.lan=(variant==1).then_some(());g.lan_acting_uuid=(variant==2).then_some(());let before=g.world.tags.clone();assert_eq!(tank_to_casting_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}}
#[test]fn test_tagged_invalid_utf16_and_unknown_effect_refuse(){let tags=[Tag::compound([("Potion",Tag::String(horizon_world::nbt::JavaString(vec![0xd800])))]),Tag::compound([("Potion",Tag::String("minecraft:swiftness".into())),("CustomPotionEffects",Tag::List(vec![Tag::compound([("Id",Tag::Byte(127))])]))])];for tag in tags {let(mut g,p,s,t)=super::fixture();let mut src=super::tconstruct_source_melter_state::Tank::new(4000);assert!(src.insert_tagged("tconstruct:potion",Some(&tag),250));g.world.tags.insert((s,"tconstruct:fuel_tank".into()),src.to_nbt());let before=g.world.tags.clone();assert_eq!(tank_to_casting_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}}
#[test]fn test_tagged_mismatch_and_untagged_network(){let(mut g,p,s,t)=super::fixture();let tag=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);let other=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);let mut src=super::tconstruct_source_melter_state::Tank::new(4000);assert!(src.insert_tagged("tconstruct:potion",Some(&tag),250));g.world.tags.insert((s,"tconstruct:fuel_tank".into()),src.to_nbt());let mut out=CastingTank::new();assert!(out.tank.insert_tagged("tconstruct:potion",Some(&other),10));out.slots[0]=Some(ItemStack::new("minecraft:bucket",1));g.world.tags.insert((t,"tconstruct:casting_tank".into()),out.to_nbt());let before=g.world.tags.clone();assert_eq!(tank_to_casting_tank(&mut g,p,s,t,Faucet::new(),true),Some(true));assert_eq!(before,g.world.tags);let(mut g,p,s,t)=super::fixture();g.lan=Some(());assert_eq!(tank_to_casting_tank(&mut g,p,s,t,Faucet::new(),true),Some(true));}
#[test]fn test_tagged_bare_id_network_refuses(){let(mut g,p,s,t)=super::fixture();g.lan=Some(());let tag=Tag::compound([("Potion",Tag::String("swiftness".into()))]);let mut src=super::tconstruct_source_melter_state::Tank::new(4000);assert!(src.insert_tagged("tconstruct:potion",Some(&tag),250));g.world.tags.insert((s,"tconstruct:fuel_tank".into()),src.to_nbt());let before=g.world.tags.clone();assert_eq!(tank_to_casting_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}
#[test]fn test_tagged_basic_potion_network_allowed(){for variant in 0..3 {let(mut g,p,s,t)=super::fixture();g.remote=(variant==0).then_some(());g.lan=(variant==1).then_some(());g.lan_acting_uuid=(variant==2).then_some(());let tag=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);let mut src=super::tconstruct_source_melter_state::Tank::new(4000);assert!(src.insert_tagged("tconstruct:potion",Some(&tag),250));g.world.tags.insert((s,"tconstruct:fuel_tank".into()),src.to_nbt());let mut out=CastingTank::new();out.slots[0]=Some(ItemStack::new("minecraft:glass_bottle",1));g.world.tags.insert((t,"tconstruct:casting_tank".into()),out.to_nbt());let BlockEntity::Chest(slots)=g.world.block_entities.get_mut(&t).unwrap();slots[0]=out.slots[0].clone();assert_eq!(tank_to_casting_tank(&mut g,p,s,t,Faucet::new(),true),Some(true));for _ in 0..35 {let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert_eq!(tank_to_casting_tank(&mut g,p,s,t,f,false),Some(true));let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap();if slots[1].is_some(){break}}let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap();let result=slots[1].as_ref().unwrap();assert_eq!(result.item.as_ref(),"minecraft:potion");assert_eq!(result.potion(),Some("minecraft:swiftness"));}}
#[test]fn test_tagged_unknown_potion_refuses(){for id in ["empty","minecraft:empty","unknown:custom"] {let(mut g,p,s,t)=super::fixture();let tag=Tag::compound([("Potion",Tag::String(id.into()))]);let mut src=super::tconstruct_source_melter_state::Tank::new(4000);assert!(src.insert_tagged("tconstruct:potion",Some(&tag),250));g.world.tags.insert((s,"tconstruct:fuel_tank".into()),src.to_nbt());let before=g.world.tags.clone();assert_eq!(tank_to_casting_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}}

fn chain_fixture() -> (Game<'static>, Pos, Pos, Pos) {
    let (mut game, pos, source, target) = super::fixture();
    let mut input = CastingTank::new();
    assert!(input.tank.insert("minecraft:water", 3000));
    input.slots[0] = Some(ItemStack::new("minecraft:water_bucket", 1));
    let mut output = CastingTank::new();
    assert!(output.tank.insert("minecraft:water", 990));
    output.slots[0] = Some(ItemStack::new("minecraft:bucket", 1));
    game.world.blocks.insert(source, 2);
    for (at, state) in [(source, input), (target, output)] {
        game.world.tags.insert((at, "tconstruct:casting_tank".into()), state.to_nbt());
        game.world.block_entities.insert(at, BlockEntity::Chest(state.slots.to_vec()));
    }
    (game, pos, source, target)
}

#[test]
fn casting_chain_reprocesses_both_containers_in_one_commit() {
    let (mut game, pos, source, target) = chain_fixture();
    assert_eq!(tank_to_casting_tank(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let input = casting_tank_at(&game, source).unwrap();
    let output = casting_tank_at(&game, target).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((input.tank.amount, output.tank.amount, faucet.pending.amount), (3910, 0, 80));
    assert!(input.slots[0].is_none());
    assert!(output.slots[0].is_none());
    assert_eq!(input.slots[1].as_ref().unwrap().item.as_ref(), "minecraft:bucket");
    assert_eq!(output.slots[1].as_ref().unwrap().item.as_ref(), "minecraft:water_bucket");
    assert_eq!(input.tank.amount + output.tank.amount + faucet.pending.amount + 1000, 4990);
    assert_eq!(game.process_calls, 2);
}

#[test]
fn casting_chain_failed_save_or_second_callback_cannot_publish_either_inventory() {
    for fault in 0..5 {
        let (mut game, pos, source, target) = chain_fixture();
        game.world.fail = fault == 0;
        game.process_fault = fault;
        let before = game.world.tags.clone();
        assert_eq!(tank_to_casting_tank(&mut game, pos, source, target, Faucet::new(), true), Some(false));
        assert_eq!(game.world.tags, before);
        for (at, expected, changed) in [(source, "minecraft:water_bucket", fault == 1), (target, "minecraft:bucket", fault == 2)] {
            let BlockEntity::Chest(slots) = game.world.block_entities.get(&at).unwrap();
            assert_eq!(slots[0].as_ref().unwrap().item.as_ref(), if changed { "minecraft:stone" } else { expected });
            assert!(slots[1].is_none());
        }
        assert!(!game.notified);
    }
}

#[test]
fn casting_chain_network_base_potion_arrives_with_same_identity() {
    for mode in 0..3 {
        let (mut game, pos, source, target) = chain_fixture();
        game.remote = (mode == 0).then_some(());
        game.lan = (mode == 1).then_some(());
        game.lan_acting_uuid = (mode == 2).then_some(());
        let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        let mut input = CastingTank::new();
        assert!(input.tank.insert_tagged("tconstruct:potion", Some(&tag), 250));
        let mut output = CastingTank::new();
        output.slots[0] = Some(ItemStack::new("minecraft:glass_bottle", 1));
        for (at, state) in [(source, input), (target, output)] {
            game.world.tags.insert((at, "tconstruct:casting_tank".into()), state.to_nbt());
            game.world.block_entities.insert(at, BlockEntity::Chest(state.slots.to_vec()));
        }
        assert_eq!(tank_to_casting_tank(&mut game, pos, source, target, Faucet::new(), true), Some(true));
        for _ in 0..35 {
            let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
            assert_eq!(tank_to_casting_tank(&mut game, pos, source, target, faucet, false), Some(true));
            if casting_tank_at(&game, target).unwrap().slots[1].is_some() { break; }
        }
        let input = casting_tank_at(&game, source).unwrap();
        let output = casting_tank_at(&game, target).unwrap();
        assert_eq!(input.tank.amount + output.tank.amount, 0);
        let bottle = output.slots[1].as_ref().unwrap();
        assert_eq!(bottle.item.as_ref(), "minecraft:potion");
        assert_eq!(bottle.potion(), Some("minecraft:swiftness"));
    }
}

#[test]
fn casting_chain_self_transfer_and_opaque_sources_wait_without_mutation() {
    let (mut game, pos, source, target) = chain_fixture();
    let before = game.world.tags.clone();
    assert_eq!(tank_to_casting_tank(&mut game, pos, source, source, Faucet::new(), true), Some(false));
    assert_eq!(game.world.tags, before);
    if let Tag::Compound(fields) = game.world.tags.get_mut(&(source, "tconstruct:casting_tank".into())).unwrap() {
        fields.insert("future".into(), Tag::Int(1));
    }
    let before = game.world.tags.clone();
    assert_eq!(tank_to_casting_tank(&mut game, pos, source, target, Faucet::new(), true), Some(false));
    assert_eq!(game.world.tags, before);
}

fn melter_casting_fixture() -> (Game<'static>, Pos, Pos, Pos) {
    let (mut game, pos, source, target) = super::fixture();
    let mut melter = super::tconstruct_source_melter_state::Melter::new();
    assert!(melter.tank.insert("minecraft:water", 180));
    melter.slots[0] = Some(ItemStack::new("minecraft:copper_ingot", 2));
    melter.fuel = 30; melter.temperature = 1000; melter.progress[0] = 7;
    let mut raw = melter.to_nbt();
    if let Tag::Compound(fields) = &mut raw {
        fields.insert("future".into(), Tag::Int(123));
        if let Some(Tag::List(slots)) = fields.get_mut(&"slots".into()) {
            if let Tag::Compound(item) = &mut slots[0] { item.insert("future_item".into(), Tag::Int(9)); }
        }
    }
    game.world.blocks.insert(source, 3);
    game.world.block_entities.insert(source, BlockEntity::Chest(melter.slots.to_vec()));
    game.world.tags.insert((source, "tconstruct:melter".into()), raw);
    let mut output = casting_tank_at(&game, target).unwrap();
    assert!(output.tank.insert("minecraft:water", 990));
    game.world.tags.insert((target, "tconstruct:casting_tank".into()), output.to_nbt());
    game.fault_after = 1;
    (game, pos, source, target)
}

#[test]
fn melter_casting_preserves_raw_machine_and_stages_completed_container() {
    let (mut game, pos, source, target) = melter_casting_fixture();
    let before = store::read(&game.world, source, "tconstruct:melter").unwrap();
    let BlockEntity::Chest(slots) = game.world.block_entities.get(&source).unwrap();
    let before_live = slots.clone();
    assert_eq!(tank_to_casting_tank(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let raw = store::read(&game.world, source, "tconstruct:melter").unwrap();
    let input = super::tconstruct_source_melter_state::Melter::from_nbt(&raw).unwrap();
    assert_eq!(raw, super::tconstruct_source_faucet_tank::patch_melter_tank(&before, &input.tank).unwrap());
    let output = casting_tank_at(&game, target).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((input.tank.amount, output.tank.amount, faucet.pending.amount), (90,0,80));
    assert!(output.slots[0].is_none());
    assert_eq!(output.slots[1].as_ref().unwrap().item.as_ref(), "minecraft:water_bucket");
    assert_eq!(output.to_nbt(), store::read(&game.world, target, "tconstruct:casting_tank").unwrap());
    for _ in 0..9 {
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(tank_to_casting_tank(&mut game, pos, source, target, faucet, false), Some(true));
    }
    let input = super::tconstruct_source_melter_state::Melter::from_nbt(&store::read(&game.world, source, "tconstruct:melter").unwrap()).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((input.tank.amount, casting_tank_at(&game, target).unwrap().tank.amount, faucet.pending.amount), (0,90,80));
    let BlockEntity::Chest(slots) = game.world.block_entities.get(&source).unwrap();
    assert_eq!(*slots, before_live);
}

#[test]
fn melter_casting_failed_save_opaque_tank_and_stale_processing_callbacks_wait() {
    for fault in 0..6 {
        let (mut game, pos, source, target) = melter_casting_fixture();
        game.world.fail = fault == 0;
        if fault == 1 {
            if let Tag::Compound(fields) = game.world.tags.get_mut(&(source, "tconstruct:melter".into())).unwrap() {
                if let Some(Tag::Compound(tank)) = fields.get_mut(&"tank".into()) { tank.insert("future".into(), Tag::Int(1)); }
            }
        } else { game.process_fault = fault; }
        let before = game.world.tags.clone();
        assert_eq!(tank_to_casting_tank(&mut game, pos, source, target, Faucet::new(), true), Some(false));
        assert_eq!(game.world.tags, before);
        let BlockEntity::Chest(slots) = game.world.block_entities.get(&target).unwrap();
        assert_eq!(slots[0].as_ref().unwrap().item.as_ref(), if fault == 2 { "minecraft:stone" } else { "minecraft:bucket" });
        assert!(slots[1].is_none());
        assert!(!game.notified);
    }
}

#[test]
fn melter_casting_potion_bottle_keeps_base_identity_and_local_custom_color() {
    for variant in 0..3 {
        let (mut game, pos, source, target) = melter_casting_fixture();
        let mut tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        if variant != 0 { if let Tag::Compound(fields) = &mut tag { fields.insert("CustomPotionColor".into(), Tag::Int(0x123456)); } }
        game.lan = (variant != 1).then_some(());
        let raw = store::read(&game.world, source, "tconstruct:melter").unwrap();
        let mut tank = Tank::new(super::tconstruct_source_melter_state::MELTER_CAPACITY);
        assert!(tank.insert_tagged("tconstruct:potion", Some(&tag), 250));
        game.world.tags.insert((source, "tconstruct:melter".into()), super::tconstruct_source_faucet_tank::patch_melter_tank(&raw, &tank).unwrap());
        let mut output = CastingTank::new(); assert!(output.tank.insert_tagged("tconstruct:potion", Some(&tag), 240));
        output.slots[0] = Some(ItemStack::new("minecraft:glass_bottle", 1));
        game.world.tags.insert((target, "tconstruct:casting_tank".into()), output.to_nbt());
        game.world.block_entities.insert(target, BlockEntity::Chest(output.slots.to_vec()));
        let before = game.world.tags.clone();
        assert_eq!(tank_to_casting_tank(&mut game, pos, source, target, Faucet::new(), true), Some(variant != 2));
        if variant == 2 { assert_eq!(game.world.tags, before); }
        else {
            let output = casting_tank_at(&game, target).unwrap();
            let item = output.slots[1].as_ref().unwrap();
            assert_eq!(item.item.as_ref(), "minecraft:potion");
            assert_eq!(item.potion(), Some("minecraft:swiftness"));
            if variant == 1 { assert!(item.data().unwrap().potion_extra.iter().any(|(key, value)| key == "custom_color" && *value == Tag::Int(0x123456))); }
            let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
            assert_eq!((faucet.pending.amount, faucet.pending.tag), (80,Some(tag)));
        }
    }
}


fn lantern_casting_fixture(id:usize)->(Game<'static>,Pos,Pos,Pos) {
    let (mut game,pos,source,target)=super::fixture();
    game.world.blocks.insert(source,id);
    game.world.tags.remove(&(source,"tconstruct:fuel_tank".into()));
    let mut input=Tank::new(50); assert!(input.insert("minecraft:water",50));
    game.world.tags.insert((source,"tconstruct:lantern".into()),input.to_nbt());
    let mut output=casting_tank_at(&game,target).unwrap(); assert!(output.tank.insert("minecraft:water",990));
    game.world.tags.insert((target,"tconstruct:casting_tank".into()),output.to_nbt());
    game.fault_after=1;
    (game,pos,source,target)
}

#[test]
fn lantern_sources_stage_completed_bucket_and_conserve_fifty_mb() {
    for id in [4,5] {
        let (mut game,pos,source,target)=lantern_casting_fixture(id);
        assert_eq!(tank_to_casting_tank(&mut game,pos,source,target,Faucet::new(),true),Some(true));
        let input=Tank::from_nbt(&store::read(&game.world,source,"tconstruct:lantern").unwrap(),50).unwrap();
        let output=casting_tank_at(&game,target).unwrap();
        let faucet=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
        assert_eq!((input.amount,output.tank.amount,faucet.pending.amount),(0,0,40));
        assert!(output.slots[0].is_none()); assert_eq!(output.slots[1].as_ref().unwrap().item.as_ref(),"minecraft:water_bucket");
        assert_eq!(output.to_nbt(),store::read(&game.world,target,"tconstruct:casting_tank").unwrap());
        for _ in 0..5 {
            let faucet=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
            assert_eq!(tank_to_casting_tank(&mut game,pos,source,target,faucet,false),Some(true));
        }
        let output=casting_tank_at(&game,target).unwrap();
        let faucet=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(output.tank.amount,40); assert_eq!(faucet.pending.amount,0); assert!(!faucet.active);
        assert_eq!(output.tank.amount+1000,990+50);
    }
}

#[test]
fn lantern_casting_failed_save_opaque_source_and_processing_races_wait() {
    for fault in 0..6 {
        let (mut game,pos,source,target)=lantern_casting_fixture(4);
        game.world.fail=fault==0;
        if fault==1 {
            if let Tag::Compound(fields)=game.world.tags.get_mut(&(source,"tconstruct:lantern".into())).unwrap() {fields.insert("future".into(),Tag::Int(7));}
        } else { game.process_fault=fault; }
        let before=game.world.tags.clone();
        assert_eq!(tank_to_casting_tank(&mut game,pos,source,target,Faucet::new(),true),Some(false));
        assert_eq!(game.world.tags,before); assert!(!game.notified);
        let BlockEntity::Chest(slots)=game.world.block_entities.get(&target).unwrap();
        assert_eq!(slots[0].as_ref().unwrap().item.as_ref(),if fault==2 {"minecraft:stone"} else {"minecraft:bucket"}); assert!(slots[1].is_none());
    }
}

#[test]
fn lantern_casting_potion_bottle_keeps_base_identity_and_local_color_only() {
    for variant in 0..3 {
        let (mut game,pos,source,target)=lantern_casting_fixture(5);
        let mut tag=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);
        if variant!=0 {if let Tag::Compound(fields)=&mut tag {fields.insert("CustomPotionColor".into(),Tag::Int(0x123456));}}
        game.lan=(variant!=1).then_some(());
        let mut input=Tank::new(50); assert!(input.insert_tagged("tconstruct:potion",Some(&tag),50));
        game.world.tags.insert((source,"tconstruct:lantern".into()),input.to_nbt());
        let mut output=CastingTank::new(); assert!(output.tank.insert_tagged("tconstruct:potion",Some(&tag),240));
        output.slots[0]=Some(ItemStack::new("minecraft:glass_bottle",1));
        game.world.tags.insert((target,"tconstruct:casting_tank".into()),output.to_nbt());
        game.world.block_entities.insert(target,BlockEntity::Chest(output.slots.to_vec()));
        let before=game.world.tags.clone();
        assert_eq!(tank_to_casting_tank(&mut game,pos,source,target,Faucet::new(),true),Some(variant!=2));
        if variant==2 {assert_eq!(game.world.tags,before);} else {
            let output=casting_tank_at(&game,target).unwrap(); let item=output.slots[1].as_ref().unwrap();
            assert_eq!(item.item.as_ref(),"minecraft:potion"); assert_eq!(item.potion(),Some("minecraft:swiftness"));
            if variant==1 {assert!(item.data().unwrap().potion_extra.iter().any(|(key,value)|key=="custom_color" && *value==Tag::Int(0x123456)));}
            let faucet=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
            assert_eq!((faucet.pending.amount,faucet.pending.tag),(40,Some(tag)));
        }
    }
}

}
fn fixture()->(Game<'static>,Pos,Pos,Pos){use tconstruct_source_melter_state::{Tank,Faucet};use tconstruct_source_casting_tank::CastingTank;let(p,s,t)=((0,1,0),(-1,1,0),(0,0,0));let mut source=Tank::new(4000);source.insert("minecraft:water",1080);let mut output=CastingTank::new();output.slots[0]=Some(ItemStack::new("minecraft:bucket",1));let tags=HashMap::from([((s,"tconstruct:fuel_tank".into()),source.to_nbt()),((p,"tconstruct:faucet".into()),Faucet::new().to_nbt()),((t,"tconstruct:casting_tank".into()),output.to_nbt())]);let blocks=HashMap::from([(s,1),(t,2)]);let block_entities=HashMap::from([(s,containers::BlockEntity::Chest(vec![])),(t,containers::BlockEntity::Chest(output.slots.to_vec()))]);(Game{res:Res,remote:None,lan:None,lan_acting_uuid:None,world:World{blocks,block_entities,chunks:HashMap::from([((-1,0),()),((0,0),())]),tags,fail:false},notified:false,process_fault:0,process_calls:0,fault_after:2,_life:std::marker::PhantomData},p,s,t)}
