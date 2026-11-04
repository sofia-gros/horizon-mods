// Focused lifecycle fixture. ItemStack/NBT and all rules under test are real;
// the small Game double supplies only host start acceptance and actor storage.
#![allow(dead_code)]
extern crate self as horizon_client;
pub mod items { pub use horizon_items::items::*; }
pub mod first_person {
    #[derive(Debug)] pub enum UseResult { Fail, Success { swing: bool } }
}
#[path = "../../mixins/tconstruct/source-ranged-draw.rs"] mod tconstruct_ranged_draw;
#[path = "../../mixins/tconstruct/source-momentum-ranged-use.rs"] mod tconstruct_source_momentum_ranged_use;
#[path = "../../mixins/tconstruct/source-bow-start.rs"] mod tconstruct_source_bow_start;
mod bow_use {
    type Result<T> = std::result::Result<T, ()>;
    include!("../../mixins/tconstruct/material-bow-use.rs");
    pub fn run(game: &mut crate::game::Game<'_>) {
        let held = game.held().unwrap().clone();
        let _ = tconstruct_bow_use(game, &held);
    }
}
mod remote_start {
    include!("../../mixins/tconstruct/material-bow-remote-start.rs");
    pub fn run(game: &mut crate::game::Game<'_>) -> bool {
        let held = game.held().unwrap().clone();
        match tconstruct_remote_bow_start(game, &held) {
            std::ops::ControlFlow::Break(result) => result,
            _ => false,
        }
    }
}
mod game {
    use crate::items::ItemStack;
    pub(crate) use crate::{tconstruct_source_bow_start, tconstruct_source_momentum_ranged_use};
    pub mod tconstruct_source_ranged_wear {
        pub fn is_broken(stack: &crate::items::ItemStack) -> bool { stack.damage >= 100 }
    }
    pub enum GuestUse { OnBlock(()) }
    pub struct Effects(pub Option<i32>);
    impl Effects { pub fn amplifier(&self, _: &str) -> Option<i32> { self.0 } }
    pub struct Potions { pub effects: Effects }
    pub struct Game<'a> {
        pub inventory: Vec<Option<ItemStack>>, pub offhand: Option<ItemStack>, pub slot: usize,
        pub using: Option<(usize, u32)>, pub potions: Potions, pub canceled: bool,
        pub creative: bool, pub ammo: bool, pub starts: usize, pub replace_on_start: bool,
        pub guest_use: Option<GuestUse>, pub lifetime: std::marker::PhantomData<&'a ()>,
    }
    impl Game<'_> {
        pub fn held(&self) -> Option<&ItemStack> {
            if self.slot == 40 { self.offhand.as_ref() } else { self.inventory[self.slot].as_ref() }
        }
        pub fn stack_in_mut(&mut self, slot: usize) -> &mut Option<ItemStack> {
            if slot == 40 { &mut self.offhand } else { &mut self.inventory[slot] }
        }
        pub fn projectile_slot(&self, _: &str) -> Option<usize> { self.ammo.then_some(1) }
        pub fn start_using_item(&mut self) -> bool {
            self.starts += 1;
            if self.canceled || self.using.is_some() { return false; }
            self.using = Some((self.slot, 0));
            if self.replace_on_start { *self.stack_in_mut(self.slot) = Some(ItemStack::new("tconstruct:crossbow", 1)); }
            true
        }
    }
}
use horizon_world::nbt::Tag;
fn actor(offhand: bool) -> game::Game<'static> {
    let mut stack = items::ItemStack::new("tconstruct:longbow", 1);
    stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([
        ("tic_draw_speed", Tag::Float(1.0f32.to_bits())), ("tic_momentum", Tag::Int(1)),
    ])));
    game::Game {
        inventory: vec![(!offhand).then_some(stack.clone()), None], offhand: offhand.then_some(stack),
        slot: if offhand { 40 } else { 0 }, using: None, potions: game::Potions { effects: game::Effects(Some(9)) },
        canceled: false, creative: false, ammo: true, starts: 0, replace_on_start: false,
        guest_use: None, lifetime: std::marker::PhantomData,
    }
}
#[test]
fn accepted_start_captures_actor_effect_and_offhand_without_reapplying() {
    let mut game = actor(true);
    assert!(remote_start::run(&mut game));
    assert_eq!(game.using, Some((40, 0)));
    assert_eq!(tconstruct_ranged_draw::draw_ticks(game.held().unwrap()), 16);
    game.potions.effects.0 = None;
    assert!(remote_start::run(&mut game));
    assert_eq!(game.starts, 1);
    assert_eq!(tconstruct_ranged_draw::draw_ticks(game.held().unwrap()), 16);
    game.using = None;
    assert!(remote_start::run(&mut game));
    assert_eq!(tconstruct_ranged_draw::draw_ticks(game.held().unwrap()), 20);
}
#[test]
fn no_ammo_or_canceled_start_cannot_capture_a_bonus() {
    let mut game = actor(false);
    game.ammo = false;
    bow_use::run(&mut game);
    assert_eq!(game.starts, 0);
    assert_eq!(game.using, None);
    game.ammo = true;
    game.canceled = true;
    tconstruct_ranged_draw::snapshot(game.stack_in_mut(0).as_mut().unwrap(), 10);
    bow_use::run(&mut game);
    assert_eq!(game.starts, 1);
    assert_eq!(game.using, None);
    assert_eq!(tconstruct_ranged_draw::draw_ticks(game.held().unwrap()), 20);
}
#[test]
fn another_actor_and_a_replaced_item_never_inherit_the_snapshot() {
    let mut first = actor(false);
    bow_use::run(&mut first);
    let mut second = actor(false);
    second.potions.effects.0 = None;
    second.inventory[0] = first.inventory[0].clone();
    bow_use::run(&mut second);
    assert_eq!(tconstruct_ranged_draw::draw_ticks(second.held().unwrap()), 20);
    assert_eq!(tconstruct_ranged_draw::draw_ticks(first.held().unwrap()), 16);
    first.using = None;
    first.replace_on_start = true;
    bow_use::run(&mut first);
    assert_eq!(tconstruct_ranged_draw::draw_ticks(first.held().unwrap()), 20);
    assert!(first.held().unwrap().data().and_then(|d| d.other("minecraft:custom_data")).is_none());
}
#[test]
fn interrupted_old_inventory_snapshot_is_removed_on_next_start() {
    let mut game = actor(false);
    bow_use::run(&mut game);
    game.offhand = game.inventory[0].clone();
    game.using = None;
    game.slot = 40;
    game.potions.effects.0 = None;
    assert!(remote_start::run(&mut game));
    assert_eq!(tconstruct_ranged_draw::draw_ticks(game.inventory[0].as_ref().unwrap()), 20);
    assert_eq!(tconstruct_ranged_draw::draw_ticks(game.held().unwrap()), 20);
}
