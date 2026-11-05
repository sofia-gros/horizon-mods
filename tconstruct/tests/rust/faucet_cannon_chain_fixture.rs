extern crate self as horizon_client;
pub mod containers{pub enum BlockEntity{Chest(Vec<Option<horizon_items::items::ItemStack>>)}}
#[path="__MIXINS__/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path="__MIXINS__/source-faucet-tank.rs"] mod tconstruct_source_faucet_tank;
#[path="__MIXINS__/source-fluid-cannon.rs"] mod tconstruct_source_fluid_cannon;
use std::collections::HashMap;use horizon_world::nbt::Tag;use horizon_items::items::ItemStack;type Pos=(i32,i32,i32);
#[path="__MIXINS__/source-smeltery-fuel.rs"] mod tconstruct_source_smeltery_fuel;
#[path="__MIXINS__/source-smeltery-tank.rs"] mod tconstruct_source_smeltery_tank;
#[path="__MIXINS__/source-fluid-container.rs"] mod tconstruct_source_fluid_container;
#[path="__MIXINS__/source-fluid-display.rs"] mod tconstruct_source_fluid_display;
#[path="__PLUTO__"] mod pluto_stack_fluid;
struct Info{name:String}struct Facts;impl Facts{fn get(&self,id:usize)->Info{Info{name:match id{1=>"tconstruct:seared_fuel_tank",2=>"tconstruct:seared_fluid_cannon",_=>"minecraft:air"}.into()}}}struct Res;impl Res{fn facts(&self)->Facts{Facts}}
struct World{blocks:HashMap<Pos,usize>,block_entities:HashMap<Pos,containers::BlockEntity>,chunks:HashMap<(i32,i32),()>,tags:HashMap<(Pos,String),Tag>,fail:bool}impl World{fn block(&self,x:i32,y:i32,z:i32)->usize{*self.blocks.get(&(x,y,z)).unwrap_or(&0)}}
struct Game<'a>{res:Res,world:World,notified:bool,signals:Vec<Pos>,_life:std::marker::PhantomData<&'a()>}impl Game<'_>{fn ensure_datapacks(&mut self){}fn container_changed(&mut self,_:Pos){self.notified=true}}
mod tconstruct_source_melter_store{use super::*;pub(super) fn read(w:&World,p:Pos,k:&str)->Option<Tag>{w.tags.get(&(p,k.into())).cloned()}pub(super) fn write_many(w:&mut World,v:&[(Pos,&str,Tag)])->bool{if w.fail||v.iter().any(|(p,k,_)|!w.tags.contains_key(&(*p,(*k).into()))){return false}for(p,k,t)in v{w.tags.insert((*p,(*k).into()),t.clone());}true}}
mod tconstruct_source_machine_runtime{use super::*;use super::containers::BlockEntity;use super::tconstruct_source_melter_store as store;use super::tconstruct_source_melter_state::{Faucet,Tank};use super::tconstruct_source_fluid_cannon::Cannon;
pub(super) fn kind(n:&str)->Option<&'static str>{(n=="tconstruct:seared_fuel_tank").then_some("tconstruct:fuel_tank")}
fn notify_casting_comparator(game: &mut Game<'_>, pos: Pos, before: i32, after: i32) { if before != after { game.signals.push(pos); } }
// __PRODUCTION_ROUTE__
fn chain_fixture() -> (Game<'static>, Pos, Pos, Pos) {
    let (mut game, pos, source, target) = super::fixture();
    let mut input = Cannon::new(); assert!(input.tank.insert("minecraft:water", 180));
    let mut item = ItemStack::new("minecraft:stone", 32);
    item.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("keep", Tag::Int(7))])));
    input.item = Some(item);
    game.world.blocks.insert(source, 2);
    game.world.tags.remove(&(source, "tconstruct:fuel_tank".into()));
    game.world.tags.insert((source, "tconstruct:fluid_cannon".into()), input.to_nbt());
    game.world.block_entities.insert(source, BlockEntity::Chest(vec![input.item.clone()]));
    (game, pos, source, target)
}
#[test]
fn cannon_chain_preserves_both_items_and_first_and_later_packet_timing() {
    let (mut game, pos, source, target) = chain_fixture();
    let snapshots: Vec<_> = [source,target].into_iter().map(|at| {
        let BlockEntity::Chest(slots) = game.world.block_entities.get(&at).unwrap();
        (at, store::read(&game.world, at, "tconstruct:fluid_cannon").unwrap(), slots.clone())
    }).collect();
    assert_eq!(cannon_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let f = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((cannon_at(&game,source).unwrap().tank.amount,cannon_at(&game,target).unwrap().tank.amount,f.pending.amount),(90,10,80));
    assert_eq!(game.signals, vec![source,target]);
    for _ in 0..9 {
        let f = Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(cannon_to_cannon(&mut game,pos,source,target,f,false),Some(true));
    }
    let f = Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
    assert_eq!((cannon_at(&game,source).unwrap().tank.amount,cannon_at(&game,target).unwrap().tank.amount,f.pending.amount),(0,100,80));
    for (at, raw, live) in snapshots {
        let state = cannon_at(&game,at).unwrap();
        assert_eq!(store::read(&game.world,at,"tconstruct:fluid_cannon"),super::tconstruct_source_faucet_tank::patch_melter_tank(&raw,&state.tank));
        let BlockEntity::Chest(slots) = game.world.block_entities.get(&at).unwrap(); assert_eq!(*slots,live);
    }
}
#[test]
fn cannon_chain_failed_save_and_either_participants_invalid_state_wait() {
    for source_fault in [false,true] { for fault in 0..6 {
        let (mut game,pos,source,target) = chain_fixture(); let at=if source_fault {source}else{target};
        game.world.fail=fault==0;
        if fault==1 { if let Tag::Compound(fields)=game.world.tags.get_mut(&(at,"tconstruct:fluid_cannon".into())).unwrap(){fields.insert("future".into(),Tag::Int(7));} }
        if fault==2 {game.world.block_entities.remove(&at);}
        if fault==3 {let BlockEntity::Chest(slots)=game.world.block_entities.get_mut(&at).unwrap();slots[0]=None;}
        if fault==4 {game.world.tags.remove(&(at,"tconstruct:fluid_cannon".into()));}
        if fault==5 {game.world.chunks.remove(&(at.0>>4,at.2>>4));}
        let before=game.world.tags.clone();
        assert_eq!(cannon_to_cannon(&mut game,pos,source,target,Faucet::new(),true),Some(false));
        assert_eq!(game.world.tags,before);assert!(!game.notified);assert!(game.signals.is_empty());
    }}
    let (mut game,pos,source,_,)=chain_fixture();let before=game.world.tags.clone();
    assert_eq!(cannon_to_cannon(&mut game,pos,source,source,Faucet::new(),true),Some(false));assert_eq!(game.world.tags,before);
}
#[test]
fn cannon_chain_last_five_mb_and_exact_potion_identity() {
    let first=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);
    let second=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);
    for matching in [false,true] {
        let (mut game,pos,source,target)=chain_fixture();
        let mut input=cannon_at(&game,source).unwrap();input.tank=Tank::new(2000);assert!(input.tank.insert_tagged("tconstruct:potion",Some(&first),180));
        let mut output=cannon_at(&game,target).unwrap();assert!(output.tank.insert_tagged("tconstruct:potion",Some(if matching{&first}else{&second}),1995));
        game.world.tags.insert((source,"tconstruct:fluid_cannon".into()),input.to_nbt());game.world.tags.insert((target,"tconstruct:fluid_cannon".into()),output.to_nbt());
        assert_eq!(cannon_to_cannon(&mut game,pos,source,target,Faucet::new(),true),Some(true));
        assert_eq!(cannon_at(&game,source).unwrap().tank.amount,if matching{175}else{180});
        assert_eq!(cannon_at(&game,target).unwrap().tank.amount,if matching{2000}else{1995});
    }
}
#[test]
fn cannon_chain_packet_finishes_after_unsupported_refill_and_stops() {
    let (mut game,pos,source,target)=chain_fixture();
    assert_eq!(cannon_to_cannon(&mut game,pos,source,target,Faucet::new(),true),Some(true));
    let mut input=cannon_at(&game,source).unwrap();input.tank=Tank::new(2000);
    let tag=Tag::compound([("Potion",Tag::String("minecraft:healing".into())),("CustomPotionColor",Tag::Int(1))]);
    assert!(input.tank.insert_tagged("tconstruct:potion",Some(&tag),250));game.world.tags.insert((source,"tconstruct:fluid_cannon".into()),input.to_nbt());
    for _ in 0..8 {
        let f=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(cannon_to_cannon(&mut game,pos,source,target,f,false),Some(true));
    }
    assert_eq!(cannon_at(&game,target).unwrap().tank.amount,90);assert_eq!(cannon_at(&game,source).unwrap().tank.amount,250);
    let mut f=Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap();let before=game.world.tags.clone();
    assert_eq!(cannon_to_cannon(&mut game,pos,source,target,f.clone(),false),Some(false));assert_eq!(game.world.tags,before);
    f.stop_after_packet=true;game.world.tags.insert((pos,"tconstruct:faucet".into()),f.to_nbt());
    assert_eq!(cannon_to_cannon(&mut game,pos,source,target,f,false),Some(true));
    assert!(!Faucet::from_nbt(&store::read(&game.world,pos,"tconstruct:faucet").unwrap()).unwrap().active);
}
}
fn fixture()->(Game<'static>,Pos,Pos,Pos){use tconstruct_source_melter_state::{Tank,Faucet};use tconstruct_source_fluid_cannon::Cannon;let(p,s,t)=((0,1,0),(-1,1,0),(0,0,0));let mut source=Tank::new(4000);source.insert("minecraft:water",180);let mut output=Cannon::new();output.item=Some(ItemStack::new("minecraft:stone",16));let tags=HashMap::from([((s,"tconstruct:fuel_tank".into()),source.to_nbt()),((p,"tconstruct:faucet".into()),Faucet::new().to_nbt()),((t,"tconstruct:fluid_cannon".into()),output.to_nbt())]);let blocks=HashMap::from([(s,1),(t,2)]);let block_entities=HashMap::from([(s,containers::BlockEntity::Chest(vec![])),(t,containers::BlockEntity::Chest(vec![output.item.clone()]))]);(Game{res:Res,world:World{blocks,block_entities,chunks:HashMap::from([((-1,0),()),((0,0),())]),tags,fail:false},notified:false,signals:vec![],_life:std::marker::PhantomData},p,s,t)}
