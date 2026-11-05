extern crate self as horizon_client;
pub mod containers{pub enum BlockEntity{Chest(Vec<Option<horizon_items::items::ItemStack>>)}}
#[path="__MIXINS__/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path="__MIXINS__/source-faucet-tank.rs"] mod tconstruct_source_faucet_tank;
#[path="__MIXINS__/source-fluid-cannon.rs"] mod tconstruct_source_fluid_cannon;
#[path="__MIXINS__/source-casting-tank.rs"] mod tconstruct_source_casting_tank;
#[path="__MIXINS__/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path="__MIXINS__/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path="__MIXINS__/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path="__MIXINS__/source-fluid-display.rs"] mod tconstruct_source_fluid_display;
#[path="__PLUTO__"] mod pluto_stack_fluid;
use std::collections::HashMap;use horizon_world::nbt::Tag;use horizon_items::items::ItemStack;type Pos=(i32,i32,i32);
struct Info{name:String}struct Facts;impl Facts{fn get(&self,id:usize)->Info{Info{name:match id{1=>"tconstruct:seared_fuel_tank",2=>"tconstruct:seared_fluid_cannon",3=>"tconstruct:seared_casting_tank",4=>"tconstruct:seared_melter",5=>"tconstruct:seared_lantern",6=>"tconstruct:scorched_lantern",_=>"minecraft:air"}.into()}}}struct Res;impl Res{fn facts(&self)->Facts{Facts}}
struct World{blocks:HashMap<Pos,usize>,block_entities:HashMap<Pos,containers::BlockEntity>,chunks:HashMap<(i32,i32),()>,tags:HashMap<(Pos,String),Tag>,fail:bool}impl World{fn block(&self,x:i32,y:i32,z:i32)->usize{*self.blocks.get(&(x,y,z)).unwrap_or(&0)}}
struct Game<'a>{res:Res,world:World,notified:bool,process_fault:u8,_life:std::marker::PhantomData<&'a()>}impl Game<'_>{fn ensure_datapacks(&mut self){}fn container_changed(&mut self,_:Pos){self.notified=true}}
mod tconstruct_source_melter_store{use super::*;pub(super) fn read(w:&World,p:Pos,k:&str)->Option<Tag>{w.tags.get(&(p,k.into())).cloned()}pub(super) fn write_many(w:&mut World,v:&[(Pos,&str,Tag)])->bool{if w.fail||v.iter().any(|(p,k,_)|!w.tags.contains_key(&(*p,(*k).into()))){return false}for(p,k,t)in v{w.tags.insert((*p,(*k).into()),t.clone());}true}}
mod tconstruct_source_machine_runtime{use super::*;use super::containers::BlockEntity;use super::tconstruct_source_melter_store as store;use super::tconstruct_source_melter_state::{Faucet,Tank};use super::tconstruct_source_fluid_cannon::Cannon;
pub(super) fn kind(n:&str)->Option<&'static str>{(n=="tconstruct:seared_fuel_tank").then_some("tconstruct:fuel_tank")}
fn cannon_at(g:&Game<'_>,p:Pos)->Option<Cannon>{let mut c=Cannon::from_nbt(&store::read(&g.world,p,"tconstruct:fluid_cannon")?)?;let BlockEntity::Chest(slots)=g.world.block_entities.get(&p)?;c.item=slots.first()?.clone();c.lossless().then_some(c)}
fn notify_casting_comparator(_: &mut Game<'_>,_:Pos,_:i32,_:i32){}
fn casting_tank_at(game: &Game<'_>, pos: Pos) -> Option<super::tconstruct_source_casting_tank::CastingTank> {
    let mut state = super::tconstruct_source_casting_tank::CastingTank::from_nbt(&store::read(&game.world, pos, "tconstruct:casting_tank")?)?;
    let BlockEntity::Chest(slots) = game.world.block_entities.get(&pos)?;
    if slots.len() < 2 { return None; }
    state.slots = [slots[0].clone(), slots[1].clone()];
    state.lossless().then_some(state)
}
fn process_casting_tank(game: &mut Game<'_>, state: &mut super::tconstruct_source_casting_tank::CastingTank) -> bool {
    let changed = state.process(&|_| true, &|_| true, None, false);
    match game.process_fault {
        1 => { game.world.blocks.insert((0,0,0), 0); },
        2 => { let BlockEntity::Chest(slots) = game.world.block_entities.get_mut(&(-1,1,0)).unwrap(); slots[0] = Some(ItemStack::new("minecraft:stone",1)); },
        3 => { let BlockEntity::Chest(slots) = game.world.block_entities.get_mut(&(0,0,0)).unwrap(); slots[0] = None; },
        4 => { game.world.blocks.insert((0,1,0), 1); },
        5 => { game.world.chunks.clear(); },
        6 => { game.world.blocks.insert((-1,1,0), 1); },
        _ => {},
    }
    changed
}
// __PRODUCTION_ROUTE__
#[test]fn transfers_and_failed_write_preserves_everything(){let(mut g,p,s,t)=super::fixture();assert_eq!(tank_to_cannon(&mut g,p,s,t,Faucet::new(),true),Some(true));let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let c=Cannon::from_nbt(&store::read(&g.world,t,"tconstruct:fluid_cannon").unwrap()).unwrap();assert_eq!((f.pending.amount,c.tank.amount),(80,10));assert_eq!(c.item.unwrap().count,16);let raw=g.world.tags.clone();g.world.fail=true;assert_eq!(tank_to_cannon(&mut g,p,s,t,f,false),Some(false));assert_eq!(raw,g.world.tags);let BlockEntity::Chest(slots)=g.world.block_entities.get(&t).unwrap();assert_eq!(slots[0].as_ref().unwrap().count,16);g.world.fail=false;for _ in 0..9{let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert_eq!(tank_to_cannon(&mut g,p,s,t,f,false),Some(true));let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let out=Cannon::from_nbt(&store::read(&g.world,t,"tconstruct:fluid_cannon").unwrap()).unwrap();let input=super::tconstruct_source_melter_state::Tank::from_nbt(&store::read(&g.world,s,"tconstruct:fuel_tank").unwrap(),4000).unwrap();assert_eq!(input.amount+f.pending.amount+out.tank.amount,180);assert_eq!(out.item.unwrap().count,16);}let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let out=Cannon::from_nbt(&store::read(&g.world,t,"tconstruct:fluid_cannon").unwrap()).unwrap();assert_eq!((f.pending.amount,out.tank.amount),(80,100));}
#[test]fn full_or_incompatible_tank_and_unloaded_wait(){use super::tconstruct_source_melter_state::Tank;let(mut g,p,s,t)=super::fixture();let mut c=Cannon::from_nbt(&store::read(&g.world,t,"tconstruct:fluid_cannon").unwrap()).unwrap();c.tank=Tank::new(2000);assert!(c.tank.insert("minecraft:lava",100));g.world.tags.insert((t,"tconstruct:fluid_cannon".into()),c.to_nbt());let before=g.world.tags.clone();assert_eq!(tank_to_cannon(&mut g,p,s,t,Faucet::new(),true),Some(true));assert_eq!(before,g.world.tags);c.tank=Tank::new(2000);assert!(c.tank.insert("minecraft:water",2000));g.world.tags.insert((t,"tconstruct:fluid_cannon".into()),c.to_nbt());let before=g.world.tags.clone();assert_eq!(tank_to_cannon(&mut g,p,s,t,Faucet::new(),true),Some(true));assert_eq!(before,g.world.tags);g.world.chunks.clear();assert_eq!(tank_to_cannon(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}
#[test]fn unknown_and_stale_live_item_refuse(){let(mut g,p,s,t)=super::fixture();if let Tag::Compound(raw)=g.world.tags.get_mut(&(t,"tconstruct:fluid_cannon".into())).unwrap(){raw.insert("unknown".into(),Tag::Int(1));}let old=g.world.tags.clone();assert_eq!(tank_to_cannon(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(old,g.world.tags);let(mut g,p,s,t)=super::fixture();let BlockEntity::Chest(slots)=g.world.block_entities.get_mut(&t).unwrap();slots[0]=None;let old=g.world.tags.clone();assert_eq!(tank_to_cannon(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(old,g.world.tags);}


fn casting_source_fixture() -> (Game<'static>, Pos, Pos, Pos) {
    let (mut game, pos, source, target) = super::fixture();
    let mut input = super::tconstruct_source_casting_tank::CastingTank::new();
    assert!(input.tank.insert("minecraft:water", 3000));
    input.slots[0] = Some(ItemStack::new("minecraft:water_bucket", 1));
    game.world.blocks.insert(source, 3);
    game.world.block_entities.insert(source, BlockEntity::Chest(input.slots.to_vec()));
    game.world.tags.insert((source, "tconstruct:casting_tank".into()), input.to_nbt());
    (game, pos, source, target)
}

#[test]
fn casting_source_cannon_reprocesses_container_without_changing_cannon_item() {
    let (mut game, pos, source, target) = casting_source_fixture();
    assert_eq!(tank_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let input = casting_tank_at(&game, source).unwrap();
    let cannon = cannon_at(&game, target).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((input.tank.amount, cannon.tank.amount, faucet.pending.amount), (3910, 10, 80));
    assert!(input.slots[0].is_none());
    assert_eq!(input.slots[1].as_ref().unwrap().item.as_ref(), "minecraft:bucket");
    assert_eq!(cannon.item.as_ref().unwrap().count, 16);
    assert!(game.notified);
    for _ in 0..9 {
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(tank_to_cannon(&mut game, pos, source, target, faucet, false), Some(true));
    }
    let input = casting_tank_at(&game, source).unwrap();
    let cannon = cannon_at(&game, target).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((input.tank.amount, cannon.tank.amount, faucet.pending.amount), (3820, 100, 80));
    assert_eq!(cannon.item.as_ref().unwrap().count, 16);
}

#[test]
fn casting_source_cannon_failed_save_and_stale_callbacks_do_not_commit() {
    for fault in 0..7 {
        let (mut game, pos, source, target) = casting_source_fixture();
        game.world.fail = fault == 0;
        game.process_fault = fault;
        let before = game.world.tags.clone();
        assert_eq!(tank_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(false));
        assert_eq!(game.world.tags, before);
        assert!(!game.notified);
        let BlockEntity::Chest(slots) = game.world.block_entities.get(&source).unwrap();
        assert_eq!(slots[0].as_ref().unwrap().item.as_ref(), if fault == 2 { "minecraft:stone" } else { "minecraft:water_bucket" });
        assert!(slots[1].is_none());
    }
}

#[test]
fn casting_source_cannon_opaque_stale_and_tagged_source_wait() {
    for fault in 0..3 {
        let (mut game, pos, source, target) = casting_source_fixture();
        if fault == 0 {
            if let Tag::Compound(fields) = game.world.tags.get_mut(&(source, "tconstruct:casting_tank".into())).unwrap() {
                fields.insert("future".into(), Tag::Int(1));
            }
        } else if fault == 1 {
            let BlockEntity::Chest(slots) = game.world.block_entities.get_mut(&source).unwrap(); slots[0] = None;
        } else {
            let mut input = casting_tank_at(&game, source).unwrap();
            input.tank = Tank::new(4000);
            let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", Tag::Int(123))]);
            assert!(input.tank.insert_tagged("tconstruct:potion", Some(&tag), 250));
            game.world.tags.insert((source, "tconstruct:casting_tank".into()), input.to_nbt());
        }
        let before = game.world.tags.clone();
        assert_eq!(tank_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(false));
        assert_eq!(game.world.tags, before);
    }
}

#[test]
fn casting_source_cannon_finishes_untagged_packet_before_refilled_tag() {
    let (mut game, pos, source, target) = casting_source_fixture();
    let mut input = super::tconstruct_source_casting_tank::CastingTank::new();
    assert!(input.tank.insert("minecraft:water", 90));
    input.slots[0] = Some(ItemStack::new("minecraft:potion", 1).with_potion("minecraft:swiftness"));
    game.world.block_entities.insert(source, BlockEntity::Chest(input.slots.to_vec()));
    game.world.tags.insert((source, "tconstruct:casting_tank".into()), input.to_nbt());
    assert_eq!(tank_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let mut refilled = casting_tank_at(&game, source).unwrap();
    assert!(refilled.tank.tag.is_some());
    if let Some(Tag::Compound(fields)) = &mut refilled.tank.tag { fields.insert("CustomPotionColor".into(), Tag::Int(123)); }
    game.world.tags.insert((source, "tconstruct:casting_tank".into()), refilled.to_nbt());
    for _ in 0..8 {
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(tank_to_cannon(&mut game, pos, source, target, faucet, false), Some(true));
    }
    assert_eq!(cannon_at(&game, target).unwrap().tank.amount, 90);
    let mut faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    let before = game.world.tags.clone();
    assert_eq!(tank_to_cannon(&mut game, pos, source, target, faucet.clone(), false), Some(false));
    assert_eq!(game.world.tags, before);
    faucet.stop_after_packet = true;
    game.world.tags.insert((pos, "tconstruct:faucet".into()), faucet.to_nbt());
    assert_eq!(tank_to_cannon(&mut game, pos, source, target, faucet, false), Some(true));
    assert!(!Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap().active);
}


fn potion_cannon_fixture(portable: bool, tag: Tag) -> (Game<'static>, Pos, Pos, Pos) {
    let (mut game, pos, source, target) = if portable { casting_source_fixture() } else { super::fixture() };
    let mut tank = Tank::new(4000);
    assert!(tank.insert_tagged("tconstruct:potion", Some(&tag), 250));
    if portable {
        let mut input = super::tconstruct_source_casting_tank::CastingTank::new();
        input.tank = tank;
        game.world.tags.insert((source, "tconstruct:casting_tank".into()), input.to_nbt());
        game.world.block_entities.insert(source, BlockEntity::Chest(input.slots.to_vec()));
    } else { game.world.tags.insert((source, "tconstruct:fuel_tank".into()), tank.to_nbt()); }
    (game, pos, source, target)
}

#[test]
fn cannon_base_potion_packets_keep_identity_from_both_source_kinds() {
    for portable in [false, true] {
        let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        let (mut game, pos, source, target) = potion_cannon_fixture(portable, tag.clone());
        assert_eq!(tank_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(true));
        for _ in 0..30 {
            let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
            let input = if portable { casting_tank_at(&game, source).unwrap().tank }
                else { Tank::from_nbt(&store::read(&game.world, source, "tconstruct:fuel_tank").unwrap(), 4000).unwrap() };
            let output = cannon_at(&game, target).unwrap();
            assert_eq!(input.amount + faucet.pending.amount + output.tank.amount, 250);
            for tank in [&input, &faucet.pending, &output.tank] {
                if tank.amount > 0 { assert_eq!(tank.tag.as_ref(), Some(&tag)); }
            }
            assert_eq!(output.item.as_ref().unwrap().count, 16);
            if output.tank.amount == 250 { break; }
            assert_eq!(tank_to_cannon(&mut game, pos, source, target, faucet, false), Some(true));
        }
        assert_eq!(cannon_at(&game, target).unwrap().tank.amount, 250);
    }
}

#[test]
fn cannon_potion_mismatch_and_failed_commit_leave_every_saved_state_unchanged() {
    for portable in [false, true] {
        for mismatch in [false, true] {
            let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
            let (mut game, pos, source, target) = potion_cannon_fixture(portable, tag);
            if mismatch {
                let mut output = cannon_at(&game, target).unwrap();
                let other = Tag::compound([("Potion", Tag::String("minecraft:healing".into()))]);
                assert!(output.tank.insert_tagged("tconstruct:potion", Some(&other), 10));
                game.world.tags.insert((target, "tconstruct:fluid_cannon".into()), output.to_nbt());
            } else { game.world.fail = true; }
            let before = game.world.tags.clone();
            assert_eq!(tank_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(mismatch));
            assert_eq!(game.world.tags, before);
            assert!(!game.notified);
        }
    }
}

#[test]
fn cannon_rejects_custom_unknown_and_invalid_potion_metadata() {
    let tags = [
        Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", Tag::Int(123))]),
        Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionEffects", Tag::List(vec![]))]),
        Tag::compound([("Potion", Tag::String("unknown:custom".into()))]),
        Tag::compound([("Potion", Tag::String("minecraft:strong_awkward".into()))]),
        Tag::compound([("Potion", Tag::String("swiftness".into()))]),
        Tag::compound([("Potion", Tag::String(horizon_world::nbt::JavaString(vec![0xd800])))]),
    ];
    for portable in [false, true] {
        for tag in &tags {
            let (mut game, pos, source, target) = potion_cannon_fixture(portable, tag.clone());
            let before = game.world.tags.clone();
            assert_eq!(tank_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(false));
            assert_eq!(game.world.tags, before);
        }
    }
}

fn melter_cannon_fixture() -> (Game<'static>, Pos, Pos, Pos) {
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
    game.world.blocks.insert(source, 4);
    game.world.block_entities.insert(source, BlockEntity::Chest(melter.slots.to_vec()));
    game.world.tags.insert((source, "tconstruct:melter".into()), raw);
    (game, pos, source, target)
}

#[test]
fn melter_cannon_packets_preserve_raw_inventory_fuel_and_progress() {
    let (mut game, pos, source, target) = melter_cannon_fixture();
    let before = store::read(&game.world, source, "tconstruct:melter").unwrap();
    let BlockEntity::Chest(slots) = game.world.block_entities.get(&source).unwrap();
    let before_live = slots.clone();
    let cannon_item = cannon_at(&game, target).unwrap().item;
    assert_eq!(tank_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let raw = store::read(&game.world, source, "tconstruct:melter").unwrap();
    let input = super::tconstruct_source_melter_state::Melter::from_nbt(&raw).unwrap();
    assert_eq!(raw, super::tconstruct_source_faucet_tank::patch_melter_tank(&before, &input.tank).unwrap());
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((input.tank.amount, cannon_at(&game, target).unwrap().tank.amount, faucet.pending.amount), (90,10,80));
    assert!(game.notified);
    for _ in 0..9 {
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(tank_to_cannon(&mut game, pos, source, target, faucet, false), Some(true));
    }
    let input = super::tconstruct_source_melter_state::Melter::from_nbt(&store::read(&game.world, source, "tconstruct:melter").unwrap()).unwrap();
    let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((input.tank.amount, cannon_at(&game, target).unwrap().tank.amount, faucet.pending.amount), (0,100,80));
    assert_eq!(store::read(&game.world, source, "tconstruct:melter").unwrap(), super::tconstruct_source_faucet_tank::patch_melter_tank(&before, &input.tank).unwrap());
    let BlockEntity::Chest(slots) = game.world.block_entities.get(&source).unwrap();
    assert_eq!(*slots, before_live);
    assert_eq!(cannon_at(&game, target).unwrap().item, cannon_item);
}

#[test]
fn melter_cannon_failed_save_unknown_tank_missing_source_and_opaque_target_wait() {
    for fault in 0..4 {
        let (mut game, pos, source, target) = melter_cannon_fixture();
        game.world.fail = fault == 0;
        if fault == 1 {
            if let Tag::Compound(fields) = game.world.tags.get_mut(&(source, "tconstruct:melter".into())).unwrap() {
                if let Some(Tag::Compound(tank)) = fields.get_mut(&"tank".into()) { tank.insert("future".into(), Tag::Int(1)); }
            }
        }
        if fault == 2 { game.world.block_entities.remove(&source); }
        if fault == 3 {
            if let Tag::Compound(fields) = game.world.tags.get_mut(&(target, "tconstruct:fluid_cannon".into())).unwrap() { fields.insert("future".into(), Tag::Int(1)); }
        }
        let before = game.world.tags.clone();
        assert_eq!(tank_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(false));
        assert_eq!(game.world.tags, before);
        assert!(!game.notified);
    }
}

#[test]
fn melter_cannon_base_potion_identity_and_unsupported_metadata() {
    for custom in [false, true] {
        let (mut game, pos, source, target) = melter_cannon_fixture();
        let mut tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
        if custom { if let Tag::Compound(fields) = &mut tag { fields.insert("CustomPotionColor".into(), Tag::Int(123)); } }
        let raw = store::read(&game.world, source, "tconstruct:melter").unwrap();
        let mut tank = Tank::new(super::tconstruct_source_melter_state::MELTER_CAPACITY);
        assert!(tank.insert_tagged("tconstruct:potion", Some(&tag), 250));
        game.world.tags.insert((source, "tconstruct:melter".into()), super::tconstruct_source_faucet_tank::patch_melter_tank(&raw, &tank).unwrap());
        let before = game.world.tags.clone();
        assert_eq!(tank_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(!custom));
        if custom { assert_eq!(game.world.tags, before); }
        else {
            let output = cannon_at(&game, target).unwrap();
            let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
            assert_eq!((output.tank.amount, output.tank.tag), (10,Some(tag.clone())));
            assert_eq!((faucet.pending.amount, faucet.pending.tag), (80,Some(tag)));
        }
    }
}


fn lantern_cannon_fixture(id:usize)->(Game<'static>,Pos,Pos,Pos) {
    let (mut game,pos,source,target)=super::fixture();
    game.world.blocks.insert(source,id); game.world.tags.remove(&(source,"tconstruct:fuel_tank".into()));
    let mut tank=Tank::new(50); assert!(tank.insert("minecraft:water",50));
    game.world.tags.insert((source,"tconstruct:lantern".into()),tank.to_nbt());
    (game,pos,source,target)
}

#[test]
fn both_lanterns_supply_cannon_preserving_its_item_and_immediate_packets() {
    for id in [5,6] {
        let (mut game,pos,source,target)=lantern_cannon_fixture(id);
        let item=cannon_at(&game,target).unwrap().item;
        assert_eq!(tank_to_cannon(&mut game,pos,source,target,Faucet::new(),true),Some(true));
        let input=Tank::from_nbt(&store::read(&game.world,source,"tconstruct:lantern").unwrap(),50).unwrap();
        let faucet=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
        assert_eq!((input.amount,cannon_at(&game,target).unwrap().tank.amount,faucet.pending.amount),(0,10,40));
        for _ in 0..4 {
            let faucet=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
            assert_eq!(tank_to_cannon(&mut game,pos,source,target,faucet,false),Some(true));
        }
        let mut refill=Tank::new(50); assert!(refill.insert("minecraft:water",50)); game.world.tags.insert((source,"tconstruct:lantern".into()),refill.to_nbt());
        let faucet=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(faucet.pending.amount,0); assert_eq!(tank_to_cannon(&mut game,pos,source,target,faucet,false),Some(true));
        let faucet=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
        assert_eq!((cannon_at(&game,target).unwrap().tank.amount,faucet.pending.amount),(60,40));
        assert_eq!(cannon_at(&game,target).unwrap().item,item);
        let BlockEntity::Chest(slots)=game.world.block_entities.get(&target).unwrap(); assert_eq!(slots[0],item);
    }
}

#[test]
fn lantern_cannon_opaque_missing_stale_unloaded_and_failed_saves_wait() {
    for fault in 0..6 {
        let (mut game,pos,source,target)=lantern_cannon_fixture(5);
        match fault {
            0=>game.world.fail=true,
            1=>{if let Tag::Compound(fields)=game.world.tags.get_mut(&(source,"tconstruct:lantern".into())).unwrap(){fields.insert("future".into(),Tag::Int(7));}},
            2=>{game.world.block_entities.remove(&source);},
            3=>{if let Tag::Compound(fields)=game.world.tags.get_mut(&(target,"tconstruct:fluid_cannon".into())).unwrap(){fields.insert("future".into(),Tag::Int(7));}},
            4=>{let BlockEntity::Chest(slots)=game.world.block_entities.get_mut(&target).unwrap();slots[0]=None;},
            _=>game.world.chunks.clear(),
        }
        let before=game.world.tags.clone(); assert_eq!(tank_to_cannon(&mut game,pos,source,target,Faucet::new(),true),Some(false));
        assert_eq!(game.world.tags,before); assert!(!game.notified);
    }
}

#[test]
fn lantern_cannon_potion_identity_refill_and_remaining_capacity() {
    for custom in [false,true] {
        let (mut game,pos,source,target)=lantern_cannon_fixture(6);
        let mut tag=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);
        if custom {if let Tag::Compound(fields)=&mut tag {fields.insert("CustomPotionColor".into(),Tag::Int(7));}}
        let mut tank=Tank::new(50);assert!(tank.insert_tagged("tconstruct:potion",Some(&tag),50));game.world.tags.insert((source,"tconstruct:lantern".into()),tank.to_nbt());
        let before=game.world.tags.clone();assert_eq!(tank_to_cannon(&mut game,pos,source,target,Faucet::new(),true),Some(!custom));
        if custom {assert_eq!(game.world.tags,before);} else {
            let second=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);
            let mut refill=Tank::new(50);assert!(refill.insert_tagged("tconstruct:potion",Some(&second),50));game.world.tags.insert((source,"tconstruct:lantern".into()),refill.to_nbt());
            for _ in 0..5 {let faucet=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();assert_eq!(tank_to_cannon(&mut game,pos,source,target,faucet,false),Some(true));}
            let input=Tank::from_nbt(&store::read(&game.world,source,"tconstruct:lantern").unwrap(),50).unwrap();let output=cannon_at(&game,target).unwrap();
            assert_eq!((input.amount,input.tag),(50,Some(second)));assert_eq!((output.tank.amount,output.tank.tag),(50,Some(tag)));
        }
    }
    let (mut game,pos,source,target)=lantern_cannon_fixture(5);let mut output=cannon_at(&game,target).unwrap();assert!(output.tank.insert("minecraft:water",1995));game.world.tags.insert((target,"tconstruct:fluid_cannon".into()),output.to_nbt());
    assert_eq!(tank_to_cannon(&mut game,pos,source,target,Faucet::new(),true),Some(true));
    let input=Tank::from_nbt(&store::read(&game.world,source,"tconstruct:lantern").unwrap(),50).unwrap();let faucet=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((input.amount,cannon_at(&game,target).unwrap().tank.amount,faucet.pending.amount),(45,2000,0));
}

}
fn fixture()->(Game<'static>,Pos,Pos,Pos){use tconstruct_source_melter_state::{Tank,Faucet};use tconstruct_source_fluid_cannon::Cannon;let(p,s,t)=((0,1,0),(-1,1,0),(0,0,0));let mut source=Tank::new(4000);source.insert("minecraft:water",180);let mut output=Cannon::new();output.item=Some(ItemStack::new("minecraft:stone",16));let tags=HashMap::from([((s,"tconstruct:fuel_tank".into()),source.to_nbt()),((p,"tconstruct:faucet".into()),Faucet::new().to_nbt()),((t,"tconstruct:fluid_cannon".into()),output.to_nbt())]);let blocks=HashMap::from([(s,1),(t,2)]);let block_entities=HashMap::from([(s,containers::BlockEntity::Chest(vec![])),(t,containers::BlockEntity::Chest(vec![output.item.clone()]))]);(Game{res:Res,world:World{blocks,block_entities,chunks:HashMap::from([((-1,0),()),((0,0),())]),tags,fail:false},notified:false,process_fault:0,_life:std::marker::PhantomData},p,s,t)}
