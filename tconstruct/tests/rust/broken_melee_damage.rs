extern crate self as horizon_client;
// Real PlutoLib graph engine under the native library export name.
#[path = "../../../pluto-lib/mixins/pluto_native_creative/tag-graph.rs"]
pub mod pluto_tag_graph;
// Use the real shared resource API in this standalone boundary harness.
extern crate self as pluto_api;
#[path = "../../../pluto-lib/mixins/pluto_native_creative/api/src/resources.rs"]
pub mod resources;
// The real projection helper and broken predicate, with host attribute/trait
// boundary doubles. The model keeps additive effects before total multipliers.
extern crate self as horizon_world;
extern crate self as horizon_items;
pub mod items { pub type ItemStack=super::Stack; }
use std::collections::{BTreeMap,HashMap,HashSet};
use std::ops::ControlFlow;
#[path = "../../mixins/tconstruct/source-tags.rs"] mod tconstruct_source_tags;
#[path = "../../mixins/tconstruct/modifier-snapshot.rs"] mod tconstruct_modifier_snapshot;
pub mod nbt {
    use super::BTreeMap;
    pub enum Tag { Float(u32), Int(i32), Byte(i8), Compound(BTreeMap<String,Tag>) }
    impl Tag {
        pub fn get(&self,k:&str)->Option<&Self>{match self {Self::Compound(v)=>v.get(k),_=>None}}
        pub fn compound<const N:usize>(v:[(&str,Tag);N])->Self{Self::Compound(v.into_iter().map(|(k,v)|(k.into(),v)).collect())}
    }
}
use nbt::Tag;
pub struct Data(Tag);
impl Data {
    fn other(&self,k:&str)->Option<&Tag>{(k=="minecraft:custom_data").then_some(&self.0)}
    fn set_other(&mut self,k:&str,v:Option<Tag>){assert_eq!(k,"minecraft:custom_data");self.0=v.unwrap();}
}
pub struct Stack { item:String,damage:u32,data:Option<Data> }
impl Stack {
    fn new(item:&str,_:u32)->Self{Self{item:item.into(),damage:0,data:None}}
    fn data(&self)->Option<&Data>{self.data.as_ref()}
    fn data_mut(&mut self)->&mut Data{self.data.get_or_insert_with(||Data(Tag::Compound(BTreeMap::new())))}
}
pub struct ItemInfo { attack_damage:f32 }
#[derive(Clone)] pub struct Player { base:f64,native:f64,additive:f64,total:f64,projection:f64,fail_set:bool,fail_read:bool }
impl Player {
    fn set_modifier(&mut self,attribute:&str,id:&str,value:f64,op:&str,enabled:bool)->Result<(),()>{
        assert_eq!((attribute,id,op,enabled),("minecraft:attack_damage","tconstruct:material_attack_projection","add_value",true));
        if self.fail_set{return Err(())}self.projection=value;Ok(())
    }
    fn attribute(&mut self,id:&str)->Result<f64,()>{assert_eq!(id,"minecraft:attack_damage");if self.fail_read{return Err(())}Ok(((self.base+self.native+self.additive+self.projection)*self.total).max(0.0))}
}
pub struct Chunks { raw:Option<Result<tconstruct_modifier_snapshot::Raw,String>>, tags:Option<Result<HashMap<String,HashSet<String>>,String>> }
impl Chunks {fn __horizon_mixins_tconstruct_modifier_resource_snapshot(&self)->&Option<Result<tconstruct_modifier_snapshot::Raw,String>>{&self.raw}fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->&Option<Result<HashMap<String,HashSet<String>>,String>>{&self.tags}}
pub struct World { stonebound_level:i32, trait_level:i32, chunks:Chunks }
pub mod game {
    pub(crate) use crate::tconstruct_modifier_snapshot;
    use super::*;
    pub struct Game<'a>{pub inventory:Vec<Option<Stack>>,pub selected:usize,pub offhand:Option<Stack>,pub acting_offhand:bool,pub attack_item:Option<String>,pub info:Option<ItemInfo>,pub player:Option<Player>,pub world:World,pub marker:std::marker::PhantomData<&'a ()>}
    impl Game<'_>{pub fn held(&self)->Option<&Stack>{if self.acting_offhand {self.offhand.as_ref()} else {self.inventory.get(self.selected).and_then(Option::as_ref)}}pub fn attack_item_info(&self)->Option<&ItemInfo>{self.info.as_ref()}}
    pub mod tconstruct_source_ranged_wear { include!("../../mixins/tconstruct/source-ranged-wear.rs"); }
    pub mod tconstruct_source_material_traits {
        use super::*;
        pub struct Traits{pub levels:BTreeMap<String,i32>,pub durability_multiplier:f32}
        pub fn levels(world:&World,_:&Stack)->Option<Traits>{
            assert!(world.trait_level>=0,"broken stack must not resolve percent traits");
            Some(Traits{levels:BTreeMap::from([("tconstruct:jagged".into(),world.trait_level),("tconstruct:stonebound".into(),world.stonebound_level)]),durability_multiplier:1.0})
        }
    }
    pub mod tconstruct_source_material_effects {
        pub fn wear_damage_bonus(damage:f32,_:u32,_:f32,jagged:i32,stonebound:i32)->f32{damage*(0.5*jagged as f32-0.25*stonebound as f32)}
    }
    include!("../../mixins/tconstruct/material-melee-bonus.rs");
    pub fn run(g:&Game<'_>)->ControlFlow<f32>{tconstruct_material_melee_bonus(g)}
}
fn game()->game::Game<'static>{
    let mut held=Stack::new("tconstruct:sword",1);
    held.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_attack_damage",Tag::Float(8.0f32.to_bits())),("tic_max_damage",Tag::Int(100)),("tic_broken",Tag::Byte(0))])));
    game::Game{inventory:vec![Some(held)],selected:0,offhand:None,acting_offhand:false,attack_item:Some("tconstruct:sword".into()),info:Some(ItemInfo{attack_damage:3.0}),player:Some(Player{base:1.0,native:3.0,additive:0.0,total:1.0,projection:0.0,fail_set:false,fail_read:false}),world:World{stonebound_level:0,trait_level:0,chunks:Chunks{raw:Some(Ok((BTreeMap::from([("tconstruct:tinkering/modifiers/jagged".into(),vec![tconstruct_modifier_snapshot::PINNED_JAGGED.to_vec()])]),BTreeMap::new()))),tags:Some(Ok(HashMap::from([("tconstruct:modifiable/melee/weapon".into(),HashSet::from(["tconstruct:sword".into()])),("tconstruct:modifiable/melee".into(),HashSet::from(["tconstruct:sword".into()]))])))}},marker:std::marker::PhantomData}
}
fn field(g:&mut game::Game<'_>,key:&str,value:Tag){let Tag::Compound(v)=&mut g.inventory[0].as_mut().unwrap().data_mut().0 else{panic!()};v.insert(key.into(),value);}
#[test] fn broken_removes_native_and_material_damage_and_repair_restores(){
    let mut g=game();assert_eq!(game::run(&g),ControlFlow::Break(9.0));
    field(&mut g,"tic_broken",Tag::Byte(1));assert_eq!(game::run(&g),ControlFlow::Break(1.0));
    field(&mut g,"tic_broken",Tag::Byte(0));g.inventory[0].as_mut().unwrap().damage=100;assert_eq!(game::run(&g),ControlFlow::Break(1.0));
    g.inventory[0].as_mut().unwrap().damage=99;assert_eq!(game::run(&g),ControlFlow::Break(9.0));
    assert_eq!(g.player.as_ref().unwrap().projection,0.0);assert_eq!(g.inventory[0].as_ref().unwrap().damage,99);
}
#[test] fn broken_preserves_player_base_effects_and_multiplier_order(){
    let mut g=game();field(&mut g,"tic_broken",Tag::Byte(-1));
    let p=g.player.as_mut().unwrap();p.base=5.0;p.additive=3.0;p.total=1.5;
    assert_eq!(game::run(&g),ControlFlow::Break(12.0));
    g.player.as_mut().unwrap().additive=-2.0;assert_eq!(game::run(&g),ControlFlow::Break(4.5));
    assert_eq!(g.player.as_ref().unwrap().projection,0.0);
}
#[test] fn percent_wear_traits_are_skipped_only_when_broken(){
    let mut g=game();g.world.trait_level=1;assert_eq!(game::run(&g),ControlFlow::Break(13.5));
    field(&mut g,"tic_broken",Tag::Byte(1));g.world.trait_level=-1;
    assert_eq!(game::run(&g),ControlFlow::Break(1.0));
}
#[test] fn selection_and_native_fallback_guards_stay_unchanged(){
    let mut g=game();field(&mut g,"tic_broken",Tag::Byte(1));g.attack_item=Some("tconstruct:pickaxe".into());assert_eq!(game::run(&g),ControlFlow::Continue(()));
    g.attack_item=Some("tconstruct:sword".into());field(&mut g,"tic_attack_damage",Tag::Int(8));assert_eq!(game::run(&g),ControlFlow::Continue(()));
    let mut g=game();g.player=None;assert_eq!(game::run(&g),ControlFlow::Continue(()));
    let mut g=game();g.player.as_mut().unwrap().fail_set=true;assert_eq!(game::run(&g),ControlFlow::Continue(()));
    let mut g=game();g.player.as_mut().unwrap().fail_read=true;assert_eq!(game::run(&g),ControlFlow::Continue(()));
}
#[test] fn current_native_item_damage_is_removed_not_a_fixed_constant(){
    let mut g=game();g.info.as_mut().unwrap().attack_damage=7.0;g.player.as_mut().unwrap().native=7.0;
    assert_eq!(game::run(&g),ControlFlow::Break(9.0));
    field(&mut g,"tic_broken",Tag::Byte(1));assert_eq!(game::run(&g),ControlFlow::Break(1.0));
    g.info=None;g.player.as_mut().unwrap().native=0.0;assert_eq!(game::run(&g),ControlFlow::Break(1.0));
}

#[test] fn acting_offhand_cannot_replace_mainhand_damage_or_broken_state() {
    let mut g=game();let mut other=game();field(&mut other,"tic_attack_damage",Tag::Float(40.0f32.to_bits()));
    g.offhand=other.inventory.remove(0);g.acting_offhand=true;
    assert_eq!(g.held().unwrap().item,"tconstruct:sword");
    assert_eq!(game::run(&g),ControlFlow::Break(9.0));
    let Tag::Compound(v)=&mut g.offhand.as_mut().unwrap().data_mut().0 else{panic!()};v.insert("tic_broken".into(),Tag::Byte(1));
    assert_eq!(game::run(&g),ControlFlow::Break(9.0));
    field(&mut g,"tic_broken",Tag::Byte(1));assert_eq!(game::run(&g),ControlFlow::Break(1.0));
    field(&mut g,"tic_broken",Tag::Byte(0));g.world.trait_level=1;
    assert_eq!(game::run(&g),ControlFlow::Break(13.5));
    assert_eq!(g.player.as_ref().unwrap().projection,0.0);
}
#[test] fn changing_selected_mainhand_preserves_id_and_missing_stack_fallbacks() {
    let mut g=game();g.acting_offhand=true;g.offhand=game().inventory.remove(0);
    let mut next=game();field(&mut next,"tic_attack_damage",Tag::Float(12.0f32.to_bits()));
    g.inventory.push(next.inventory.remove(0));g.selected=1;
    assert_eq!(game::run(&g),ControlFlow::Break(13.0));
    g.inventory[1].as_mut().unwrap().item="tconstruct:pickaxe".into();
    assert_eq!(game::run(&g),ControlFlow::Continue(()));
    g.inventory[1]=None;assert_eq!(game::run(&g),ControlFlow::Continue(()));
    g.selected=usize::MAX;assert_eq!(game::run(&g),ControlFlow::Continue(()));
}

#[test] fn missing_or_removed_weapon_tag_removes_only_saved_base_contribution() {
    let mut g=game();let p=g.player.as_mut().unwrap();p.base=5.0;p.additive=3.0;p.total=1.5;
    assert_eq!(game::run(&g),ControlFlow::Break(24.0));
    for tags in [None,Some(Err("unavailable".into())),Some(Ok(HashMap::new())),Some(Ok(HashMap::from([("tconstruct:modifiable/melee/weapon".into(),HashSet::new())])))] {
        g.world.chunks.tags=tags;
        assert_eq!(game::run(&g),ControlFlow::Break(12.0));
        assert_eq!(g.player.as_ref().unwrap().projection,0.0);
    }
    g.world.chunks.tags=Some(Ok(HashMap::from([("tconstruct:modifiable/melee/weapon".into(),HashSet::from(["tconstruct:sword".into()])),("tconstruct:modifiable/melee".into(),HashSet::from(["tconstruct:sword".into()]))])));
    assert_eq!(game::run(&g),ControlFlow::Break(24.0));
    field(&mut g,"tic_broken",Tag::Byte(1));assert_eq!(game::run(&g),ControlFlow::Break(12.0));
}
#[test] fn base_weapon_gate_does_not_disable_independent_percent_modifier() {
    let mut g=game();g.world.chunks.tags=Some(Ok(HashMap::from([("tconstruct:modifiable/melee".into(),HashSet::from(["tconstruct:sword".into()]))])));g.world.trait_level=1;
    // Existing percent-wear projection uses the remaining base 1, not the
    // removed saved tool damage 8; its independent 50% fixture still applies.
    assert_eq!(game::run(&g),ControlFlow::Break(1.5));
    field(&mut g,"tic_broken",Tag::Byte(1));g.world.trait_level=-1;
    assert_eq!(game::run(&g),ControlFlow::Break(1.0));
}

#[test] fn conditional_wear_requires_melee_tag_independently_of_weapon_tag() {
    let mut g=game();g.world.trait_level=1;
    assert_eq!(game::run(&g),ControlFlow::Break(13.5));
    // Valid weapon base remains even if a pack independently removes MELEE.
    g.world.chunks.tags=Some(Ok(HashMap::from([("tconstruct:modifiable/melee/weapon".into(),HashSet::from(["tconstruct:sword".into()]))])));
    g.world.trait_level=-1; // Resolver must not be called when ineligible.
    assert_eq!(game::run(&g),ControlFlow::Break(9.0));
    for tags in [None,Some(Err("unavailable".into())),Some(Ok(HashMap::new())),Some(Ok(HashMap::from([("tconstruct:modifiable/melee".into(),HashSet::new())])))] {
        g.world.chunks.tags=tags;assert_eq!(game::run(&g),ControlFlow::Break(1.0));
    }
    g.world.chunks.tags=Some(Ok(HashMap::from([("tconstruct:modifiable/melee".into(),HashSet::from(["tconstruct:sword".into()]))])));
    g.world.trait_level=1;assert_eq!(game::run(&g),ControlFlow::Break(1.5));
    let p=g.player.as_mut().unwrap();p.additive=3.0;p.total=2.0;
    assert_eq!(game::run(&g),ControlFlow::Break(9.0));
    assert_eq!(g.player.as_ref().unwrap().projection,0.0);
    field(&mut g,"tic_broken",Tag::Byte(1));g.world.trait_level=-1;
    assert_eq!(game::run(&g),ControlFlow::Break(8.0));
}

#[test] fn wear_definition_uses_highest_layer_and_fails_closed() {
    let mut g=game();g.world.trait_level=1;
    assert_eq!(game::run(&g),ControlFlow::Break(13.5));
    let id="tconstruct:tinkering/modifiers/jagged";
    let pinned=tconstruct_modifier_snapshot::PINNED_JAGGED;
    for changed in [b"{}".to_vec(), [pinned,b" "].concat(), String::from_utf8(pinned.to_vec()).unwrap().replace("0.005","0.5").into_bytes()] {
        g.world.chunks.raw=Some(Ok((BTreeMap::from([(id.into(),vec![pinned.to_vec(),changed])]),BTreeMap::new())));
        assert_eq!(game::run(&g),ControlFlow::Break(9.0));
    }
    for raw in [None,Some(Err("unavailable".into())),Some(Ok((BTreeMap::new(),BTreeMap::new())))] {
        g.world.chunks.raw=raw;assert_eq!(game::run(&g),ControlFlow::Break(9.0));
    }
    g.world.chunks.raw=Some(Ok((BTreeMap::from([(id.into(),vec![b"{}".to_vec(),pinned.to_vec()])]),BTreeMap::new())));
    assert_eq!(game::run(&g),ControlFlow::Break(13.5));
    assert_eq!(g.player.as_ref().unwrap().projection,0.0);
}

#[test] fn base_projection_gates_each_wear_effect_independently() {
    let mut g=game();g.world.trait_level=1;g.world.stonebound_level=1;
    let stone="tconstruct:tinkering/modifiers/stonebound";
    g.world.chunks.raw.as_mut().unwrap().as_mut().unwrap().0.insert(stone.into(),vec![tconstruct_modifier_snapshot::PINNED_STONEBOUND.to_vec()]);
    assert_eq!(game::run(&g),ControlFlow::Break(11.25));
    g.world.chunks.raw.as_mut().unwrap().as_mut().unwrap().0.remove("tconstruct:tinkering/modifiers/jagged");
    assert_eq!(game::run(&g),ControlFlow::Break(6.75));
    g.world.chunks.raw.as_mut().unwrap().as_mut().unwrap().0.get_mut(stone).unwrap().push(b"{}".to_vec());
    assert_eq!(game::run(&g),ControlFlow::Break(9.0));
}
