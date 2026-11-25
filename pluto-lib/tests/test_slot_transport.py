"""Production slot conversion over a minimal native value double, not game evidence."""
from pathlib import Path
import os
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class SlotTransport(unittest.TestCase):
    def test_moving_replacing_and_editing_slots_keep_source_components(self):
        public = (ROOT / 'pluto-lib/mod/api/src/pure/block_entity_api_v1.rs').read_text(encoding='utf-8')
        public = public.split('// Host-owned value', 1)[1].split('/// Fluid in a tank', 1)[0]
        hooks = (ROOT / 'pluto-lib/mod/api/src/hooks.rs').read_text(encoding='utf-8')
        adapter = 'fn slot_of' + hooks.split('fn slot_of', 1)[1].split('fn layer_to_native', 1)[0]
        fixture = r'''
use std::sync::Arc;
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq)]
pub struct Identifier(String);
impl Identifier { fn parse(s: &str)->Result<Self, ()> { Ok(Self(s.into())) } }
impl std::fmt::Display for Identifier {
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result { f.write_str(&self.0) }
}
mod pure {
    pub mod base { pub(crate) use crate::{Identifier, Nbt}; }
    pub mod player_api {
        // Only the stack's own custom_model_data matters to slots; the fixture stacks carry none.
        #[derive(Clone, Debug, PartialEq, Default)]
        pub struct CustomModelData { pub strings: Vec<String> }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Nbt { Byte(i8), Short(i16), Int(i32), Long(i64), Float(f32), Double(f64), ByteArray(Vec<i8>), String(String), List(Vec<Nbt>), Compound(NbtCompound), IntArray(Vec<i32>), LongArray(Vec<i64>) }
type NbtCompound = BTreeMap<String, Nbt>;
#[derive(Clone, Debug, PartialEq)]
struct ModBlockEntity { nbt:NbtCompound, items:Vec<Option<ItemStack>>, data:Vec<i32> }
fn compound_from_tag(c:&NbtCompound)->NbtCompound { c.clone() }
#[derive(Clone, Debug, PartialEq, Default)]
struct Extra { max: Option<i32>, custom: NbtCompound, name: String, enchantments: Vec<String>, wire: Vec<u8>, map_id: Option<i32> }
enum Tag { Int(i32), Compound(NbtCompound), List(Vec<Tag>) }
impl Extra { fn set_other(&mut self, key:&str, tag:Option<Tag>) {
    match (key, tag) {
        ("minecraft:max_damage", Some(Tag::Int(v))) => self.max=Some(v),
        ("minecraft:max_damage", None) => self.max=None,
        ("minecraft:custom_data", Some(Tag::Compound(v))) => self.custom=v,
        ("minecraft:custom_data", None) => self.custom.clear(),
        ("minecraft:custom_model_data", None) => {}
        _ => panic!("unexpected component"),
    }
} }
#[derive(Clone, Debug, PartialEq)]
struct ItemStack { item: Arc<str>, count:u32, damage:u32, extra:Extra }
// The adapter reads the optional minecraft:tool component; the fixture stacks carry none.
impl Extra { fn other(&self, _key:&str)->Option<&Tag> { None } }
impl ItemStack { fn data(&self)->Option<&Extra> { Some(&self.extra) } }
impl ItemStack {
    fn new(item:&str,count:u32)->Self { Self {item:Arc::from(item),count,damage:0,extra:Extra::default()} }
    fn data_mut(&mut self)->&mut Extra { &mut self.extra }
    fn tidy(&mut self) {}
}
struct PublicData { count:u32, damage:u32, max_damage:Option<i32>, custom_data:NbtCompound, custom_model_data:Option<pure::player_api::CustomModelData>, can_place_on:Option<Nbt> }
fn read_stack(s:&ItemStack)->PublicData { PublicData {count:s.count,damage:s.damage,max_damage:s.extra.max,custom_data:s.extra.custom.clone(),custom_model_data:None,can_place_on:read_can_place_on(s)} }
fn read_custom_model_data(_:&ItemStack)->Option<pure::player_api::CustomModelData> { None }
fn custom_model_data_tag(_:&Option<pure::player_api::CustomModelData>)->Option<Tag> { None }
fn compound_to_tag(c:&NbtCompound)->NbtCompound { c.clone() }
// The adapter reads the optional minecraft:profile component the same way; the fixture stacks carry none.
fn read_profile(_:&ItemStack)->Option<NbtCompound> { None }
// The stack data's can_place_on is read the same way in production read_stack; the fixture stacks carry none.
fn read_can_place_on(_:&ItemStack)->Option<Nbt> { None }
// The adapter writes a slot's can_place_on back through tag_from_nbt. The fixture's Tag carries
// only the int/list/compound shapes and no fixture stack carries the component, so this double is
// shaped on the production conversion but never exercised.
fn tag_from_nbt(nbt:&Nbt)->Tag {
    match nbt {
        Nbt::Int(value) => Tag::Int(*value),
        Nbt::List(values) => Tag::List(values.iter().map(tag_from_nbt).collect()),
        Nbt::Compound(compound) => Tag::Compound(compound_to_tag(compound)),
        _ => Tag::Int(0),
    }
}
// The loot transport rides along in the adapter slice (its functions sit between the slot
// conversion's own in hooks.rs) but is not exercised here; these doubles only let it compile.
mod serde_json {
    #[derive(Clone, Debug, PartialEq)]
    pub enum Value { Null, Number(Number), String(String), Array(Vec<Value>), Object(Map) }
    #[derive(Clone, Debug, PartialEq)]
    pub struct Number(f64);
    impl Number {
        pub fn from_f64(value: f64) -> Option<Self> { value.is_finite().then(|| Self(value)) }
        pub fn as_i64(&self) -> Option<i64> { Some(self.0 as i64) }
    }
    pub type Map = std::collections::BTreeMap<String, Value>;
    impl From<i8> for Value { fn from(value: i8) -> Self { Self::Number(Number(f64::from(value))) } }
    impl From<i16> for Value { fn from(value: i16) -> Self { Self::Number(Number(f64::from(value))) } }
    impl From<i32> for Value { fn from(value: i32) -> Self { Self::Number(Number(f64::from(value))) } }
    impl From<i64> for Value { fn from(value: i64) -> Self { Self::Number(Number(value as f64)) } }
    impl From<String> for Value { fn from(value: String) -> Self { Self::String(value) } }
}
mod horizon_client {
    pub mod mod_api {
        #[derive(Clone, Debug, PartialEq)]
        pub struct LootStack { pub item: String, pub count: i32, pub damage: i32, pub components: Vec<(String, crate::serde_json::Value)>, pub enchantments: Vec<(String, i32)> }
        // The launch-stack noting in the adapter slice reads the launch event's entity and
        // player; the name and kind ride along as the host's event carries them.
        #[derive(Clone, Debug, PartialEq)]
        pub struct ProjectileLaunchEvent<'a> { pub entity: u32, pub player: Option<u128>, pub name: &'a str, pub kind: &'a str }
    }
}
fn nbt_from_json(value:&serde_json::Value)->NbtCompound {
    match value { serde_json::Value::Object(map) => map.iter().map(|(key, value)| (key.clone(), nbt_value_from_json(value))).collect(), _ => NbtCompound::new() }
}
fn nbt_value_from_json(value:&serde_json::Value)->Nbt {
    match value {
        serde_json::Value::Number(number) => Nbt::Int(number.as_i64().unwrap_or(0) as i32),
        serde_json::Value::String(text) => Nbt::String(text.clone()),
        serde_json::Value::Array(items) => Nbt::List(items.iter().map(nbt_value_from_json).collect()),
        serde_json::Value::Object(_) => Nbt::Compound(nbt_from_json(value)),
        serde_json::Value::Null => Nbt::Int(0),
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerPlayer { pub entity_id: i32, pub name: String, pub uuid: String }
trait Host {
    fn server_players(&mut self) -> Vec<ServerPlayer>;
    fn acting_player(&mut self) -> Option<ServerPlayer>;
    fn entity(&mut self, id: u32) -> Option<u32>;
    fn entity_nbt(&mut self, entity: u32) -> Option<NbtCompound>;
    fn held_stack(&mut self, player: u128) -> Option<ItemStack>;
    fn entity_by_uuid(&mut self, uuid: u128) -> Option<(u32, u32, u32)>;
}
mod loot_api_v2 {
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct PlayerProfile { pub name: String, pub uuid: String }
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct LootEntityFacts { pub player: Option<PlayerProfile> }
    pub fn facts_from_saved(_compound: &crate::NbtCompound, _kind: &str) -> LootEntityFacts { LootEntityFacts::default() }
}
// The launch-stack noting rides along in the adapter slice (note_launch_stack sits between
// the slot conversion's own functions in hooks.rs) but is not exercised here; these doubles
// only let it compile.
mod entity_events_v1 {
    pub fn stack_from_saved_item(_stack: &crate::NbtCompound) -> Option<crate::PublicData> { None }
    pub fn saved_mainhand(_data: &crate::NbtCompound) -> Option<crate::PublicData> { None }
    pub fn note_projectile_tool(_projectile: u32, _tool: crate::PublicData) {}
}
/* PUBLIC */
/* ADAPTER */
#[test]
fn transport() {
    let mut source=ItemStack::new("test:tool",4);
    source.extra=Extra {max:Some(80),custom:BTreeMap::from([("material".into(),Nbt::Int(7))]),name:"original".into(),enchantments:vec!["test:enchantment".into()],wire:vec![0,255,3],map_id:None};
    source.damage=6;
    let mut other=source.clone(); other.extra.name="other source".into();
    let mut entity=ModBlockEntity {nbt:BTreeMap::new(),items:vec![Some(source.clone()),Some(other.clone()),None],data:vec![]};
    assert!(with_block_entity(&mut entity, |_,slots| { slots.swap(0,1); slots[2]=slots[0].take(); true }));
    assert_eq!(entity.items,vec![None,Some(source.clone()),Some(other)]);
    let unchanged=entity.clone();
    assert!(!with_block_entity(&mut entity, |_,slots| { slots.clear(); false }));
    assert_eq!(entity,unchanged);
    let before_source=source.clone();
    assert!(!with_quick_move(&mut entity,&mut source,|remainder,data,slots| {
        remainder.count=0; remainder.damage=99;
        data.insert("changed".into(),Nbt::Int(1)); slots.clear(); false
    }));
    assert_eq!(source,before_source);
    assert_eq!(entity,unchanged);
    let mut remainder=source.clone();
    assert!(with_quick_move(&mut entity,&mut remainder,|stack,data,slots| {
        let mut transferred=stack.clone(); transferred.count=3;
        slots[0]=Some(transferred);
        stack.count=1; stack.damage=10; stack.max_damage=None; stack.custom_data.clear();
        data.insert("changed".into(),Nbt::Int(1)); true
    }));
    assert_eq!((remainder.count,remainder.damage),(1,10));
    assert_eq!(remainder.extra.max,None);
    assert!(remainder.extra.custom.is_empty());
    assert_eq!(remainder.extra.name,source.extra.name);
    assert_eq!(remainder.extra.enchantments,source.extra.enchantments);
    assert_eq!(remainder.extra.wire,source.extra.wire);
    let transferred=entity.items[0].as_ref().unwrap();
    assert_eq!(transferred.count,3);
    assert_eq!(transferred.extra,source.extra);
    assert_eq!(entity.nbt["changed"],Nbt::Int(1));
    assert!(with_quick_move(&mut entity,&mut remainder,|stack,_,_| {
        *stack=ItemSlot::new(Identifier::parse("test:replacement").unwrap(),2); true
    }));
    assert_eq!(remainder,ItemStack::new("test:replacement",2));
    assert!(with_block_entity(&mut entity, |_,slots| {
        slots[0].as_mut().unwrap().count=0; true
    }));
    assert_eq!(entity.items[0],None);
    let slot=slot_of(&source).unwrap();
    assert_eq!(slot,slot_of(&source).unwrap());
    let mut destination=ItemStack::new("test:tool",1);
    destination.extra.name="destination must not win".into();
    let mut moved=slot.clone(); moved.count=2; moved.damage=9;
    write_slot(&mut destination,&moved);
    assert_eq!(destination.extra,source.extra);
    assert_eq!((destination.count,destination.damage),(2,9));
    moved.custom_data.insert("material".into(),Nbt::Int(12)); moved.max_damage=Some(100);
    write_slot(&mut destination,&moved);
    assert_eq!(destination.extra.custom["material"],Nbt::Int(12));
    assert_eq!(destination.extra.max,Some(100));
    assert_eq!(destination.extra.name,source.extra.name);
    assert_eq!(source.extra.custom["material"],Nbt::Int(7));
    moved.item=Identifier::parse("test:other").unwrap();
    write_slot(&mut destination,&moved);
    assert!(destination.extra.name.is_empty());
    assert!(destination.extra.enchantments.is_empty());
    let plain=ItemSlot::new(Identifier::parse("test:tool").unwrap(),1);
    write_slot(&mut source,&plain);
    assert_eq!(source,ItemStack::new("test:tool",1));
}
'''
        source = fixture.replace('/* PUBLIC */', '// Host-owned value' + public).replace('/* ADAPTER */', adapter)
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            code, binary = folder / 'transport.rs', folder / ('transport.exe' if os.name == 'nt' else 'transport')
            code.write_text(source, encoding='utf-8', newline='\n')
            subprocess.run(['rustc', '+1.95.0', '--edition=2021', '--test', str(code), '-o', str(binary)], check=True)
            subprocess.run([str(binary)], check=True)


if __name__ == '__main__':
    unittest.main()
