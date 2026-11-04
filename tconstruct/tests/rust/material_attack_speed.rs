// Execute the real injected helper with lightweight host boundary doubles.
extern crate self as horizon_world;
extern crate self as horizon_client;
extern crate self as horizon_items;
pub mod items { pub type ItemStack = super::Stack; }
use std::collections::BTreeMap;
use std::ops::ControlFlow;
pub mod attack_strength { pub const BASE_ATTACK_SPEED: f64 = 4.0; }
pub mod nbt {
    use super::BTreeMap;
    pub enum Tag { Float(u32), Int(i32), Byte(i8), Compound(BTreeMap<String, Tag>) }
    impl Tag {
        pub fn compound<const N: usize>(values: [(&str, Tag); N]) -> Self { Self::Compound(values.into_iter().map(|(k,v)|(k.to_owned(),v)).collect()) }
        pub fn get(&self, id: &str) -> Option<&Self> { match self { Self::Compound(map) => map.get(id), _ => None } } }
}
use nbt::Tag;
pub struct Data(Tag);
impl Data {
    fn other(&self, id: &str) -> Option<&Tag> { (id == "minecraft:custom_data").then_some(&self.0) }
    fn set_other(&mut self, id: &str, value: Option<Tag>) { assert_eq!(id,"minecraft:custom_data"); self.0=value.unwrap(); }
}
pub struct Stack { item: String, data: Option<Data>, damage: u32 }
impl Stack {
    fn new(id: &str, _: u32) -> Self { Self { item:id.into(), data:None, damage:0 } }
    fn data(&self) -> Option<&Data> { self.data.as_ref() }
    fn data_mut(&mut self) -> &mut Data { self.data.get_or_insert_with(|| Data(Tag::Compound(BTreeMap::new()))) }
}
pub struct Attribute(f64);
impl Attribute { fn base_value(&self) -> f64 { self.0 } }
pub struct Attributes { value: Option<Attribute>, valid_handle: bool }
impl Attributes {
    fn key(&self, id: &str) -> Option<&'static str> { (id == "minecraft:attack_speed" && self.value.is_some()).then_some("minecraft:attack_speed") }
    fn lookup(&self, key: Option<&&str>) -> Option<()> { (key.is_some() && self.valid_handle).then_some(()) }
    fn instance(&self, _: &()) -> Result<&Attribute, ()> { self.value.as_ref().ok_or(()) }
}
pub struct Player { attributes: Attributes }
#[derive(Default)] pub struct Effects(Vec<(&'static str, &'static str, f64)>);
impl Effects { fn modifiers(&self) -> Vec<(&'static str, &'static str, f64)> { self.0.clone() } }
pub struct Potions { effects: Effects }
pub mod game {
    use super::*;
    pub mod tconstruct_source_ranged_wear { include!("../../mixins/tconstruct/source-ranged-wear.rs"); }
    pub struct Game<'a> { pub held: Option<Stack>, pub player: Option<Player>, pub potions: Potions, pub marker: std::marker::PhantomData<&'a ()> }
    impl Game<'_> { pub fn held(&self) -> Option<&Stack> { self.held.as_ref() } }
    include!("../../mixins/tconstruct/material-attack-speed.rs");
    pub fn run(game: &Game<'_>) -> ControlFlow<f64> { tconstruct_material_attack_speed(game) }
}
use game::run as tconstruct_material_attack_speed;
fn game(base: f64, speed: f32) -> game::Game<'static> {
    game::Game { held: Some(Stack { damage:0, item: "tconstruct:sword".into(), data: Some(Data(Tag::Compound(BTreeMap::from([("tic_attack_speed".into(), Tag::Float(speed.to_bits()))])))) }),
        player: Some(Player { attributes: Attributes { value: Some(Attribute(base)), valid_handle: true } }), potions: Potions { effects: Effects::default() }, marker: std::marker::PhantomData }
}
fn result(g: &game::Game<'_>) -> f64 { match tconstruct_material_attack_speed(g) { ControlFlow::Break(v) => v, _ => panic!("missing replacement") } }
#[test] fn preserves_default_and_honors_changed_player_base() {
    assert_eq!(result(&game(4.0, 2.0)), 2.0);
    assert_eq!(result(&game(6.0, 2.0)), 4.0);
    assert_eq!(result(&game(3.0, 2.0)), 1.0);
    assert_eq!(result(&game(0.0, 5.0)), 1.0);
}
#[test] fn constructs_item_modifier_before_adding_player_base() {
    let base=2.0f64.powi(-51);
    let expected=base + (6.0f64 - 4.0);
    assert_ne!(expected.to_bits(), ((base + 6.0) - 4.0).to_bits());
    assert_eq!(result(&game(base,6.0)).to_bits(),expected.to_bits());
}
#[test] fn host_default_is_used_only_when_base_is_unavailable() {
    let mut g=game(6.0, 2.0); g.player=None; assert_eq!(result(&g),2.0);
    let mut g=game(6.0, 2.0); g.player.as_mut().unwrap().attributes.value=None; assert_eq!(result(&g),2.0);
    let mut g=game(6.0, 2.0); g.player.as_mut().unwrap().attributes.valid_handle=false; assert_eq!(result(&g),2.0);
}
#[test] fn potion_factors_follow_base_and_tool_then_clamp() {
    let mut g=game(6.0,2.0);
    g.potions.effects.0=vec![("minecraft:attack_speed","haste",0.2),("minecraft:movement_speed","other",10.0),("minecraft:attack_speed","fatigue",-0.1)];
    assert_eq!(result(&g),4.0 * 1.2 * 0.9);
    assert_eq!(result(&game(0.0,2.0)),0.0);
    assert_eq!(result(&game(1024.0,1024.0)),1024.0);
}
#[test] fn absent_or_non_material_stat_retains_native_path() {
    let mut g=game(6.0,2.0); g.held=None; assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
    let mut g=game(6.0,2.0); g.held.as_mut().unwrap().item="minecraft:iron_sword".into(); assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
    let mut g=game(6.0,2.0); g.held.as_mut().unwrap().data=None; assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
    let mut g=game(6.0,2.0); g.held.as_mut().unwrap().data=Some(Data(Tag::Compound(BTreeMap::from([("tic_attack_speed".into(),Tag::Int(2))])))); assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
}

fn durability(g: &mut game::Game<'_>, maximum: i32, damage: u32, broken: i8) {
    let stack=g.held.as_mut().unwrap(); stack.damage=damage;
    let Tag::Compound(fields)=&mut stack.data_mut().0 else { panic!() };
    fields.insert("tic_max_damage".into(),Tag::Int(maximum));
    fields.insert("tic_broken".into(),Tag::Byte(broken));
}
#[test] fn break_and_repair_remove_and_restore_only_tool_modifier() {
    let mut g=game(6.0,2.0);
    durability(&mut g,100,99,0); assert_eq!(result(&g),4.0);
    durability(&mut g,100,100,0); assert_eq!(result(&g),6.0);
    durability(&mut g,100,5,1); assert_eq!(result(&g),6.0);
    durability(&mut g,100,5,0); assert_eq!(result(&g),4.0);
    assert_eq!(g.held.as_ref().unwrap().damage,5);
    assert_eq!(g.player.as_ref().unwrap().attributes.value.as_ref().unwrap().base_value(),6.0);
}
#[test] fn broken_tool_keeps_base_potions_and_clamp_without_native_fallback() {
    let mut g=game(6.0,2.0); durability(&mut g,100,1,1);
    g.potions.effects.0=vec![("minecraft:attack_speed","haste",0.2),("minecraft:attack_speed","fatigue",-0.1)];
    assert_eq!(result(&g),6.0*1.2*0.9);
    let mut g=game(0.0,100.0); durability(&mut g,100,100,0); assert_eq!(result(&g),0.0);
    let mut g=game(2048.0,2.0); durability(&mut g,100,100,0); assert_eq!(result(&g),1024.0);
    let mut g=game(6.0,2.0); durability(&mut g,100,100,0); g.player=None; assert_eq!(result(&g),4.0);
}
#[test] fn broken_flag_is_independent_of_positive_maximum() {
    let mut g=game(6.0,2.0);
    durability(&mut g,0,0,-1); assert_eq!(result(&g),6.0);
    let Tag::Compound(fields)=&mut g.held.as_mut().unwrap().data_mut().0 else { panic!() };
    fields.remove("tic_max_damage"); assert_eq!(result(&g),6.0);
    durability(&mut g,0,100,0); assert_eq!(result(&g),4.0);
}
#[test] fn broken_flag_does_not_expand_missing_or_malformed_stat_eligibility() {
    let mut g=game(6.0,2.0); durability(&mut g,100,100,1);
    let Tag::Compound(fields)=&mut g.held.as_mut().unwrap().data_mut().0 else { panic!() };
    fields.insert("tic_attack_speed".into(),Tag::Int(2));
    assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
    let Tag::Compound(fields)=&mut g.held.as_mut().unwrap().data_mut().0 else { panic!() };
    fields.remove("tic_attack_speed");
    assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
}
