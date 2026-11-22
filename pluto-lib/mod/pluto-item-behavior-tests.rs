// Compile the actual transport and HEAD adapter bodies against reduced snapshots,
// not reimplementations. This tests branching only, not native type bindings.
mod horizon_world {
    pub mod nbt {
        #[derive(Clone, Debug, PartialEq)]
        pub enum Tag {
            Compound(Vec<(String, Tag)>),
            String(String),
            Int(i32),
        }
    }
    pub mod level {
        pub mod pluto_item_behavior_access {
            include!("pluto-item-behavior-access.rs");
        }
    }
}
mod horizon_client_core {
    pub mod scoreboard {
        // The real conversion's shapes for the cases the adapters meet.
        pub fn tag_to_json(t: &super::super::horizon_world::nbt::Tag) -> serde_json::Value {
            use super::super::horizon_world::nbt::Tag;
            match t {
                Tag::Compound(c) => serde_json::Value::Object(
                    c.iter().map(|(k, v)| (k.clone(), tag_to_json(v))).collect(),
                ),
                Tag::String(s) => serde_json::Value::String(s.clone()),
                Tag::Int(v) => serde_json::Value::from(*v),
            }
        }
    }
}
use horizon_world::nbt::Tag;

struct ItemData {
    custom: Option<Tag>,
}
impl ItemData {
    fn other(&self, id: &str) -> Option<&Tag> {
        if id == "minecraft:custom_data" { self.custom.as_ref() } else { None }
    }
}
struct ItemStack {
    item: String,
    data: Option<ItemData>,
}
impl ItemStack {
    fn data(&self) -> Option<&ItemData> { self.data.as_ref() }
    fn with_custom(item: &str, custom: Tag) -> Self {
        Self { item: item.into(), data: Some(ItemData { custom: Some(custom) }) }
    }
    fn plain(item: &str) -> Self { Self { item: item.into(), data: None } }
}
struct Game<'a> {
    held: Option<ItemStack>,
    sheared: Vec<(usize, String)>,
    saddled: Vec<(usize, String)>,
    marker: std::marker::PhantomData<&'a ()>,
}
impl Game<'_> {
    fn new(held: Option<ItemStack>) -> Self {
        Self { held, sheared: Vec::new(), saddled: Vec::new(), marker: std::marker::PhantomData }
    }
    fn held(&self) -> Option<&ItemStack> { self.held.as_ref() }
    fn interact_shear_mob(&mut self, index: usize, item: &str) -> bool {
        self.sheared.push((index, item.to_owned()));
        true
    }
    fn shear_saddle(&mut self, index: usize, item: &str) -> Option<bool> {
        self.saddled.push((index, item.to_owned()));
        Some(false)
    }
}
include!("pluto-item-enchantability-head.rs");
include!("pluto-item-shear-head.rs");
include!("pluto-item-shear-saddle-head.rs");

#[test]
fn enchantability_and_tool_action_adapters_delegate_and_answer() {
    use horizon_world::level::pluto_item_behavior_access as access;
    use std::ops::ControlFlow::{Break, Continue};
    // No dispatchers installed yet: every query delegates to the original.
    let tool = ItemStack::with_custom(
        "test:tool",
        Tag::Compound(vec![("material".into(), Tag::String("test:gold".into()))]),
    );
    assert_eq!(pluto_item_enchantability(&tool), Continue(()));
    let mut game = Game::new(Some(ItemStack::plain("test:kama")));
    assert_eq!(pluto_shear_mob(&mut game, 3, "test:kama"), Continue(()));
    assert_eq!(pluto_shear_saddle(&mut game, 4, "test:kama"), Continue(()));
    assert!(game.sheared.is_empty() && game.saddled.is_empty());

    // Install both dispatchers; a second install of each fails.
    access::set_enchantability_dispatcher(|item, data| match item {
        "test:tool" => {
            assert_eq!(data["material"], serde_json::Value::String("test:gold".into()));
            Some(14)
        }
        // A negative answer must never reach the game as a wrapped u32.
        "test:cursed" => Some(-3),
        _ => None,
    })
    .unwrap();
    assert!(access::set_enchantability_dispatcher(|_, _| None).is_err());
    assert_eq!(pluto_item_enchantability(&tool), Break(14));
    assert_eq!(pluto_item_enchantability(&ItemStack::plain("test:cursed")), Break(0));
    assert_eq!(pluto_item_enchantability(&ItemStack::plain("test:other")), Continue(()));

    access::set_tool_action_dispatcher(|item, _, action| {
        assert_eq!(action, "shears_harvest");
        (item == "test:kama").then_some(true)
    })
    .unwrap();
    assert!(access::set_tool_action_dispatcher(|_, _, _| None).is_err());

    // The vanilla shears never re-ask the question.
    let mut game = Game::new(Some(ItemStack::plain("minecraft:shears")));
    assert_eq!(pluto_shear_mob(&mut game, 3, "minecraft:shears"), Continue(()));
    assert_eq!(pluto_shear_saddle(&mut game, 4, "minecraft:shears"), Continue(()));
    assert!(game.sheared.is_empty() && game.saddled.is_empty());
    // An unanswered stack runs the original gate and fails it as before.
    let mut game = Game::new(Some(ItemStack::plain("test:stick")));
    assert_eq!(pluto_shear_mob(&mut game, 3, "test:stick"), Continue(()));
    assert_eq!(pluto_shear_saddle(&mut game, 4, "test:stick"), Continue(()));
    assert!(game.sheared.is_empty() && game.saddled.is_empty());
    // A granted stack runs the original body under the vanilla name, result and all.
    let mut game = Game::new(Some(ItemStack::with_custom(
        "test:kama",
        Tag::Compound(vec![("tic_materials".into(), Tag::Int(3))]),
    )));
    assert_eq!(pluto_shear_mob(&mut game, 3, "test:kama"), Break(true));
    assert_eq!(game.sheared, vec![(3, "minecraft:shears".to_string())]);
    assert_eq!(pluto_shear_saddle(&mut game, 4, "test:kama"), Break(Some(false)));
    assert_eq!(game.saddled, vec![(4, "minecraft:shears".to_string())]);
}
