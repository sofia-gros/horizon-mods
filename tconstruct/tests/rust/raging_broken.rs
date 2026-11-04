// Execute the actual damage RETURN helper and actual retained-durability
// predicate. Actor, traits and unrelated percent damage are boundary doubles.
extern crate self as horizon_world;
extern crate self as horizon_items;
pub mod items { pub type ItemStack = super::Stack; }
use std::collections::{BTreeMap, HashSet};
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
pub struct Chunks(HashSet<Option<u32>>);
impl Chunks { #[allow(non_snake_case)] fn __horizon_mixins_tconstruct_melee_active(&self) -> &HashSet<Option<u32>> { &self.0 } }
pub struct World { chunks: Chunks, level: Option<i32> }
pub mod game {
    use super::*;
    pub struct Game<'a> { pub player: Option<Player>, pub stack: Option<Stack>, pub world: World, pub lan_acting: Option<u32>, pub marker: std::marker::PhantomData<&'a ()> }
    impl Game<'_> {
        pub fn held(&self) -> Option<&Stack> { self.stack.as_ref() }
        pub fn attack_damage(&mut self) -> f32 { 4.0 }
    }
    pub mod tconstruct_source_ranged_wear { include!("../../mixins/tconstruct/source-ranged-wear.rs"); }
    pub mod tconstruct_source_material_traits {
        use super::*;
        pub struct Traits { pub levels: BTreeMap<String,i32>, pub durability_multiplier: f32 }
        pub fn levels(world: &World, _: &Stack) -> Option<Traits> { Some(Traits { levels:BTreeMap::from([("tconstruct:raging".into(),world.level?)]), durability_multiplier:1.0 }) }
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
    game::Game { player:Some(Player { auto_spin_attack_dmg:0.0,health:Health(1.0) }), stack:Some(stack), world:World { chunks:Chunks(HashSet::new()),level:Some(1) }, lan_acting:None, marker:std::marker::PhantomData }
}
fn field(g: &mut game::Game<'_>, key: &str, value: Tag) {
    let Tag::Compound(fields)=&mut g.stack.as_mut().unwrap().data_mut().0 else { panic!() };
    fields.insert(key.into(),value);
}
const BASE:(f32,f32,bool,bool)=(4.0,1.0,false,false);
#[test] fn flag_suppresses_raging_below_max_and_repair_restores_it() {
    let mut g=game(); assert_eq!(game::run(BASE,&mut g,true),(6.5,1.0,false,true));
    field(&mut g,"tic_broken",Tag::Byte(1)); assert_eq!(game::run(BASE,&mut g,true),BASE);
    field(&mut g,"tic_broken",Tag::Byte(-1)); assert_eq!(game::run(BASE,&mut g,true),BASE);
    field(&mut g,"tic_broken",Tag::Byte(0)); assert_eq!(game::run(BASE,&mut g,true),(6.5,1.0,false,true));
    assert_eq!(g.stack.as_ref().unwrap().damage,0);
}
#[test] fn damage_limit_and_positive_maximum_guards_remain() {
    let mut g=game(); g.stack.as_mut().unwrap().damage=100; assert_eq!(game::run(BASE,&mut g,true),BASE);
    g.stack.as_mut().unwrap().damage=101; assert_eq!(game::run(BASE,&mut g,true),BASE);
    g.stack.as_mut().unwrap().damage=0; field(&mut g,"tic_max_damage",Tag::Int(0)); assert_eq!(game::run(BASE,&mut g,true),BASE);
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
