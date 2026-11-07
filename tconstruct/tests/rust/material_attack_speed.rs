// Real PlutoLib graph engine under the native library export name.
#[path = "../../../pluto-lib/mixins/pluto_native_creative/tag-graph.rs"]
pub mod pluto_tag_graph;
// Use the real shared resource API in this standalone boundary harness.
extern crate self as pluto_api;
#[path = "../../../pluto-lib/mixins/pluto_native_creative/api/src/resources.rs"]
pub mod resources;
// Execute the real injected helper with lightweight host boundary doubles.
extern crate self as horizon_world;
extern crate self as horizon_client;
extern crate self as horizon_items;
pub mod items { pub type ItemStack = super::Stack; }
use std::collections::{BTreeMap,HashMap,HashSet};
use std::ops::ControlFlow;
#[path = "../../mixins/tconstruct/source-tags.rs"] mod tconstruct_source_tags;
#[path = "../../mixins/tconstruct/modifier-snapshot.rs"] mod tconstruct_modifier_snapshot;
pub mod first_person { #[derive(PartialEq)] pub enum Hand { Main, Off } }
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
    pub(crate) use crate::tconstruct_modifier_snapshot;
    pub struct Chunks { pub tags: Option<Result<HashMap<String,HashSet<String>>,String>>, pub raw: Option<Result<tconstruct_modifier_snapshot::Raw,String>> }
    impl Chunks { pub fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->&Option<Result<HashMap<String,HashSet<String>>,String>>{&self.tags} pub fn __horizon_mixins_tconstruct_modifier_resource_snapshot(&self)->&Option<Result<tconstruct_modifier_snapshot::Raw,String>> { &self.raw } }
    pub struct World { pub chunks: Chunks, pub level: Option<i32> }
    pub mod tconstruct_source_ranged_wear { include!("../../mixins/tconstruct/source-ranged-wear.rs"); }
    // Resource/trait-resolution boundary double; the production helper uses
    // the existing live world definition resolver and checked upgrade merger.
    pub mod tconstruct_source_material_traits {
        use super::*;
        pub struct Traits(pub i32);
        pub fn levels(world: &World, _: &Stack) -> Option<Traits> { world.level.map(Traits) }
        pub fn level_with_upgrade(stack: &Stack, traits: &Traits, id: &str) -> Option<i32> {
            let saved=stack.data()?.other("minecraft:custom_data")?.get("tic_upgrades");
            let upgrade=match saved {
                None=>0,
                Some(tag @ Tag::Compound(_))=>match tag.get(id) {None=>0,Some(Tag::Int(n)) if *n>=0=>*n,_=>return None},
                _=>return None,
            };
            if traits.0<0 {return None;} traits.0.checked_add(upgrade)
        }
    }
    pub struct Game<'a> { pub world: World, pub hand: first_person::Hand, pub inventory: Vec<Option<Stack>>, pub selected: usize, pub offhand: Option<Stack>, pub player: Option<Player>, pub potions: Potions, pub marker: std::marker::PhantomData<&'a ()> }
    impl Game<'_> { pub fn held(&self) -> Option<&Stack> { if self.hand==first_person::Hand::Main {self.inventory.get(self.selected).and_then(Option::as_ref)} else {self.offhand.as_ref()} } }
    include!("../../mixins/tconstruct/material-attack-speed.rs");
    pub fn run(game: &Game<'_>) -> ControlFlow<f64> { tconstruct_material_attack_speed(game) }
}
use game::run as tconstruct_material_attack_speed;
fn game(base: f64, speed: f32) -> game::Game<'static> {
    game::Game { world: game::World { level: Some(0), chunks: game::Chunks { tags:Some(Ok(HashMap::from([("tconstruct:modifiable/melee/weapon".into(),HashSet::from(["tconstruct:sword".into()]))]))), raw: Some(Ok((BTreeMap::from([("tconstruct:tinkering/modifiers/overwield".into(),vec![tconstruct_modifier_snapshot::PINNED_OVERWIELD.to_vec()])]),BTreeMap::new()))) } }, hand: first_person::Hand::Main, inventory: vec![Some(Stack { damage:0, item: "tconstruct:sword".into(), data: Some(Data(Tag::Compound(BTreeMap::from([("tic_attack_speed".into(), Tag::Float(speed.to_bits()))])))) })], selected:0, offhand:None,
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
    let mut g=game(6.0,2.0); g.inventory[0]=None; assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
    let mut g=game(6.0,2.0); g.inventory[0].as_mut().unwrap().item="minecraft:iron_sword".into(); assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
    let mut g=game(6.0,2.0); g.inventory[0].as_mut().unwrap().data=None; assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
    let mut g=game(6.0,2.0); g.inventory[0].as_mut().unwrap().data=Some(Data(Tag::Compound(BTreeMap::from([("tic_attack_speed".into(),Tag::Int(2))])))); assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
}

fn durability(g: &mut game::Game<'_>, maximum: i32, damage: u32, broken: i8) {
    let stack=g.inventory[0].as_mut().unwrap(); stack.damage=damage;
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
    assert_eq!(g.inventory[0].as_ref().unwrap().damage,5);
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
    let Tag::Compound(fields)=&mut g.inventory[0].as_mut().unwrap().data_mut().0 else { panic!() };
    fields.remove("tic_max_damage"); assert_eq!(result(&g),6.0);
    durability(&mut g,0,100,0); assert_eq!(result(&g),4.0);
}
#[test] fn broken_flag_does_not_expand_missing_or_malformed_stat_eligibility() {
    let mut g=game(6.0,2.0); durability(&mut g,100,100,1);
    let Tag::Compound(fields)=&mut g.inventory[0].as_mut().unwrap().data_mut().0 else { panic!() };
    fields.insert("tic_attack_speed".into(),Tag::Int(2));
    assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
    let Tag::Compound(fields)=&mut g.inventory[0].as_mut().unwrap().data_mut().0 else { panic!() };
    fields.remove("tic_attack_speed");
    assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
}

fn overslime(g: &mut game::Game<'_>, amount: Tag, upgrade: Tag) {
    let Tag::Compound(fields)=&mut g.inventory[0].as_mut().unwrap().data_mut().0 else {panic!()};
    fields.insert("tic_overslime".into(),amount);
    fields.insert("tic_upgrades".into(),Tag::compound([("tconstruct:overwield",upgrade)]));
}
#[test] fn overwield_reads_live_slime_and_merged_level_without_rebuilding() {
    let mut g=game(6.0,2.0);g.world.level=Some(2);
    for (slime,level) in [(0,0),(1,1),(2,2),(3,3),(99,3)] {
        overslime(&mut g,Tag::Int(slime),Tag::Int(1));
        assert_eq!(result(&g),4.0*(1.0+f64::from(level as f32*0.15f32)));
    }
    g.world.level=Some(0);overslime(&mut g,Tag::Int(3),Tag::Int(1));
    assert_eq!(result(&g),4.0*(1.0+f64::from(0.15f32)));
    g.world.level=None;assert_eq!(result(&g),4.0);
}
#[test] fn overwield_keeps_potions_base_broken_gates_and_clamp() {
    let mut g=game(6.0,2.0);g.world.level=Some(2);overslime(&mut g,Tag::Int(2),Tag::Int(0));
    g.potions.effects.0=vec![("minecraft:attack_speed","haste",0.2),("minecraft:movement_speed","other",10.0)];
    assert_eq!(result(&g),4.0*1.2*(1.0+f64::from(0.3f32)));
    durability(&mut g,100,100,0);assert_eq!(result(&g),6.0*1.2);
    durability(&mut g,100,0,1);assert_eq!(result(&g),6.0*1.2);
    durability(&mut g,100,0,0);assert_eq!(result(&g),4.0*1.2*(1.0+f64::from(0.3f32)));
    g.player.as_mut().unwrap().attributes.value=Some(Attribute(2048.0));assert_eq!(result(&g),1024.0);
}
#[test] fn overwield_malformed_negative_or_overflow_levels_supply_no_bonus() {
    let mut g=game(4.0,2.0);g.world.level=Some(1);
    for value in [Tag::Int(0),Tag::Int(-1),Tag::Float(1.0f32.to_bits()),Tag::Byte(1)] {
        overslime(&mut g,value,Tag::Int(1));assert_eq!(result(&g),2.0);
    }
    for value in [Tag::Int(-1),Tag::Float(1.0f32.to_bits()),Tag::Int(i32::MAX)] {
        overslime(&mut g,Tag::Int(10),value);assert_eq!(result(&g),2.0);
    }
    g.world.level=Some(-1);overslime(&mut g,Tag::Int(10),Tag::Int(0));assert_eq!(result(&g),2.0);
}

#[test] fn overwield_requires_exact_highest_definition_and_mainhand() {
    let mut g=game(4.0,2.0);g.world.level=Some(2);overslime(&mut g,Tag::Int(2),Tag::Int(0));
    let boosted=result(&g);assert!(boosted>2.0);
    // held() is the acting stack, including while the host acts with its offhand.
    g.hand=first_person::Hand::Off;assert_eq!(result(&g),boosted);
    g.hand=first_person::Hand::Main;assert_eq!(result(&g),boosted);
    let pinned=tconstruct_modifier_snapshot::PINNED_OVERWIELD;
    for changed in [b"{}".to_vec(),String::from_utf8(pinned.to_vec()).unwrap().replace("0.15","0.25").into_bytes(),String::from_utf8(pinned.to_vec()).unwrap().replace("mainhand","offhand").into_bytes(),[pinned,b" "].concat()] {
        g.world.chunks.raw=Some(Ok((BTreeMap::from([("tconstruct:tinkering/modifiers/overwield".into(),vec![pinned.to_vec(),changed])]),BTreeMap::new())));
        assert_eq!(result(&g),2.0);
    }
    for raw in [None,Some(Err("unreadable".into())),Some(Ok((BTreeMap::new(),BTreeMap::new())))] {
        g.world.chunks.raw=raw;assert_eq!(result(&g),2.0);
    }
    g.world.chunks.raw=Some(Ok((BTreeMap::from([("tconstruct:tinkering/modifiers/overwield".into(),vec![b"{}".to_vec(),pinned.to_vec()])]),BTreeMap::new())));
    assert_eq!(result(&g),boosted);
}

#[test] fn base_speed_uses_live_weapon_tag_but_overwield_remains_independent() {
    let mut g=game(6.0,2.0);g.world.level=Some(2);overslime(&mut g,Tag::Int(2),Tag::Int(0));
    let factor=1.0+f64::from(0.3f32);
    g.potions.effects.0=vec![("minecraft:attack_speed","haste",0.2)];
    assert_eq!(result(&g),4.0*1.2*factor);
    for tags in [None,Some(Err("unavailable".into())),Some(Ok(HashMap::new())),Some(Ok(HashMap::from([("tconstruct:modifiable/melee/weapon".into(),HashSet::new())])))] {
        g.world.chunks.tags=tags;
        assert_eq!(result(&g),6.0*1.2*factor);
    }
    g.world.chunks.tags=Some(Ok(HashMap::from([("tconstruct:modifiable/melee/weapon".into(),HashSet::from(["tconstruct:sword".into()]))])));
    assert_eq!(result(&g),4.0*1.2*factor);
    durability(&mut g,100,100,0);assert_eq!(result(&g),6.0*1.2);
}
#[test] fn acting_offhand_does_not_replace_selected_mainhand_attributes() {
    let mut g=game(6.0,2.0);g.world.level=Some(2);overslime(&mut g,Tag::Int(2),Tag::Int(0));
    let main=result(&g);
    g.offhand=game(4.0,100.0).inventory.remove(0);
    g.hand=first_person::Hand::Off;
    assert!(g.held().is_some());assert_eq!(result(&g),main);
    g.inventory.push(game(4.0,3.0).inventory.remove(0));g.selected=1;
    assert_eq!(result(&g),5.0);
    g.inventory[1]=None;
    assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
    g.selected=usize::MAX;
    assert_eq!(tconstruct_material_attack_speed(&g),ControlFlow::Continue(()));
}
