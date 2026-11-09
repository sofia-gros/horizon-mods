//! A handle to a connected player, the counterpart of Fabric's `ServerPlayerEntity` for what mods do most: tell the player something and
//! give them items.
//!
//! The handle is plain data (uuid and name). Its methods reach the game through a *scope* that PlutoLib opens around every callback
//! that receives a player ([`with_ops`]); called outside such a scope they do nothing and report failure, so a stored handle can never touch
//! the game from the wrong place.

use std::cell::Cell;
use std::marker::PhantomData;

/// What the game can do for a player. Implemented once, in `hooks.rs`, over Horizon's `Host`.
pub trait PlayerOps {
    /// Sends a system chat line to the player; false when the player is not online.
    fn tell(&mut self, player: u128, line: &str) -> bool;
    /// Gives items to the player; returns how many did not fit.
    fn give(&mut self, player: u128, item: &str, count: u32) -> u32;
    /// The JSON stored under the attachment `id` for the player, if any (does not create it).
    fn attachment_get(&mut self, player: u128, id: &str) -> Option<String>;
    /// Stores JSON under the attachment `id` for the player; false if it could not be stored.
    fn attachment_set(&mut self, player: u128, id: &str, json: &str) -> bool;
    /// How many plain `item` the player carries in the main inventory.
    fn count_item(&mut self, player: u128, item: &str) -> u32;
    /// Takes up to `count` plain `item` from the player; returns how many were taken.
    fn take_item(&mut self, player: u128, item: &str, count: u32) -> u32;
    /// How many items of the item tag `tag` (`minecraft:logs`, `c:gems/diamond`) the player carries in the main inventory.
    fn count_in_tag(&mut self, _player: u128, _tag: &str) -> u32 {
        0
    }
    /// Takes up to `count` items of the item tag `tag` from the player; returns how many were taken.
    fn take_in_tag(&mut self, _player: u128, _tag: &str, _count: u32) -> u32 {
        0
    }
    /// How many of the 36 main inventory slots are empty.
    fn empty_slots(&mut self, player: u128) -> u32;
    /// The language option of the player's client (`en_us`, `ja_jp`, ...); None if the player is not known.
    fn language(&mut self, player: u128) -> Option<String>;
    /// Gives the player a status effect; true if anything changed.
    fn add_effect(&mut self, player: u128, effect: &EffectInstance) -> bool;
    /// Takes the effect `id` off the player; true if they had it.
    fn remove_effect(&mut self, player: u128, id: &str) -> bool;
    /// The status effects the player has; None if the player is not known.
    fn effects(&mut self, player: u128) -> Option<Vec<EffectInstance>>;
    /// The player's food level (0 to 20), saturation and exhaustion; None if the player is not known.
    fn food(&mut self, player: u128) -> Option<FoodStatus>;
    /// Feeds a reached player as FoodData.eat; false if unsupported or invalid.
    fn add_food(&mut self, _player: u128, _nutrition: i32, _saturation: f32) -> bool {
        false
    }
    /// The stack in main inventory `slot` (0 to 35, the hotbar first), with its custom data; None for an empty or unknown slot.
    fn stack(&mut self, _player: u128, _slot: i32) -> Option<StackData> {
        None
    }
    /// Gives a stack with its damage, max damage and custom data; returns what did not fit (the whole stack if nothing could be given).
    fn give_stack(&mut self, _player: u128, stack: StackData) -> Option<StackData> {
        Some(stack)
    }
    /// Replaces count, damage and custom data of the stack in `slot` if it still holds `expected_item`; true if it did.
    fn update_stack(&mut self, _player: u128, _slot: i32, _expected_item: &str, _new: &StackData) -> bool {
        false
    }
}

/// FoodData.eat arithmetic, shared by the real adapter and the in-memory backend.
pub(crate) fn fed_food(before: FoodStatus, nutrition: i32, modifier: f32) -> Option<FoodStatus> {
    let gained = nutrition as f32 * modifier * 2.0;
    let saturation = before.saturation + gained;
    if !modifier.is_finite() || !before.saturation.is_finite() || !before.exhaustion.is_finite() || !saturation.is_finite() {
        return None;
    }
    let level = before.level.wrapping_add(nutrition).clamp(0, 20);
    Some(FoodStatus { level, saturation: saturation.clamp(0.0, level as f32), exhaustion: before.exhaustion })
}

/// An item stack as a mod sees it: what it is, how many, how worn, and the data a mod keeps on it (Minecraft: `ItemStack` with the
/// `minecraft:custom_data` component).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StackData {
    /// The item id, such as `tconstruct:pickaxe`.
    pub item: String,
    /// How many.
    pub count: u32,
    /// Damage taken, 0 for a new item.
    pub damage: u32,
    /// The most damage this stack takes before it breaks, when it differs from the item's (Minecraft: the `minecraft:max_damage`
    /// component); None for the item's own. A tool whose durability depends on its materials sets it.
    pub max_damage: Option<i32>,
    /// The custom data compound, empty for none.
    pub custom_data: crate::pure::base::NbtCompound,
}

thread_local! {
    static OPS: Cell<Option<*mut dyn PlayerOps>> = const { Cell::new(None) };
}

/// Restores the previous scope when dropped, also on panic.
struct Scope<'a> {
    previous: Option<*mut dyn PlayerOps>,
    _borrow: PhantomData<&'a mut dyn PlayerOps>,
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        OPS.with(|slot| slot.set(self.previous));
    }
}

/// Runs `f` with `ops` available to every [`ServerPlayer`] method called (on this thread) during `f`.
pub fn with_ops<R>(ops: &mut dyn PlayerOps, f: impl FnOnce() -> R) -> R {
    // SAFETY: the pointer is only dereferenced while `f` runs, strictly inside the lifetime of `ops`; `Scope` clears it on every exit.
    let raw: *mut dyn PlayerOps = unsafe { std::mem::transmute::<&mut dyn PlayerOps, *mut dyn PlayerOps>(ops) };
    let previous = OPS.with(|slot| slot.replace(Some(raw)));
    let _scope = Scope { previous, _borrow: PhantomData };
    f()
}

pub(super) fn with_current<R>(default: R, f: impl FnOnce(&mut dyn PlayerOps) -> R) -> R {
    match OPS.with(Cell::get) {
        // SAFETY: see `with_ops`: the pointer is valid for as long as it is stored.
        Some(raw) => f(unsafe { &mut *raw }),
        None => default,
    }
}

/// A status effect on a player (Minecraft: `MobEffectInstance`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectInstance {
    /// The effect, such as `minecraft:speed` or a mod's own.
    pub id: String,
    /// 0 for level I.
    pub amplifier: u8,
    /// Ticks left; -1 for an infinite one.
    pub duration: i32,
    /// From a beacon or a conduit: translucent particles and a framed icon.
    pub ambient: bool,
    /// Whether the effect shows particles.
    pub particles: bool,
}

impl EffectInstance {
    /// An effect of level I lasting `duration` ticks (20 are one second; -1 never ends), with particles.
    pub fn new(id: &str, duration: i32) -> Self {
        Self { id: id.to_owned(), amplifier: 0, duration, ambient: false, particles: true }
    }

    /// The same effect at another level (0 is level I).
    pub fn amplifier(mut self, amplifier: u8) -> Self {
        self.amplifier = amplifier;
        self
    }

    /// The same effect without particles.
    pub fn without_particles(mut self) -> Self {
        self.particles = false;
        self
    }
}

/// A player's hunger (Minecraft: `FoodData`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoodStatus {
    /// The food level, 0 to 20.
    pub level: i32,
    /// The saturation.
    pub saturation: f32,
    /// The exhaustion.
    pub exhaustion: f32,
}

/// A connected player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerPlayer {
    pub uuid: u128,
    pub name: String,
}

impl ServerPlayer {
    /// Fabric: `ServerPlayerEntity.sendMessage(Text)`. False if the player is gone or this is called outside a callback.
    pub fn send_message(&self, text: &str) -> bool {
        with_current(false, |ops| ops.tell(self.uuid, text))
    }

    /// Fabric: `PlayerInventory.offerOrDrop`, roughly: gives `count` of `item`; returns how many did not fit (`count` if called outside a callback).
    pub fn give_item(&self, item: &str, count: u32) -> u32 {
        with_current(count, |ops| ops.give(self.uuid, item, count))
    }

    /// Fabric: `PlayerInventory.count(item)`. 0 outside a callback that carries this player.
    pub fn count_item(&self, item: &str) -> u32 {
        with_current(0, |ops| ops.count_item(self.uuid, item))
    }

    /// How many items of the item tag `tag` the player carries in the main inventory (0 outside a callback).
    pub fn count_in_tag(&self, tag: &str) -> u32 {
        with_current(0, |ops| ops.count_in_tag(self.uuid, tag))
    }

    /// Takes up to `count` items of the item tag `tag` from the player; returns how many were taken (0 outside a callback).
    pub fn take_in_tag(&self, tag: &str, count: u32) -> u32 {
        with_current(0, |ops| ops.take_in_tag(self.uuid, tag, count))
    }

    /// Fabric: `PlayerInventory.remove` (by item): takes up to `count`; returns how many were taken (0 outside a callback).
    pub fn take_item(&self, item: &str, count: u32) -> u32 {
        with_current(0, |ops| ops.take_item(self.uuid, item, count))
    }

    /// The language option of the player's client (`en_us`, `ja_jp`, ...); `en_us` outside a callback that carries this player. Use it to write
    /// a message in the player's language with `I18n::translate_in`.
    pub fn language(&self) -> String {
        with_current(None, |ops| ops.language(self.uuid)).unwrap_or_else(|| "en_us".to_owned())
    }

    /// Fabric: `LivingEntity.addStatusEffect`. Gives the player a status effect; it replaces a weaker or shorter one of the same kind. False if
    /// nothing changed (an unknown effect, a duration of 0, a dead player) or the call is made outside a callback that carries the player.
    pub fn add_effect(&self, effect: &EffectInstance) -> bool {
        with_current(false, |ops| ops.add_effect(self.uuid, effect))
    }

    /// Fabric: `LivingEntity.removeStatusEffect`. Takes the effect `id` off the player; true if they had it.
    pub fn remove_effect(&self, id: &str) -> bool {
        with_current(false, |ops| ops.remove_effect(self.uuid, id))
    }

    /// The status effects the player has; empty outside a callback that carries the player.
    pub fn effects(&self) -> Vec<EffectInstance> {
        with_current(None, |ops| ops.effects(self.uuid)).unwrap_or_default()
    }

    /// Fabric: `PlayerEntity.getHungerManager`. The player's hunger; None outside a callback that carries the player.
    pub fn food(&self) -> Option<FoodStatus> {
        with_current(None, |ops| ops.food(self.uuid))
    }

    /// Feeds the player as FoodData.eat: saturation is a modifier, adding nutrition * modifier * 2 before capping at the new food level.
    /// Exhaustion is preserved. False outside a player scope, for an unavailable player or a non-finite calculation.
    pub fn add_food(&self, nutrition: i32, saturation: f32) -> bool {
        if !saturation.is_finite() { return false; }
        with_current(false, |ops| ops.add_food(self.uuid, nutrition, saturation))
    }

    /// The stack in main inventory slot `slot` (0 to 35, the hotbar first) with its custom data; None for an empty slot, or outside a callback.
    pub fn stack(&self, slot: i32) -> Option<StackData> {
        with_current(None, |ops| ops.stack(self.uuid, slot))
    }

    /// Gives the player a stack with its damage, max damage and custom data (a tool made by a mod). Returns what did not fit; the whole
    /// stack comes back outside a callback that carries the player.
    pub fn give_stack(&self, stack: StackData) -> Option<StackData> {
        with_current(Some(stack.clone()), |ops| ops.give_stack(self.uuid, stack))
    }

    /// Writes back count, damage and custom data of the stack in `slot`, if it still holds the item `new.item`. True if it was written;
    /// false outside a callback, for an empty slot, or when the slot holds another item.
    pub fn update_stack(&self, slot: i32, new: &StackData) -> bool {
        with_current(false, |ops| ops.update_stack(self.uuid, slot, &new.item, new))
    }

    /// Empty slots in the 36-slot main inventory (Fabric: counting `PlayerInventory.getEmptySlot`). 0 outside a callback.
    pub fn empty_slots(&self) -> u32 {
        with_current(0, |ops| ops.empty_slots(self.uuid))
    }
}

/// An in-memory game for tests: the players it knows keep messages, items and attachments in plain collections. Run code that uses
/// [`ServerPlayer`] inside [`with_ops`] with one of these to check what it did.
#[derive(Default, Debug)]
pub struct MemoryPlayerOps {
    /// Every `(player, line)` sent with `send_message`.
    pub told: Vec<(u128, String)>,
    /// Items each player carries, by `(player, item id)`.
    pub items: std::collections::HashMap<(u128, String), u32>,
    /// The members of each item tag, for `count_in_tag` and `take_in_tag`.
    pub item_tags: std::collections::HashMap<String, Vec<String>>,
    /// JSON stored per `(player, attachment id)`.
    pub attachments: std::collections::HashMap<(u128, String), String>,
    /// How many of the 36 main slots are empty for every player (set it to test full inventories).
    pub free_slots: u32,
    /// The language option of every player's client; empty means `en_us`.
    pub language: String,
    /// The status effects of every player, by `(player, effect id)`.
    pub effects: std::collections::BTreeMap<(u128, String), EffectInstance>,
    /// The food status every player has; None means a full, saturated player.
    pub food: Option<FoodStatus>,
    /// The main inventory slots of every player, by player; a missing player has none.
    pub stacks: std::collections::HashMap<u128, Vec<Option<StackData>>>,
}

impl PlayerOps for MemoryPlayerOps {
    fn tell(&mut self, player: u128, line: &str) -> bool {
        self.told.push((player, line.to_owned()));
        true
    }
    fn give(&mut self, player: u128, item: &str, count: u32) -> u32 {
        *self.items.entry((player, item.to_owned())).or_default() += count;
        0
    }
    fn attachment_get(&mut self, player: u128, id: &str) -> Option<String> {
        self.attachments.get(&(player, id.to_owned())).cloned()
    }
    fn attachment_set(&mut self, player: u128, id: &str, json: &str) -> bool {
        self.attachments.insert((player, id.to_owned()), json.to_owned());
        true
    }
    fn count_item(&mut self, player: u128, item: &str) -> u32 {
        self.items.get(&(player, item.to_owned())).copied().unwrap_or(0)
    }
    fn take_item(&mut self, player: u128, item: &str, count: u32) -> u32 {
        let have = self.items.entry((player, item.to_owned())).or_default();
        let taken = (*have).min(count);
        *have -= taken;
        taken
    }
    fn count_in_tag(&mut self, player: u128, tag: &str) -> u32 {
        self.item_tags.get(tag).map(|members| members.iter().map(|item| self.items.get(&(player, item.clone())).copied().unwrap_or(0)).sum()).unwrap_or(0)
    }
    fn take_in_tag(&mut self, player: u128, tag: &str, count: u32) -> u32 {
        let members = self.item_tags.get(tag).cloned().unwrap_or_default();
        let mut taken = 0;
        for item in members {
            if taken >= count {
                break;
            }
            taken += self.take_item(player, &item, count - taken);
        }
        taken
    }
    fn empty_slots(&mut self, _player: u128) -> u32 {
        self.free_slots
    }
    fn language(&mut self, _player: u128) -> Option<String> {
        Some(if self.language.is_empty() { "en_us".to_owned() } else { self.language.clone() })
    }
    fn add_effect(&mut self, player: u128, effect: &EffectInstance) -> bool {
        if effect.duration == 0 || effect.duration < -1 {
            return false;
        }
        self.effects.insert((player, effect.id.clone()), effect.clone());
        true
    }
    fn remove_effect(&mut self, player: u128, id: &str) -> bool {
        self.effects.remove(&(player, id.to_owned())).is_some()
    }
    fn effects(&mut self, player: u128) -> Option<Vec<EffectInstance>> {
        Some(self.effects.iter().filter(|((p, _), _)| *p == player).map(|(_, e)| e.clone()).collect())
    }
    fn food(&mut self, _player: u128) -> Option<FoodStatus> {
        Some(self.food.unwrap_or(FoodStatus { level: 20, saturation: 5.0, exhaustion: 0.0 }))
    }
    fn add_food(&mut self, player: u128, nutrition: i32, saturation: f32) -> bool {
        let Some(after) = self.food(player).and_then(|before| fed_food(before, nutrition, saturation)) else { return false };
        self.food = Some(after);
        true
    }
    fn stack(&mut self, player: u128, slot: i32) -> Option<StackData> {
        self.stacks.get(&player)?.get(usize::try_from(slot).ok()?)?.clone().filter(|s| s.count > 0)
    }
    fn give_stack(&mut self, player: u128, stack: StackData) -> Option<StackData> {
        let slots = self.stacks.entry(player).or_default();
        if let Some(index) = slots.iter().position(Option::is_none) {
            slots[index] = Some(stack);
            None
        } else if slots.len() < 36 {
            slots.push(Some(stack));
            None
        } else {
            Some(stack)
        }
    }
    fn update_stack(&mut self, player: u128, slot: i32, expected_item: &str, new: &StackData) -> bool {
        let Some(held) = usize::try_from(slot).ok().and_then(|i| self.stacks.get_mut(&player)?.get_mut(i)) else { return false };
        match held {
            Some(current) if current.item == expected_item && new.item == expected_item => {
                *held = if new.count == 0 { None } else { Some(new.clone()) };
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod stack_tests {
    use super::*;
    use crate::pure::base::Nbt;

    fn pickaxe(damage: u32) -> StackData {
        StackData { item: "tconstruct:pickaxe".into(), count: 1, damage, max_damage: Some(60), custom_data: [("stats".to_owned(), Nbt::Int(60))].into() }
    }

    #[test]
    fn a_stack_made_by_a_mod_can_be_given_with_its_data() {
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 1, name: "Alex".into() };
        with_ops(&mut ops, || {
            assert_eq!(player.give_stack(pickaxe(0)), None);
            assert_eq!(player.stack(0).unwrap().custom_data["stats"], Nbt::Int(60));
        });
        // a full inventory hands the stack back
        ops.stacks.insert(1, vec![Some(pickaxe(0)); 36]);
        assert_eq!(with_ops(&mut ops, || player.give_stack(pickaxe(1))), Some(pickaxe(1)));
        assert_eq!(player.give_stack(pickaxe(2)), Some(pickaxe(2)), "outside a callback nothing is given");
    }

    #[test]
    fn a_stack_can_be_read_and_written_back_with_its_data() {
        let mut ops = MemoryPlayerOps::default();
        ops.stacks.insert(1, vec![None, Some(pickaxe(0))]);
        let player = ServerPlayer { uuid: 1, name: "Alex".into() };
        with_ops(&mut ops, || {
            assert_eq!(player.stack(0), None);
            let mut stack = player.stack(1).unwrap();
            assert_eq!(stack.custom_data["stats"], Nbt::Int(60));
            assert_eq!(stack.max_damage, Some(60));
            stack.damage = 7;
            stack.max_damage = Some(120);
            stack.custom_data.insert("stats".into(), Nbt::Int(61));
            assert!(player.update_stack(1, &stack));
            assert_eq!(player.stack(1).unwrap().damage, 7);
            assert_eq!(player.stack(1).unwrap().max_damage, Some(120));
            // another item in the slot is left alone
            let other = StackData { item: "minecraft:stick".into(), ..stack.clone() };
            assert!(!player.update_stack(1, &other));
            assert!(!player.update_stack(5, &stack), "no such slot");
            // a count of 0 empties the slot
            let empty = StackData { count: 0, ..stack };
            assert!(player.update_stack(1, &empty));
            assert_eq!(player.stack(1), None);
        });
        assert_eq!(player.stack(1), None, "outside a callback there is nothing");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        told: Vec<(u128, String)>,
        given: Vec<(u128, String, u32)>,
        attached: std::collections::HashMap<(u128, String), String>,
    }

    impl PlayerOps for Recorder {
        fn tell(&mut self, player: u128, line: &str) -> bool {
            self.told.push((player, line.to_owned()));
            true
        }
        fn give(&mut self, player: u128, item: &str, count: u32) -> u32 {
            self.given.push((player, item.to_owned(), count));
            count.saturating_sub(2) // pretend only two fit
        }
        fn attachment_get(&mut self, player: u128, id: &str) -> Option<String> {
            self.attached.get(&(player, id.to_owned())).cloned()
        }
        fn attachment_set(&mut self, player: u128, id: &str, json: &str) -> bool {
            self.attached.insert((player, id.to_owned()), json.to_owned());
            true
        }
        fn count_item(&mut self, _: u128, item: &str) -> u32 {
            if item == "minecraft:stick" { 4 } else { 0 }
        }
        fn take_item(&mut self, _: u128, _: &str, count: u32) -> u32 {
            count.min(1)
        }
        fn empty_slots(&mut self, _: u128) -> u32 {
            3
        }
        fn language(&mut self, _: u128) -> Option<String> {
            None
        }
        fn add_effect(&mut self, _: u128, _: &EffectInstance) -> bool {
            false
        }
        fn remove_effect(&mut self, _: u128, _: &str) -> bool {
            false
        }
        fn effects(&mut self, _: u128) -> Option<Vec<EffectInstance>> {
            None
        }
        fn food(&mut self, _: u128) -> Option<FoodStatus> {
            None
        }
    }

    fn player() -> ServerPlayer {
        ServerPlayer { uuid: 7, name: "Alex".into() }
    }

    #[test]
    fn feeding_uses_vanilla_caps_preserves_exhaustion_and_rejects_invalid_values() {
        let mut ops = MemoryPlayerOps { food: Some(FoodStatus { level: 12, saturation: 2.0, exhaustion: 3.0 }), ..Default::default() };
        with_ops(&mut ops, || {
            assert!(player().add_food(4, 0.5));
            assert_eq!(player().food(), Some(FoodStatus { level: 16, saturation: 6.0, exhaustion: 3.0 }));
            assert!(player().add_food(40, 1.0));
            assert_eq!(player().food(), Some(FoodStatus { level: 20, saturation: 20.0, exhaustion: 3.0 }));
            assert!(player().add_food(-5, 0.5));
            assert_eq!(player().food(), Some(FoodStatus { level: 15, saturation: 15.0, exhaustion: 3.0 }));
            let before = player().food();
            for modifier in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
                assert!(!player().add_food(4, modifier));
                assert_eq!(player().food(), before);
            }
        });
        assert!(!player().add_food(4, 0.5), "a stored handle has no authority");
        let mut unavailable = Recorder::default();
        assert!(!with_ops(&mut unavailable, || player().add_food(4, 0.5)), "unsupported backends stay false");
    }

    #[test]
    fn methods_reach_the_game_inside_a_scope() {
        let mut ops = Recorder::default();
        let (said, left) = with_ops(&mut ops, || (player().send_message("hi"), player().give_item("minecraft:stick", 5)));
        assert!(said);
        assert_eq!(left, 3);
        assert_eq!(ops.told, [(7, "hi".to_owned())]);
        assert_eq!(ops.given, [(7, "minecraft:stick".to_owned(), 5)]);
    }

    #[test]
    fn inventory_questions_are_answered_inside_a_scope_and_by_zero_outside() {
        let mut ops = Recorder::default();
        let inside = with_ops(&mut ops, || (player().count_item("minecraft:stick"), player().take_item("minecraft:stick", 5), player().empty_slots()));
        assert_eq!(inside, (4, 1, 3));
        assert_eq!((player().count_item("minecraft:stick"), player().take_item("minecraft:stick", 5), player().empty_slots()), (0, 0, 0));
    }

    #[test]
    fn a_stored_handle_does_nothing_outside_a_scope() {
        let stored = player();
        assert!(!stored.send_message("late"));
        assert_eq!(stored.give_item("minecraft:stick", 4), 4);
    }

    #[test]
    fn the_scope_ends_even_when_the_callback_panics() {
        let mut ops = Recorder::default();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| with_ops(&mut ops, || panic!("boom"))));
        assert!(caught.is_err());
        assert!(!player().send_message("after"), "the scope must be closed");
    }
}
