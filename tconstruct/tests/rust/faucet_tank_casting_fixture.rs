extern crate self as horizon_client;
pub mod containers { pub enum BlockEntity { Chest(Vec<Option<horizon_items::items::ItemStack>>) } }
#[path="__MIXINS__/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path="__MIXINS__/source-faucet-tank.rs"] mod tconstruct_source_faucet_tank;
#[path="__MIXINS__/source-casting-tank.rs"] mod tconstruct_source_casting_tank;
#[path="__MIXINS__/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path="__MIXINS__/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path="__MIXINS__/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path="__MIXINS__/source-fluid-display.rs"] mod tconstruct_source_fluid_display;
#[path="__PLUTO__"] mod pluto_stack_fluid;
use std::collections::HashMap;
use horizon_world::nbt::Tag;
type Pos=(i32,i32,i32);
struct Info{name:String} struct Facts; impl Facts{fn get(&self,id:usize)->Info{Info{name:match id{1=>"tconstruct:seared_fuel_tank",2=>"tconstruct:seared_table",3=>"tconstruct:seared_casting_tank",4=>"tconstruct:scorched_table",5=>"tconstruct:seared_fuel_gauge",6=>"tconstruct:seared_ingot_tank",7=>"tconstruct:seared_ingot_gauge",8=>"tconstruct:scorched_fuel_tank",9=>"tconstruct:scorched_fuel_gauge",10=>"tconstruct:scorched_ingot_tank",11=>"tconstruct:scorched_ingot_gauge",_=>"minecraft:air"}.into()}}} struct Res;impl Res{fn facts(&self)->Facts{Facts}}
struct World{blocks:HashMap<Pos,usize>,block_entities:HashMap<Pos,containers::BlockEntity>,chunks:HashMap<(i32,i32),()>,tags:HashMap<(Pos,String),Tag>,fail:bool} impl World{fn block(&self,x:i32,y:i32,z:i32)->usize{*self.blocks.get(&(x,y,z)).unwrap_or(&0)}}
struct Game<'a>{res:Res,world:World,signals:usize,slot_updates:usize,switch_target:bool,change_recipe:bool,recipe_changed:bool,change_target_slot:bool,change_source_slot:bool,_life:std::marker::PhantomData<&'a()>}
mod tconstruct_source_melter_store{use super::*;pub(super) fn read(w:&World,p:Pos,k:&str)->Option<Tag>{w.tags.get(&(p,k.into())).cloned()}pub(super) fn write_many(w:&mut World,v:&[(Pos,&str,Tag)])->bool{if w.fail||v.iter().any(|(p,k,_)|!w.tags.contains_key(&(*p,(*k).into()))){return false}for(p,k,t)in v{w.tags.insert((*p,(*k).into()),t.clone());}true}}
impl Game<'_>{fn container_changed(&mut self,_:Pos){self.slot_updates+=1}}
mod tconstruct_source_machine_runtime{
use super::*;use super::containers::BlockEntity;use super::tconstruct_source_melter_store as store;use super::tconstruct_source_melter_state::{Faucet,Tank,Casting};
pub(super) fn kind(n:&str)->Option<&'static str>{
    super::tconstruct_source_smeltery_fuel::capacity(n).map(|capacity|
        if capacity == 4000 { "tconstruct:fuel_tank" } else { "tconstruct:ingot_tank" })
}
fn ensure(_: &mut Game<'_>,_:Pos,_:&str)->bool{true}
fn notify_casting_comparator(g:&mut Game<'_>,_:Pos,before:i32,after:i32){if before!=after{g.signals+=1}}
fn process_casting_tank(g: &mut Game<'_>, state:&mut super::tconstruct_source_casting_tank::CastingTank)->bool{let changed=state.process(&|_|true,&|_|true,None,false);if g.switch_target{g.world.blocks.insert((0,0,0),4);}if g.change_recipe{g.recipe_changed=true;}if g.change_target_slot{let BlockEntity::Chest(slots)=g.world.block_entities.get_mut(&(0,0,0)).unwrap();slots[0]=Some(horizon_items::items::ItemStack::new("minecraft:stone",1));}if g.change_source_slot{let BlockEntity::Chest(slots)=g.world.block_entities.get_mut(&(-1,1,0)).unwrap();slots[0]=Some(horizon_items::items::ItemStack::new("minecraft:stone",1));}changed}
fn casting_tank_at(g:&Game<'_>,p:Pos)->Option<super::tconstruct_source_casting_tank::CastingTank>{let mut state=super::tconstruct_source_casting_tank::CastingTank::from_nbt(&store::read(&g.world,p,"tconstruct:casting_tank")?)?;let super::containers::BlockEntity::Chest(slots)=g.world.block_entities.get(&p)?;if slots.len()<2{return None}state.slots=[slots[0].clone(),slots[1].clone()];state.lossless().then_some(state)}
fn casting_at(g:&Game<'_>,p:Pos)->Option<(&'static str,Casting)>{let mut state=Casting::from_nbt(&tconstruct_source_melter_store::read(&g.world,p,"tconstruct:table")?)?;let BlockEntity::Chest(slots)=g.world.block_entities.get(&p)?;if slots.len()<2{return None}state.input=slots[0].clone();state.output=slots[1].clone();Some(("tconstruct:table",state))}
struct Fluid{amount:u32} struct Recipe{fluid:Fluid,cooling_time:u32,consumes_cast:bool,switch_slots:bool}
fn casting_recipe_at(g: &Game<'_>,_:Pos,_:&str,_:&Casting,t:&Tank)->Option<(String,Recipe,horizon_items::items::ItemStack)>{(matches!(t.fluid.as_deref(),Some("x:copper" | "minecraft:water"))).then(||((if g.recipe_changed {"test:changed"} else {"test:copper"}).into(),Recipe{fluid:Fluid{amount:90},cooling_time:1,consumes_cast:false,switch_slots:false},horizon_items::items::ItemStack::new("minecraft:copper_ingot",1)))}
fn prepare_faucet_casting(g:&Game<'_>,p:Pos,k:&str,c:&mut Casting,f:&Faucet,i:&Tank){if let Some(t)=super::tconstruct_source_melter_state::casting_faucet_offer(c,f,i){if let Some((id,r,out))=casting_recipe_at(g,p,k,c,t){c.prepare(&id,r.fluid.amount,r.cooling_time,out,r.consumes_cast,r.switch_slots);}}}
// __PRODUCTION_ROUTE__
#[test]fn reserve_tick_and_failed_write(){let(mut g,p,s,t)=super::fixture();assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(true));let source=Tank::from_nbt(&tconstruct_source_melter_store::read(&g.world,s,"tconstruct:fuel_tank").unwrap(),4000).unwrap();let faucet=Faucet::from_nbt(&tconstruct_source_melter_store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert_eq!((source.amount,faucet.pending.amount),(90,80));let before=g.world.tags.clone();g.world.fail=true;assert!(tick_tank_casting(&mut g,p,s,t,faucet.clone()));assert_eq!(before,g.world.tags);g.world.fail=false;assert!(tick_tank_casting(&mut g,p,s,t,faucet));let c=Casting::from_nbt(&tconstruct_source_melter_store::read(&g.world,t,"tconstruct:table").unwrap()).unwrap();let f=Faucet::from_nbt(&tconstruct_source_melter_store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert_eq!((c.tank.amount,f.pending.amount),(20,70));}
#[test]fn stale_recipe_and_unknown_casting_refuse(){let(mut g,p,s,t)=super::fixture();let mut c=Casting::new();assert!(c.prepare("test:old",90,1,horizon_items::items::ItemStack::new("minecraft:copper_ingot",1),false,false));g.world.tags.insert((t,"tconstruct:table".into()),c.to_nbt());let before=g.world.tags.clone();assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(true));assert_eq!(before,g.world.tags);let tag=g.world.tags.get_mut(&(t,"tconstruct:table".into())).unwrap();if let Tag::Compound(fields)=tag{fields.insert("legacy".into(),Tag::Int(1));}let before=g.world.tags.clone();assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(false));assert_eq!(before,g.world.tags);}
#[test]fn portable_source_preserves_slots_and_refuses_bad_state(){use super::tconstruct_source_casting_tank::CastingTank;let(mut g,p,s,t)=super::fixture();let mut ct=CastingTank::new();assert!(ct.tank.insert("x:copper",300));ct.slots[0]=Some(horizon_items::items::ItemStack::new("minecraft:bucket",1));ct.slots[1]=Some(horizon_items::items::ItemStack::new("minecraft:stone",2));g.world.blocks.insert(s,3);g.world.block_entities.insert(s,super::containers::BlockEntity::Chest(ct.slots.to_vec()));g.world.tags.insert((s,"tconstruct:casting_tank".into()),ct.to_nbt());let before_slots=ct.slots.clone();assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(true));let saved=CastingTank::from_nbt(&store::read(&g.world,s,"tconstruct:casting_tank").unwrap()).unwrap();assert_eq!(saved.tank.amount,210);assert_eq!(g.signals,1);assert_eq!(saved.slots,before_slots);let super::containers::BlockEntity::Chest(live)=g.world.block_entities.get(&s).unwrap();assert_eq!(live.as_slice(),before_slots);let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let old=g.world.tags.clone();let signals=g.signals;g.world.fail=true;assert!(tick_tank_casting(&mut g,p,s,t,f));assert_eq!(old,g.world.tags);assert_eq!(signals,g.signals);g.world.fail=false;let super::containers::BlockEntity::Chest(live)=g.world.block_entities.get_mut(&s).unwrap();live[0]=None;let old=g.world.tags.clone();let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert!(tick_tank_casting(&mut g,p,s,t,f));assert_eq!(old,g.world.tags);}
#[test]fn source_container_reprocess_is_atomic(){use super::tconstruct_source_casting_tank::CastingTank;let(mut g,p,s,t)=super::fixture();let mut ct=CastingTank::new();assert!(ct.tank.insert("minecraft:water",3000));ct.slots[0]=Some(horizon_items::items::ItemStack::new("minecraft:water_bucket",1));g.world.blocks.insert(s,3);g.world.block_entities.insert(s,super::containers::BlockEntity::Chest(ct.slots.to_vec()));g.world.tags.insert((s,"tconstruct:casting_tank".into()),ct.to_nbt());let before=g.world.tags.clone();g.world.fail=true;assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(false));assert_eq!(before,g.world.tags);let BlockEntity::Chest(slots)=g.world.block_entities.get(&s).unwrap();assert_eq!(slots[0].as_ref().unwrap().item.as_ref(),"minecraft:water_bucket");assert!(slots[1].is_none());assert_eq!((g.signals,g.slot_updates),(0,0));g.world.fail=false;assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(true));let saved=CastingTank::from_nbt(&store::read(&g.world,s,"tconstruct:casting_tank").unwrap()).unwrap();let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let out=Casting::from_nbt(&store::read(&g.world,t,"tconstruct:table").unwrap()).unwrap();assert_eq!(saved.tank.amount+f.pending.amount+out.tank.amount,4000);assert!(saved.slots[0].is_none());assert_eq!(saved.slots[1].as_ref().unwrap().item.as_ref(),"minecraft:bucket");let BlockEntity::Chest(slots)=g.world.block_entities.get(&s).unwrap();assert!(slots[0].is_none());assert_eq!(slots[1].as_ref().unwrap().item.as_ref(),"minecraft:bucket");assert_eq!((g.signals,g.slot_updates),(1,1));}
#[test]fn source_refill_tag_waits_only_after_untagged_packet_finishes(){use super::tconstruct_source_casting_tank::CastingTank;let(mut g,p,s,t)=super::fixture();let mut ct=CastingTank::new();assert!(ct.tank.insert("minecraft:water",90));ct.slots[0]=Some(horizon_items::items::ItemStack::new("minecraft:potion",1).with_potion("minecraft:swiftness"));g.world.blocks.insert(s,3);g.world.block_entities.insert(s,super::containers::BlockEntity::Chest(ct.slots.to_vec()));g.world.tags.insert((s,"tconstruct:casting_tank".into()),ct.to_nbt());assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(true));let saved=CastingTank::from_nbt(&store::read(&g.world,s,"tconstruct:casting_tank").unwrap()).unwrap();assert_eq!(saved.tank.fluid.as_deref(),Some("tconstruct:potion"));assert!(saved.tank.tag.is_some());for _ in 0..8{let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert!(tick_tank_casting(&mut g,p,s,t,f));}let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let out=Casting::from_nbt(&store::read(&g.world,t,"tconstruct:table").unwrap()).unwrap();assert_eq!((f.pending.amount,out.tank.amount),(0,90));let before=g.world.tags.clone();assert!(tick_tank_casting(&mut g,p,s,t,f));assert_eq!(before,g.world.tags);let mut f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();f.stop_after_packet=true;g.world.tags.insert((p,"tconstruct:faucet".into()),f.to_nbt());assert!(tick_tank_casting(&mut g,p,s,t,f));let stopped=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert!(!stopped.active);}
#[test]fn recheck_target_and_recipe_after_source_process(){use super::tconstruct_source_casting_tank::CastingTank;for variant in 0..4{let(mut g,p,s,t)=super::fixture();let mut ct=CastingTank::new();assert!(ct.tank.insert("minecraft:water",3000));ct.slots[0]=Some(horizon_items::items::ItemStack::new("minecraft:water_bucket",1));g.world.blocks.insert(s,3);g.world.block_entities.insert(s,super::containers::BlockEntity::Chest(ct.slots.to_vec()));g.world.tags.insert((s,"tconstruct:casting_tank".into()),ct.to_nbt());g.switch_target=variant==0;g.change_recipe=variant==1;g.change_target_slot=variant==2;g.change_source_slot=variant==3;let before=g.world.tags.clone();assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(false));assert_eq!(before,g.world.tags);let BlockEntity::Chest(slots)=g.world.block_entities.get(&s).unwrap();assert_eq!(slots[0].as_ref().unwrap().item.as_ref(),if variant==3 {"minecraft:stone"} else {"minecraft:water_bucket"});assert!(slots[1].is_none());assert_eq!((g.signals,g.slot_updates),(0,0));}}
#[test]fn portable_source_tagged_and_unknown_refuse(){use super::tconstruct_source_casting_tank::CastingTank;let(mut g,p,s,t)=super::fixture();let mut ct=CastingTank::new();let tag=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);assert!(ct.tank.insert_tagged("tconstruct:potion",Some(&tag),250));g.world.blocks.insert(s,3);g.world.block_entities.insert(s,super::containers::BlockEntity::Chest(ct.slots.to_vec()));g.world.tags.insert((s,"tconstruct:casting_tank".into()),ct.to_nbt());let before=g.world.tags.clone();assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(false));assert_eq!(before,g.world.tags);let raw=g.world.tags.get_mut(&(s,"tconstruct:casting_tank".into())).unwrap();if let Tag::Compound(fields)=raw{fields.insert("unknown".into(),Tag::Int(1));}let before=g.world.tags.clone();assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(false));assert_eq!(before,g.world.tags);}
#[test]fn unknown_source_and_unloaded_refuse(){let(mut g,p,s,t)=super::fixture();let tag=g.world.tags.get_mut(&(s,"tconstruct:fuel_tank".into())).unwrap();if let Tag::Compound(fields)=tag{fields.insert("legacy".into(),Tag::Int(1));}let before=g.world.tags.clone();assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(false));assert_eq!(before,g.world.tags);g.world.chunks.clear();assert_eq!(activate_tank_casting(&mut g,p,s,t,Faucet::new()),Some(false));assert_eq!(before,g.world.tags);}

fn output_fixture(destination: usize) -> (Game<'static>, Pos, Pos, Pos, &'static str, u32) {
    let (mut game, pos, source, target) = super::fixture();
    let mut input = super::tconstruct_source_casting_tank::CastingTank::new();
    assert!(input.tank.insert("minecraft:water", 3000));
    input.slots[0] = Some(horizon_items::items::ItemStack::new("minecraft:water_bucket", 1));
    game.world.blocks.insert(source, 3);
    game.world.block_entities.insert(source, BlockEntity::Chest(input.slots.to_vec()));
    game.world.tags.insert((source, "tconstruct:casting_tank".into()), input.to_nbt());
    game.world.blocks.insert(target, destination);
    game.world.block_entities.insert(target, BlockEntity::Chest(vec![]));
    let name = game.res.facts().get(destination).name;
    let (key, capacity) = super::tconstruct_source_faucet_tank::tank_kind(&name).unwrap();
    game.world.tags.insert((target, key.into()), Tank::new(capacity).to_nbt());
    (game, pos, source, target, key, capacity)
}

#[test]
fn casting_output_reserves_and_reprocesses_into_all_eight_tanks() {
    for destination in [1, 5, 6, 7, 8, 9, 10, 11] {
        let (mut game, pos, source, target, key, capacity) = output_fixture(destination);
        assert_eq!(casting_tank_to_tank(&mut game, pos, source, target, Faucet::new(), true), Some(true));
        let input = casting_tank_at(&game, source).unwrap();
        let output = Tank::from_nbt(&store::read(&game.world, target, key).unwrap(), capacity).unwrap();
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!((input.tank.amount, output.amount, faucet.pending.amount), (3910, 10, 80));
        assert!(input.slots[0].is_none());
        assert_eq!(input.slots[1].as_ref().unwrap().item.as_ref(), "minecraft:bucket");
        for _ in 0..9 {
            let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
            assert_eq!(casting_tank_to_tank(&mut game, pos, source, target, faucet, false), Some(true));
        }
        let input = casting_tank_at(&game, source).unwrap();
        let output = Tank::from_nbt(&store::read(&game.world, target, key).unwrap(), capacity).unwrap();
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!((input.tank.amount, output.amount, faucet.pending.amount), (3820, 100, 80));
    }
}

#[test]
fn casting_output_failed_save_and_stale_callbacks_preserve_inventory() {
    for fault in 0..4 {
        let (mut game, pos, source, target, _, _) = output_fixture(1);
        game.world.fail = fault == 0;
        game.switch_target = fault == 1;
        game.change_source_slot = fault == 2;
        if fault == 3 { game.world.chunks.clear(); }
        let before = game.world.tags.clone();
        assert_eq!(casting_tank_to_tank(&mut game, pos, source, target, Faucet::new(), true), Some(false));
        assert_eq!(game.world.tags, before);
        let BlockEntity::Chest(slots) = game.world.block_entities.get(&source).unwrap();
        assert_eq!(slots[0].as_ref().unwrap().item.as_ref(), if fault == 2 { "minecraft:stone" } else { "minecraft:water_bucket" });
        assert!(slots[1].is_none());
        assert_eq!((game.signals, game.slot_updates), (0, 0));
    }
}

#[test]
fn casting_output_rejects_unknown_destination_and_source_slots() {
    for target_fault in [true, false] {
        let (mut game, pos, source, target, key, _) = output_fixture(1);
        if target_fault {
            if let Tag::Compound(fields) = game.world.tags.get_mut(&(target, key.into())).unwrap() {
                fields.insert("future".into(), Tag::Int(1));
            }
        } else {
            let BlockEntity::Chest(slots) = game.world.block_entities.get_mut(&source).unwrap();
            slots[0] = None;
        }
        let before = game.world.tags.clone();
        assert_eq!(casting_tank_to_tank(&mut game, pos, source, target, Faucet::new(), true), Some(false));
        assert_eq!(game.world.tags, before);
    }
}

#[test]
fn casting_output_keeps_potion_identity_and_rejects_mixing_or_custom_tags() {
    for variant in 0..3 {
        let (mut game, pos, source, target, key, capacity) = output_fixture(1);
        let potion = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        let other = Tag::compound([("Potion", Tag::String("minecraft:healing".into()))]);
        let mut input = super::tconstruct_source_casting_tank::CastingTank::new();
        assert!(input.tank.insert_tagged("tconstruct:potion", Some(&potion), 250));
        if variant == 2 {
            if let Some(Tag::Compound(fields)) = &mut input.tank.tag { fields.insert("CustomPotionColor".into(), Tag::Int(123)); }
        }
        game.world.block_entities.insert(source, BlockEntity::Chest(input.slots.to_vec()));
        game.world.tags.insert((source, "tconstruct:casting_tank".into()), input.to_nbt());
        if variant == 1 {
            let mut output = Tank::new(capacity);
            assert!(output.insert_tagged("tconstruct:potion", Some(&other), 250));
            game.world.tags.insert((target, key.into()), output.to_nbt());
        }
        let before = game.world.tags.clone();
        let result = casting_tank_to_tank(&mut game, pos, source, target, Faucet::new(), true);
        if variant == 2 { assert_eq!(result, Some(false)); } else { assert_eq!(result, Some(true)); }
        if variant > 0 { assert_eq!(game.world.tags, before); }
        else {
            let output = Tank::from_nbt(&store::read(&game.world, target, key).unwrap(), capacity).unwrap();
            assert_eq!((output.amount, output.tag), (10, Some(potion)));
        }
    }
}

#[test]
fn casting_output_finishes_reserved_packet_before_waiting_on_refilled_tag() {
    let (mut game, pos, source, target, key, capacity) = output_fixture(1);
    let mut input = super::tconstruct_source_casting_tank::CastingTank::new();
    assert!(input.tank.insert("minecraft:water", 90));
    // Inject unsupported source metadata after reserving the ordinary packet,
    // modeling a refill without making assumptions about host component syntax.
    game.world.block_entities.insert(source, BlockEntity::Chest(input.slots.to_vec()));
    game.world.tags.insert((source, "tconstruct:casting_tank".into()), input.to_nbt());
    assert_eq!(casting_tank_to_tank(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let mut input = casting_tank_at(&game, source).unwrap();
    let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", Tag::Int(123))]);
    assert!(input.tank.insert_tagged("tconstruct:potion", Some(&tag), 250));
    game.world.tags.insert((source, "tconstruct:casting_tank".into()), input.to_nbt());
    for _ in 0..8 {
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(casting_tank_to_tank(&mut game, pos, source, target, faucet, false), Some(true));
    }
    let output = Tank::from_nbt(&store::read(&game.world, target, key).unwrap(), capacity).unwrap();
    assert_eq!(output.amount, 90);
    let mut faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    let before = game.world.tags.clone();
    assert_eq!(casting_tank_to_tank(&mut game, pos, source, target, faucet.clone(), false), Some(false));
    assert_eq!(game.world.tags, before);
    faucet.stop_after_packet = true;
    game.world.tags.insert((pos, "tconstruct:faucet".into()), faucet.to_nbt());
    assert_eq!(casting_tank_to_tank(&mut game, pos, source, target, faucet, false), Some(true));
    assert!(!Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap().active);
}

}
fn fixture()->(Game<'static>,Pos,Pos,Pos){use tconstruct_source_melter_state::{Tank,Faucet,Casting};let(p,s,t)=((0,1,0),(-1,1,0),(0,0,0));let mut tank=Tank::new(4000);tank.insert("x:copper",180);let tags=HashMap::from([((s,"tconstruct:fuel_tank".into()),tank.to_nbt()),((p,"tconstruct:faucet".into()),Faucet::new().to_nbt()),((t,"tconstruct:table".into()),Casting::new().to_nbt())]);(Game{res:Res,signals:0,slot_updates:0,switch_target:false,change_recipe:false,recipe_changed:false,change_target_slot:false,change_source_slot:false,world:World{blocks:HashMap::from([(s,1),(t,2)]),block_entities:HashMap::from([(s,containers::BlockEntity::Chest(vec![])),(t,containers::BlockEntity::Chest(vec![None,None]))]),chunks:HashMap::from([((-1,0),()),((0,0),())]),tags,fail:false},_life:std::marker::PhantomData},p,s,t)}
