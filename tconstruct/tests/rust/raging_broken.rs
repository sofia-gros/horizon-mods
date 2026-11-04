// Execute the actual damage RETURN helper and actual retained-durability
// predicate. Actor, traits and unrelated percent damage are boundary doubles.
extern crate self as horizon_world;
extern crate self as horizon_items;
pub mod items { pub type ItemStack = super::Stack; }
use std::collections::{BTreeMap, HashMap, HashSet};
#[path = "../../mixins/tconstruct/source-tags.rs"] mod tconstruct_source_tags;
#[path = "../../mixins/tconstruct/modifier-snapshot.rs"] mod tconstruct_modifier_snapshot;
pub mod nbt {
    use super::BTreeMap;
    pub enum Tag { Float(u32), Int(i32), Byte(i8), Compound(BTreeMap<String,Tag>) }
    impl Tag {
        pub fn get(&self, id: &str) -> Option<&Self> { match self { Self::Compound(v) => v.get(id), _ => None } }
        pub fn compound<const N: usize>(v: [(&str,Tag);N]) -> Self { Self::Compound(v.into_iter().map(|(k,v)|(k.into(),v)).collect()) }
    }
}
use nbt::Tag;
pub struct Data(Tag);
impl Data {
    fn other(&self, id: &str) -> Option<&Tag> { (id=="minecraft:custom_data").then_some(&self.0) }
    fn set_other(&mut self, id: &str, v: Option<Tag>) { assert_eq!(id,"minecraft:custom_data"); self.0=v.unwrap(); }
}
pub struct Stack { item: String, damage: u32, data: Option<Data> }
impl Stack {
    fn new(item: &str, _: u32) -> Self { Self { item:item.into(), damage:0, data:None } }
    fn data(&self) -> Option<&Data> { self.data.as_ref() }
    fn data_mut(&mut self) -> &mut Data { self.data.get_or_insert_with(||Data(Tag::Compound(BTreeMap::new()))) }
}
pub struct Health(f32);
impl Health { fn health(&self) -> f32 { self.0 } }
pub struct Player { auto_spin_attack_dmg: f32, health: Health }
pub struct Chunks(HashSet<Option<u32>>, Option<Result<tconstruct_modifier_snapshot::Raw,String>>, Option<Result<HashMap<String,HashSet<String>>,String>>);
impl Chunks { fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->&Option<Result<HashMap<String,HashSet<String>>,String>> { &self.2 } fn __horizon_mixins_tconstruct_modifier_resource_snapshot(&self)->&Option<Result<tconstruct_modifier_snapshot::Raw,String>> { &self.1 } #[allow(non_snake_case)] fn __horizon_mixins_tconstruct_melee_active(&self) -> &HashSet<Option<u32>> { &self.0 } }
pub struct World { chunks: Chunks, level: Option<i32> }
pub mod game {
    use super::*;
    pub(crate) use crate::tconstruct_modifier_snapshot;
    pub struct Game<'a> { pub player: Option<Player>, pub inventory: Vec<Option<Stack>>, pub selected: usize, pub offhand: Option<Stack>, pub acting_off: bool, pub world: World, pub lan_acting: Option<u32>, pub marker: std::marker::PhantomData<&'a ()> }
    impl Game<'_> {
        pub fn held(&self) -> Option<&Stack> { if self.acting_off { self.offhand.as_ref() } else { self.inventory.get(self.selected).and_then(Option::as_ref) } }
        pub fn attack_damage(&mut self) -> f32 { 4.0 }
    }
    pub mod tconstruct_source_ranged_wear { include!("../../mixins/tconstruct/source-ranged-wear.rs"); }
    pub mod tconstruct_source_material_traits {
        use super::*;
        pub struct Traits { pub levels: BTreeMap<String,i32>, pub durability_multiplier: f32 }
        pub fn levels(world: &World, _: &Stack) -> Option<Traits> { Some(Traits { levels:BTreeMap::from([("tconstruct:raging".into(),world.level?)]), durability_multiplier:1.0 }) }
        pub fn level_with_upgrade(stack: &Stack, traits: &Traits, id: &str) -> Option<i32> {
            let saved=stack.data()?.other("minecraft:custom_data")?.get("tic_upgrades");
            let upgrade=match saved {None=>0,Some(tag @ Tag::Compound(_))=>match tag.get(id) {None=>0,Some(Tag::Int(n)) if *n>=0=>*n,_=>return None},_=>return None};
            let level=traits.levels.get(id).copied().unwrap_or(0); if level<0 {return None;} level.checked_add(upgrade)
        }
    }
    pub mod tconstruct_source_material_effects {
        pub fn wear_damage_bonus(_: f32, _: u32, _: f32, jagged: i32, stonebound: i32) -> f32 {
            // This fixture deliberately has neither unrelated percent trait.
            assert_eq!((jagged,stonebound),(0,0)); 0.0
        }
    }
    include!("../../mixins/tconstruct/source-melee-damage-return.rs");
    pub fn run(value:(f32,f32,bool,bool), game: &mut Game<'_>, living: bool) -> (f32,f32,bool,bool) { tconstruct_melee_secondary_damage(value,game,living) }
}
fn game() -> game::Game<'static> {
    let mut stack=Stack::new("tconstruct:sword",1);
    stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
        ("tic_max_damage",Tag::Int(100)), ("tic_broken",Tag::Byte(0)), ("tic_raging_melee_multiplier",Tag::Float(1.0f32.to_bits())),
    ])));
    game::Game { player:Some(Player { auto_spin_attack_dmg:0.0,health:Health(1.0) }), inventory:vec![Some(stack)], selected:0, offhand:None, acting_off:false, world:World { chunks:Chunks(HashSet::new(),Some(Ok((BTreeMap::from([("tconstruct:tinkering/modifiers/raging".into(),vec![tconstruct_modifier_snapshot::PINNED_RAGING.to_vec()])]),BTreeMap::new()))),Some(Ok(HashMap::from([("tconstruct:modifiable/melee".into(),HashSet::from(["tconstruct:sword".into()]))])))),level:Some(1) }, lan_acting:None, marker:std::marker::PhantomData }
}
fn field(g: &mut game::Game<'_>, key: &str, value: Tag) {
    let Tag::Compound(fields)=&mut g.inventory[g.selected].as_mut().unwrap().data_mut().0 else { panic!() };
    fields.insert(key.into(),value);
}
const BASE:(f32,f32,bool,bool)=(4.0,1.0,false,false);
#[test] fn flag_suppresses_raging_below_max_and_repair_restores_it() {
    let mut g=game(); assert_eq!(game::run(BASE,&mut g,true),(6.5,1.0,false,true));
    field(&mut g,"tic_broken",Tag::Byte(1)); assert_eq!(game::run(BASE,&mut g,true),BASE);
    field(&mut g,"tic_broken",Tag::Byte(-1)); assert_eq!(game::run(BASE,&mut g,true),BASE);
    field(&mut g,"tic_broken",Tag::Byte(0)); assert_eq!(game::run(BASE,&mut g,true),(6.5,1.0,false,true));
    assert_eq!(g.inventory[g.selected].as_ref().unwrap().damage,0);
}
#[test] fn damage_limit_and_positive_maximum_guards_remain() {
    let mut g=game(); g.inventory[g.selected].as_mut().unwrap().damage=100; assert_eq!(game::run(BASE,&mut g,true),BASE);
    g.inventory[g.selected].as_mut().unwrap().damage=101; assert_eq!(game::run(BASE,&mut g,true),BASE);
    g.inventory[g.selected].as_mut().unwrap().damage=0; field(&mut g,"tic_max_damage",Tag::Int(0)); assert_eq!(game::run(BASE,&mut g,true),BASE);
    field(&mut g,"tic_max_damage",Tag::Float(100.0f32.to_bits())); assert_eq!(game::run(BASE,&mut g,true),BASE);
}
#[test] fn living_spin_traits_and_actor_health_gates_unchanged() {
    let mut g=game(); assert_eq!(game::run(BASE,&mut g,false),BASE);
    g.player.as_mut().unwrap().auto_spin_attack_dmg=3.0; assert_eq!(game::run(BASE,&mut g,true),BASE);
    g.player.as_mut().unwrap().auto_spin_attack_dmg=0.0; g.world.level=None; assert_eq!(game::run(BASE,&mut g,true),BASE);
    g.world.level=Some(1); g.lan_acting=Some(42); g.player.as_mut().unwrap().health.0=6.0;
    assert_eq!(game::run(BASE,&mut g,true),(5.25,1.0,false,true));
    field(&mut g,"tic_broken",Tag::Byte(1)); assert_eq!(game::run(BASE,&mut g,true),BASE);
}
#[test] fn preserves_existing_magic_critical_and_cooldown_values_when_suppressed() {
    let mut g=game(); field(&mut g,"tic_broken",Tag::Byte(1));
    let input=(7.0,0.25,true,true); assert_eq!(game::run(input,&mut g,true),input);
    g.world.chunks.0.insert(None);
    // The unrelated secondary-AOE critical adjustment still runs.
    assert_eq!(game::run(input,&mut g,true),(6.5,0.25,false,true));
}

#[test] fn saved_raging_merges_once_and_rejects_invalid_inputs() {
    for (trait_level,upgrade,expected) in [(0,2,9.0),(1,2,11.5),(2,0,9.0),(i32::MAX,1,4.0),(-1,2,4.0),(1,-1,4.0)] {
        let mut g=game();g.world.level=Some(trait_level);
        field(&mut g,"tic_upgrades",Tag::compound([("tconstruct:raging",Tag::Int(upgrade))]));
        for _ in 0..2 {assert_eq!(game::run((4.0,1.0,false,false),&mut g,true).0,expected);}
        field(&mut g,"tic_broken",Tag::Byte(1));assert_eq!(game::run((4.0,1.0,false,false),&mut g,true).0,4.0);
    }
    let mut g=game();field(&mut g,"tic_upgrades",Tag::Int(2));assert_eq!(game::run((4.0,1.0,false,false),&mut g,true).0,4.0);
    field(&mut g,"tic_upgrades",Tag::compound([("tconstruct:raging",Tag::Float(2.0f32.to_bits()))]));assert_eq!(game::run((4.0,1.0,false,false),&mut g,true).0,4.0);
}
#[test] fn raging_uses_only_exact_highest_definition() {
    let mut g=game();field(&mut g,"tic_upgrades",Tag::compound([("tconstruct:raging",Tag::Int(2))]));
    let pinned=tconstruct_modifier_snapshot::PINNED_RAGING;
    for changed in [b"{}".to_vec(),String::from_utf8(pinned.to_vec()).unwrap().replace("2.5","5.0").into_bytes(),[pinned,b" "].concat()] {
        g.world.chunks.1=Some(Ok((BTreeMap::from([("tconstruct:tinkering/modifiers/raging".into(),vec![pinned.to_vec(),changed])]),BTreeMap::new())));
        assert_eq!(game::run((4.0,1.0,false,false),&mut g,true).0,4.0);
    }
    for raw in [None,Some(Err("unreadable".into())),Some(Ok((BTreeMap::new(),BTreeMap::new())))] {
        g.world.chunks.1=raw;assert_eq!(game::run((4.0,1.0,false,false),&mut g,true).0,4.0);
    }
    g.world.chunks.1=Some(Ok((BTreeMap::from([("tconstruct:tinkering/modifiers/raging".into(),vec![b"{}".to_vec(),pinned.to_vec()])]),BTreeMap::new())));
    assert_eq!(game::run((4.0,1.0,false,false),&mut g,true).0,11.5);
}

#[test] fn raging_requires_live_melee_membership_without_suppressing_base_damage() {
    let mut g=game();field(&mut g,"tic_upgrades",Tag::compound([("tconstruct:raging",Tag::Int(2))]));
    let base=(4.0,1.0,false,false);
    assert_eq!(game::run(base,&mut g,true).0,11.5);
    for tags in [None,Some(Err("unavailable".into())),Some(Ok(HashMap::new())),Some(Ok(HashMap::from([("tconstruct:modifiable/melee".into(),HashSet::new())])))] {
        g.world.chunks.2=tags;
        assert_eq!(game::run(base,&mut g,true),base);
        // Stale multiplier and saved upgrade remain, but cannot grant eligibility.
        assert!(g.inventory[g.selected].as_ref().unwrap().data().unwrap().other("minecraft:custom_data").unwrap().get("tic_raging_melee_multiplier").is_some());
    }
    g.world.chunks.2=Some(Ok(HashMap::from([("tconstruct:modifiable/melee".into(),HashSet::from(["tconstruct:sword".into()]))])));
    assert_eq!(game::run(base,&mut g,true).0,11.5);
    g.inventory[g.selected].as_mut().unwrap().item="tconstruct:longbow".into();
    assert_eq!(game::run(base,&mut g,true),base);
    g.world.chunks.2.as_mut().unwrap().as_mut().unwrap().get_mut("tconstruct:modifiable/melee").unwrap().insert("tconstruct:longbow".into());
    assert_eq!(game::run(base,&mut g,true).0,11.5);
}

#[test] fn selected_mainhand_is_independent_of_acting_hand_and_same_id_metadata() {
    let mut g=game();
    let mut other=game();
    field(&mut other,"tic_raging_melee_multiplier",Tag::Float(3.0f32.to_bits()));
    g.offhand=other.inventory[0].take();g.acting_off=true;
    assert_eq!(g.held().unwrap().item,g.inventory[0].as_ref().unwrap().item);
    assert_eq!(game::run(BASE,&mut g,true).0,6.5);
    field(&mut g,"tic_upgrades",Tag::compound([("tconstruct:raging",Tag::Int(1))]));
    assert_eq!(game::run(BASE,&mut g,true).0,9.0);
    field(&mut g,"tic_upgrades",Tag::compound([]));
    field(&mut g,"tic_broken",Tag::Byte(1));
    assert_eq!(game::run(BASE,&mut g,true),BASE);
    field(&mut g,"tic_broken",Tag::Byte(0));
    g.inventory.push(g.offhand.take());g.selected=1;
    assert_eq!(game::run(BASE,&mut g,true).0,11.5);
    // Guest-shaped context: selected inventory and actor health change together.
    g.lan_acting=Some(42);g.player.as_mut().unwrap().health.0=6.0;
    assert_eq!(game::run(BASE,&mut g,true).0,7.75);
    g.selected=0;assert_eq!(game::run(BASE,&mut g,true).0,5.25);
    g.inventory[0]=None;assert_eq!(game::run(BASE,&mut g,true),BASE);
    g.selected=999;assert_eq!(game::run(BASE,&mut g,true),BASE);
    g.selected=1;g.player=None;assert_eq!(game::run(BASE,&mut g,true),BASE);
}
