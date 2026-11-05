extern crate self as horizon_client;
pub mod containers { pub enum BlockEntity { Chest(Vec<Option<horizon_items::items::ItemStack>>) } }
#[path="__MIXINS__/source-fluid-cannon.rs"] mod tconstruct_source_fluid_cannon;
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
struct Info{name:String} struct Facts; impl Facts{fn get(&self,id:usize)->Info{Info{name:match id{1=>"tconstruct:seared_drain",2=>"tconstruct:seared_fuel_tank",3=>"tconstruct:smeltery_controller",4=>"tconstruct:seared_duct",5=>"tconstruct:scorched_drain",6=>"tconstruct:foundry_controller",7=>"tconstruct:seared_fluid_cannon",_=>"minecraft:air"}.into()}}} struct Res;impl Res{fn facts(&self)->Facts{Facts}}
struct Chunks{loaded:HashMap<(i32,i32),()>,owners:HashMap<Pos,Pos>} impl Chunks{fn contains_key(&self,p:&(i32,i32))->bool{self.loaded.contains_key(p)}fn __horizon_mixins_tconstruct_smeltery_owners(&self)->&HashMap<Pos,Pos>{&self.owners}}
struct World{blocks:HashMap<Pos,usize>,block_entities:HashMap<Pos,containers::BlockEntity>,chunks:Chunks,tags:HashMap<(Pos,String),Tag>,fail:bool,formed:bool} impl World{fn block(&self,x:i32,y:i32,z:i32)->usize{*self.blocks.get(&(x,y,z)).unwrap_or(&0)}}
struct Game<'a>{res:Res,remote:Option<()>,lan:Option<()>,lan_acting_uuid:Option<()>,world:World,signals:usize,notifications:usize,_life:std::marker::PhantomData<&'a()>}
impl Game<'_> { fn container_changed(&mut self, _: Pos) { self.notifications += 1; } }
mod tconstruct_source_smeltery_world{use super::*;pub(super) fn detect(g:&Game<'_>,_:Pos)->Option<([i32;3],[i32;3],u32)>{g.world.formed.then_some(([-2,-1,-1],[0,1,1],4000))}}
mod tconstruct_source_faucet_tank{pub(super) fn tank_kind(n:&str)->Option<(&'static str,u32)>{(n=="tconstruct:seared_fuel_tank").then_some(("tconstruct:fuel_tank",4000))}}
mod tconstruct_source_melter_store{use super::*;pub(super) fn read(w:&World,p:Pos,k:&str)->Option<Tag>{w.tags.get(&(p,k.into())).cloned()}pub(super) fn write_many(w:&mut World,v:&[(Pos,&str,Tag)])->bool{if w.fail||v.iter().any(|(p,k,_)|!w.tags.contains_key(&(*p,(*k).into()))){return false}for(p,k,t)in v{w.tags.insert((*p,(*k).into()),t.clone());}true}}
mod runtime{
use super::*;use super::tconstruct_source_melter_store as store;use super::tconstruct_source_smeltery_state::Smeltery;use super::tconstruct_source_melter_state::{Faucet,Tank};use super::containers::BlockEntity;
fn notify_casting_comparator(game: &mut Game<'_>, _: Pos, before: i32, after: i32) { if before != after { game.signals += 1; } }
// __SMELTERY_HELPERS__
// __PRODUCTION_ROUTE__
#[test]fn reserves_atomically_and_preserves_opaque_controller(){let(mut g,p,s,t,c)=super::fixture();let before=g.world.tags.get(&(c,"tconstruct:smeltery".into())).unwrap().get("heat_codes").cloned();assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(true));let raw=g.world.tags.get(&(c,"tconstruct:smeltery".into())).unwrap();assert_eq!(raw.get("heat_codes").cloned(),before);let state=Smeltery::from_nbt(raw).unwrap();let out=Tank::from_nbt(&store::read(&g.world,t,"tconstruct:fuel_tank").unwrap(),4000).unwrap();let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert_eq!((state.tank.total(),out.amount,f.pending.amount),(90,10,80));let old=g.world.tags.clone();g.world.fail=true;assert_eq!(smeltery_to_tank(&mut g,p,s,t,f,false),Some(false));assert_eq!(old,g.world.tags);}
#[test]fn tagged_fluid_and_unloaded_controller_wait(){let(mut g,p,s,t,c)=super::fixture();let raw=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap();let mut state=Smeltery::from_nbt(raw).unwrap();state.tank.fluids[0].tag=Some(Tag::compound([("test",Tag::Int(1))]));if let Tag::Compound(fields)=raw {fields.insert("tank".into(),state.tank.to_nbt());}let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);g.world.chunks.loaded.remove(&(-1,-1));assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}
#[test]fn potion_layers_conserve_identity_and_rollback(){let(mut g,p,s,t,c)=super::fixture();let first=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into()))]);let second=Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]);let raw=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap();let mut state=Smeltery::from_nbt(raw).unwrap();state.tank.fluids.clear();assert!(state.tank.insert_tagged("tconstruct:potion",Some(&first),180));assert!(state.tank.insert_tagged("tconstruct:potion",Some(&second),90));if let Tag::Compound(fields)=raw{fields.insert("tank".into(),state.tank.to_nbt());}g.lan=Some(());assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(true));let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let out=Tank::from_nbt(&store::read(&g.world,t,"tconstruct:fuel_tank").unwrap(),4000).unwrap();let state=Smeltery::from_nbt(&store::read(&g.world,c,"tconstruct:smeltery").unwrap()).unwrap();assert_eq!(state.tank.amount_tagged("tconstruct:potion",Some(&first)),90);assert_eq!(state.tank.amount_tagged("tconstruct:potion",Some(&second)),90);assert_eq!((f.pending.amount,out.amount),(80,10));assert_eq!(f.pending.tag,Some(first.clone()));assert_eq!(out.tag,Some(first));let before=g.world.tags.clone();g.world.fail=true;assert_eq!(smeltery_to_tank(&mut g,p,s,t,f,false),Some(false));assert_eq!(before,g.world.tags);g.world.fail=false;for _ in 0..25{let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();if !f.active{break}assert_eq!(smeltery_to_tank(&mut g,p,s,t,f,false),Some(true));}let state=Smeltery::from_nbt(&store::read(&g.world,c,"tconstruct:smeltery").unwrap()).unwrap();let out=Tank::from_nbt(&store::read(&g.world,t,"tconstruct:fuel_tank").unwrap(),4000).unwrap();let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert_eq!(state.tank.amount_tagged("tconstruct:potion",Some(&second)),90);assert_eq!(out.amount,180);assert_eq!(f.pending.amount,0);assert!(!f.active);}
#[test]fn potion_custom_effects_pass_only_nonnetwork(){let(mut g,p,s,t,c)=super::fixture();let tag=Tag::compound([("Potion",Tag::String("minecraft:swiftness".into())),("CustomPotionColor",Tag::Int(0x123456)),("CustomPotionEffects",Tag::List(vec![Tag::compound([("Id",Tag::Byte(1)),("Duration",Tag::Int(120))])]))]);let raw=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap();let mut state=Smeltery::from_nbt(raw).unwrap();state.tank.fluids.clear();assert!(state.tank.insert_tagged("tconstruct:potion",Some(&tag),180));if let Tag::Compound(fields)=raw{fields.insert("tank".into(),state.tank.to_nbt());}assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(true));let out=Tank::from_nbt(&store::read(&g.world,t,"tconstruct:fuel_tank").unwrap(),4000).unwrap();assert_eq!(out.tag,Some(tag.clone()));let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();assert_eq!(f.pending.tag,Some(tag));g.lan=Some(());let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,f,false),Some(false));assert_eq!(before,g.world.tags);}
#[test]fn potion_network_complex_and_invalid_refuse(){for tag in [Tag::compound([("Potion",Tag::String("minecraft:swiftness".into())),("CustomPotionColor",Tag::Int(3))]),Tag::compound([("Potion",Tag::String(horizon_world::nbt::JavaString(vec![0xd800])))])]{let(mut g,p,s,t,c)=super::fixture();let raw=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap();let mut state=Smeltery::from_nbt(raw).unwrap();state.tank.fluids.clear();assert!(state.tank.insert_tagged("tconstruct:potion",Some(&tag),180));if let Tag::Compound(fields)=raw{fields.insert("tank".into(),state.tank.to_nbt());}g.lan=Some(());let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}}
#[test]fn invalid_pending_or_output_tag_waits(){for pending in [false,true] {let(mut g,p,s,t,_)=super::fixture();let tag=Tag::compound([("Potion",Tag::String("unknown:custom".into()))]);if pending {let mut f=Faucet::new();f.active=true;assert!(f.pending.insert_tagged("tconstruct:potion",Some(&tag),20));g.world.tags.insert((p,"tconstruct:faucet".into()),f.to_nbt());}else{let mut out=Tank::new(4000);assert!(out.insert_tagged("tconstruct:potion",Some(&tag),10));g.world.tags.insert((t,"tconstruct:fuel_tank".into()),out.to_nbt());}let f=Faucet::from_nbt(&store::read(&g.world,p,"tconstruct:faucet").unwrap()).unwrap();let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,f,false),Some(false));assert_eq!(before,g.world.tags);}}
#[test]fn stale_controller_unknown_tank_and_duct_fail_closed(){let(mut g,p,s,t,c)=super::fixture();g.world.formed=false;let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);g.world.formed=true;g.world.blocks.insert(s,4);assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),None);g.world.blocks.insert(s,1);if let Tag::Compound(raw)=g.world.tags.get_mut(&(c,"tconstruct:smeltery".into())).unwrap(){if let Some(Tag::Compound(tank))=raw.get_mut(&"tank".into()){tank.insert("unknown".into(),Tag::Int(1));}}let before=g.world.tags.clone();assert_eq!(smeltery_to_tank(&mut g,p,s,t,Faucet::new(),true),Some(false));assert_eq!(before,g.world.tags);}
fn cannon_drain_fixture(foundry: bool) -> (Game<'static>, Pos, Pos, Pos, Pos) {
    let (mut game, pos, source, target, controller) = super::fixture();
    let mut output = super::tconstruct_source_fluid_cannon::Cannon::new();
    output.item = Some(horizon_items::items::ItemStack::new("minecraft:stone", 32));
    game.world.blocks.insert(target, 7);
    game.world.block_entities.insert(target, BlockEntity::Chest(vec![output.item.clone()]));
    game.world.tags.insert((target, "tconstruct:fluid_cannon".into()), output.to_nbt());
    if foundry { game.world.blocks.insert(source, 5); game.world.blocks.insert(controller, 6); }
    let mut input = Smeltery::from_nbt(&store::read(&game.world, controller, "tconstruct:smeltery").unwrap()).unwrap();
    input.resize(1, 4000);
    input.slots[0] = Some(horizon_items::items::ItemStack::new("minecraft:copper_ingot", 2));
    input.fuel = 30; input.temperature = 1000; input.progress[0] = 7;
    let mut raw = input.to_nbt();
    if let Tag::Compound(fields) = &mut raw {
        fields.insert("heat_codes".into(), Tag::Int(123));
        if let Some(Tag::List(slots)) = fields.get_mut(&"slots".into()) {
            if let Tag::Compound(item) = &mut slots[0] { item.insert("future_item".into(), Tag::Int(9)); }
        }
    }
    game.world.tags.insert((controller, "tconstruct:smeltery".into()), raw);
    (game, pos, source, target, controller)
}

#[test]
fn cannon_drain_first_and_later_packets_preserve_raw_controller_and_cannon_item() {
    for foundry in [false, true] {
        let (mut game, pos, source, target, controller) = cannon_drain_fixture(foundry);
        let before = store::read(&game.world, controller, "tconstruct:smeltery").unwrap();
        let item = cannon_at(&game, target).unwrap().item;
        assert_eq!(smeltery_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(true));
        let raw = store::read(&game.world, controller, "tconstruct:smeltery").unwrap();
        let input = Smeltery::from_nbt(&raw).unwrap();
        assert_eq!(raw, patch_smeltery_controller_tank(&before, &input).unwrap());
        let output = cannon_at(&game, target).unwrap();
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!((input.tank.total(), output.tank.amount, faucet.pending.amount), (90,10,80));
        assert_eq!(output.item, item);
        assert_eq!((game.signals, game.notifications), (1,2));
        for _ in 0..9 {
            let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
            assert_eq!(smeltery_to_cannon(&mut game, pos, source, target, faucet, false), Some(true));
        }
        let output = cannon_at(&game, target).unwrap();
        let input = Smeltery::from_nbt(&store::read(&game.world, controller, "tconstruct:smeltery").unwrap()).unwrap();
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!((input.tank.total(), output.tank.amount, faucet.pending.amount), (0,100,80));
        assert_eq!(output.item, item);
    }
}

#[test]
fn cannon_drain_failed_save_opaque_target_live_item_mismatch_and_stale_structure_wait() {
    for fault in 0..5 {
        let (mut game, pos, source, target, _) = cannon_drain_fixture(false);
        game.world.fail = fault == 0;
        if fault == 1 {
            if let Tag::Compound(fields) = game.world.tags.get_mut(&(target, "tconstruct:fluid_cannon".into())).unwrap() { fields.insert("future".into(), Tag::Int(1)); }
        }
        if fault == 2 {
            let BlockEntity::Chest(slots) = game.world.block_entities.get_mut(&target).unwrap(); slots[0] = None;
        }
        if fault == 3 { game.world.formed = false; }
        if fault == 4 { game.world.chunks.loaded.remove(&(-1,-1)); }
        let before = game.world.tags.clone();
        assert_eq!(smeltery_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(false));
        assert_eq!(game.world.tags, before);
        assert_eq!((game.signals, game.notifications), (0,0));
    }
}

#[test]
fn cannon_drain_potion_identity_mismatch_and_custom_metadata() {
    let tag = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
    let other = Tag::compound([("Potion", Tag::String("minecraft:healing".into()))]);
    for fault in 0..3 {
        let (mut game, pos, source, target, controller) = cannon_drain_fixture(true);
        let raw = store::read(&game.world, controller, "tconstruct:smeltery").unwrap();
        let mut input = Smeltery::from_nbt(&raw).unwrap(); input.tank.fluids.clear();
        let mut source_tag = tag.clone();
        if fault == 2 { if let Tag::Compound(fields) = &mut source_tag { fields.insert("CustomPotionColor".into(), Tag::Int(123)); } }
        assert!(input.tank.insert_tagged("tconstruct:potion", Some(&source_tag), 180));
        game.world.tags.insert((controller, "tconstruct:smeltery".into()), patch_smeltery_controller_tank(&raw, &input).unwrap());
        if fault == 1 {
            let mut output = cannon_at(&game, target).unwrap();
            assert!(output.tank.insert_tagged("tconstruct:potion", Some(&other), 10));
            game.world.tags.insert((target, "tconstruct:fluid_cannon".into()), output.to_nbt());
        }
        game.lan = Some(());
        let before = game.world.tags.clone();
        assert_eq!(smeltery_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(fault != 2));
        if fault > 0 { assert_eq!(game.world.tags, before); }
        else {
            let output = cannon_at(&game, target).unwrap();
            let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
            assert_eq!((output.tank.amount, output.tank.tag), (10,Some(tag.clone())));
            assert_eq!((faucet.pending.amount, faucet.pending.tag), (80,Some(tag.clone())));
        }
    }
}

#[test]
fn cannon_drain_pending_packet_finishes_before_unsupported_selected_layer_waits() {
    let (mut game, pos, source, target, controller) = cannon_drain_fixture(false);
    assert_eq!(smeltery_to_cannon(&mut game, pos, source, target, Faucet::new(), true), Some(true));
    let raw = store::read(&game.world, controller, "tconstruct:smeltery").unwrap();
    let mut input = Smeltery::from_nbt(&raw).unwrap();
    input.tank.fluids[0].tag = Some(Tag::compound([("future", Tag::Int(1))]));
    game.world.tags.insert((controller, "tconstruct:smeltery".into()), patch_smeltery_controller_tank(&raw, &input).unwrap());
    for _ in 0..8 {
        let faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
        assert_eq!(smeltery_to_cannon(&mut game, pos, source, target, faucet, false), Some(true));
    }
    assert_eq!(cannon_at(&game, target).unwrap().tank.amount, 90);
    let mut faucet = Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap();
    let before = game.world.tags.clone();
    assert_eq!(smeltery_to_cannon(&mut game, pos, source, target, faucet.clone(), false), Some(false));
    assert_eq!(game.world.tags, before);
    faucet.stop_after_packet = true;
    game.world.tags.insert((pos, "tconstruct:faucet".into()), faucet.to_nbt());
    assert_eq!(smeltery_to_cannon(&mut game, pos, source, target, faucet, false), Some(true));
    assert!(!Faucet::from_nbt(&store::read(&game.world, pos, "tconstruct:faucet").unwrap()).unwrap().active);
}


}
fn fixture()->(Game<'static>,Pos,Pos,Pos,Pos){use tconstruct_source_melter_state::{Tank,Faucet};use tconstruct_source_smeltery_state::Smeltery;let(p,s,t,c)=((0,1,0),(-2,0,0),(0,0,0),(-1,0,-1));let mut state=Smeltery::new();state.tank.capacity=4000;state.tank.insert("x:copper",180);let mut raw=state.to_nbt();if let Tag::Compound(fields)=&mut raw{fields.insert("heat_codes".into(),Tag::Int(123));}let tags=HashMap::from([((c,"tconstruct:smeltery".into()),raw),((p,"tconstruct:faucet".into()),Faucet::new().to_nbt()),((t,"tconstruct:fuel_tank".into()),Tank::new(4000).to_nbt())]);(Game{res:Res,remote:None,lan:None,lan_acting_uuid:None,world:World{blocks:HashMap::from([(s,1),(t,2),(c,3)]),block_entities:HashMap::from([(c,containers::BlockEntity::Chest(vec![])),(t,containers::BlockEntity::Chest(vec![]))]),chunks:Chunks{loaded:HashMap::from([((-1,0),()),((0,0),()),((-1,-1),())]),owners:HashMap::from([(s,c)])},tags,fail:false,formed:true},signals:0,notifications:0,_life:std::marker::PhantomData},p,s,t,c)}
