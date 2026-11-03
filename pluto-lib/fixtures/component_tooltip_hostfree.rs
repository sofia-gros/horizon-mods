use std::collections::HashMap;
mod horizon_world { pub mod nbt { #[derive(Clone)] pub enum Tag { Int(i32), String } } }
mod horizon_client { pub mod items { pub use crate::FakeItemStack as ItemStack; } pub mod text { pub use crate::FakeText as Text; } pub mod item_tooltip {
    use crate::FakeItemStack as ItemStack;
    pub struct TooltipDisplay(bool);
    impl TooltipDisplay { pub fn for_stack(s: &ItemStack) -> Self { Self(s.show_damage) } pub fn shows(&self, _: &str) -> bool { self.0 } }
} }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FakeText { key: Option<String>, value: String }
impl FakeText {
    fn literal(s: impl Into<String>) -> Self { Self { key: None, value: s.into() } }
    fn translate(k: &str, args: &[Text]) -> Self { Self { key: Some(k.into()), value: args.iter().map(|x| x.value.as_str()).collect::<Vec<_>>().join("/") } }
    fn translation_key(&self) -> Option<&str> { self.key.as_deref() }
}
struct Data(HashMap<String, horizon_world::nbt::Tag>);
impl Data { fn other(&self, key: &str) -> Option<&horizon_world::nbt::Tag> { self.0.get(key) } }
pub struct FakeItemStack { item: String, damage: u32, show_damage: bool, unbreakable: bool, data: Data }
impl FakeItemStack { fn data(&self) -> Option<&Data> { Some(&self.data) } }
include!("../mixins/pluto_component_display_3a/policy.rs");
fn stack(max: horizon_world::nbt::Tag, damage: u32) -> ItemStack { ItemStack { item: "minecraft:flint".into(), damage, show_damage: true, unbreakable: false, data: Data(HashMap::from([("minecraft:max_damage".into(), max)])) } }
fn lines() -> Vec<(Text,u32)> { vec![(Text::literal("name"), 1), (Text::literal("minecraft:flint"), 2), (Text::literal("tail"), 3)] }
#[test] fn inserts_before_id_once() { let s=stack(horizon_world::nbt::Tag::Int(100),40); let mut l=lines(); component_durability_lines(&s,&mut l,true); component_durability_lines(&s,&mut l,true); assert_eq!(l[1].0.value,"60/100"); assert_eq!(l.len(),4); assert_eq!(l[2].0.value,"minecraft:flint"); }
#[test] fn replaces_existing_and_preserves_color() { let s=stack(horizon_world::nbt::Tag::Int(100),40); let mut l=lines(); l.insert(1,(Text::translate("item.durability",&[]),17)); component_durability_lines(&s,&mut l,true); assert_eq!(l[1].0.value,"60/100"); assert_eq!(l[1].1,17); assert_eq!(l.len(),4); }
#[test] fn excludes_nonadvanced_undamaged_or_hidden() { for (damage,advanced,visible) in [(40,false,true),(0,true,true),(40,true,false)] { let mut s=stack(horizon_world::nbt::Tag::Int(100),damage); s.show_damage=visible; let mut l=lines(); component_durability_lines(&s,&mut l,advanced); assert_eq!(l.len(),3); } }
#[test] fn invalid_and_overdamaged() { for max in [horizon_world::nbt::Tag::Int(0), horizon_world::nbt::Tag::Int(-1), horizon_world::nbt::Tag::String] { let mut l=lines(); component_durability_lines(&stack(max,1),&mut l,true); assert_eq!(l.len(),3); } let mut l=lines(); component_durability_lines(&stack(horizon_world::nbt::Tag::Int(100),120),&mut l,true); assert_eq!(l[1].0.value,"0/100"); }

#[test] fn unbreakable_follows_host_damaged_display_rule() { let mut s=stack(horizon_world::nbt::Tag::Int(100),40); s.unbreakable=true; let mut l=lines(); component_durability_lines(&s,&mut l,true); assert_eq!(l[1].0.value,"60/100"); }
