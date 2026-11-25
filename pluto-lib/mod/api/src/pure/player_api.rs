//! A handle to a connected player, the counterpart of Fabric's `ServerPlayerEntity` for what mods do most: tell the player something and
//! give them items.
//!
//! The handle is plain data (uuid and name). Its methods reach the game through a *scope* that PlutoLib opens around every callback
//! that receives a player ([`with_ops`]); called outside such a scope they do nothing and report failure, so a stored handle can never touch
//! the game from the wrong place.

use std::cell::Cell;
use std::marker::PhantomData;

use super::base::Event;

/// What the game can do for a player. Implemented once, in `hooks.rs`, over Horizon's `Host`.
pub trait PlayerOps {
    /// Queues a bounded mod payload; unsupported hosts must report failure.
    fn net_send_to_player(&mut self, _player: u128, _channel: &str, _data: &[u8]) -> Result<(), String> {
        Err("mod networking is unavailable".into())
    }
    fn net_send_to_server(&mut self, _channel: &str, _data: &[u8]) -> Result<(), String> {
        Err("mod networking is unavailable".into())
    }
    /// Sends a system chat line to the player; false when the player is not online.
    fn tell(&mut self, player: u128, line: &str) -> bool;
    /// Gives items to the player; returns how many did not fit.
    fn give(&mut self, player: u128, item: &str, count: u32) -> u32;
    /// The JSON stored under the attachment `id` for the player, if any (does not create it).
    fn attachment_get(&mut self, player: u128, id: &str) -> Option<String>;
    /// Stores JSON under the attachment `id` for the player; false if it could not be stored.
    fn attachment_set(&mut self, player: u128, id: &str, json: &str) -> bool;
    /// Player attachment access with an explicit sync policy. Unsupported
    /// synchronized stores refuse rather than silently dropping the policy.
    fn attachment_get_with_policy(&mut self, player: u128, id: &str, sync: bool) -> Option<String> {
        if sync { None } else { self.attachment_get(player, id) }
    }
    fn attachment_set_with_policy(&mut self, player: u128, id: &str, json: &str, sync: bool) -> bool {
        !sync && self.attachment_set(player, id, json)
    }
    /// Current-world entity attachments; missing holders and wrong dimensions
    /// are unavailable. EntityInfo IDs are callback-local, not persistent handles.
    fn entity_attachment_get(&mut self, _entity: u32, _dimension: &str, _id: &str, _sync: bool) -> Option<String> { None }
    fn entity_attachment_set(&mut self, _entity: u32, _dimension: &str, _id: &str, _json: &str, _sync: bool) -> bool { false }
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
    /// Sets the stack carried in a slot named by the carried numbering (see
    /// [`crate::pure::entity_events_v1::carried_slot_number`]), creating it in an empty slot, or clearing it
    /// when `stack` is None. The host reaches a stack this way in the equipment slots (feet 100, legs
    /// 101, chest 102, head 103, offhand -106) and in the main inventory slots (0 to 35): the main
    /// slots go through the host's checked inventory write, which needs the slot to still hold
    /// what the caller read there, so a stack that moved in between is not overwritten. The
    /// equipment write (`LivingEntity.setItemSlot`) creates and clears, which is what an item that
    /// equips a copy of itself on an entity's chest (the piggyback pack) needs. Any other slot, an
    /// unknown player and a client answer false. The memory backend keeps no such restriction and
    /// answers for every slot.
    fn set_stack(&mut self, _player: u128, _slot: i32, _stack: Option<&StackData>) -> bool {
        false
    }
    /// The stack the NAMED player carries in `slot` (the carried numbering, as in
    /// [`Self::set_stack`]), with its custom data; None for an empty slot, an unknown player, or
    /// outside a callback that carries a player scope. Unlike [`ServerPlayer::stack`], this names
    /// the player, so it also answers where no player's state is swapped in (a packet receiver,
    /// a server tick, a native menu click), which is what a keybind packet's server half reads
    /// the sender's worn piece and off hand through.
    fn stack_named(&mut self, _player: u128, _slot: i32) -> Option<StackData> {
        None
    }
    /// Opens a mod menu for the player's item (not a block): the menu type `menu` (a content menu
    /// type, such as a tool's container), with `items` in its slots and `extra` as its menu-extra
    /// data, titled `title`. The window id, or None when the player is not here, the menu type is
    /// unknown, or the call runs inside the player's own turn (call it from a packet receiver, a
    /// server tick or a native menu click, where the player's state is not swapped in). The
    /// menu's slots are the mod's input slots: the player moves items in them by vanilla's rules,
    /// and what they hold when the menu closes comes back through the native-menu close event.
    /// The extra data's first bytes mark an item menu for the client screen: they must begin with
    /// `PLUTO\x01`, the versioned item-menu magic, followed by whatever the mod's screen needs
    /// (the worn slot's carried number, for one).
    fn open_item_menu(&mut self, _player: u128, _menu: &str, _title: &str, _items: Vec<Option<crate::pure::block_entity_api_v1::ItemSlot>>, _extra: Vec<u8>) -> Option<i32> {
        None
    }
    /// The player's selected hotbar slot (0 to 8); None if unknown.
    fn selected_slot(&mut self, _player: u128) -> Option<i32> {
        None
    }
    /// Whether `item` is on the player's item cooldown (the item's own `use_cooldown`
    /// component, else the item's default); false for an item with no cooldown, or when the
    /// game does not report cooldowns. The default answers false.
    fn is_item_on_cooldown(&mut self, _player: u128, _item: &str) -> bool {
        false
    }
    /// Starts the player's cooldown for `item` for `ticks` ticks (the item's own
    /// `use_cooldown` component, else the item's default, names the group); nothing happens
    /// for an item with no cooldown, or when the game does not report cooldowns.
    fn start_item_cooldown(&mut self, _player: u128, _item: &str, _ticks: i32) {}
    /// Ends the player's item use in progress as `LivingEntity.releaseUsingItem` does: the use's own
    /// release runs first (a callback's Stop may skip that part, never the end itself), then the use is
    /// over. The game applies the end at its next tick seam, so a use that already ended is untouched
    /// and an end ordered during one use never lands on a later use of a mod's. True when the end was
    /// taken; false when this backend cannot end uses (the default) or the player is not known.
    fn release_use(&mut self, _player: u128) -> bool {
        false
    }
    /// Ends the player's item use in progress as `LivingEntity.stopUsingItem` does: the use is simply
    /// over, with no release run — what an item breaking in the hand does to its use. Applied at the
    /// game's next tick seam like [`PlayerOps::release_use`]. True when the end was taken; false by default.
    fn stop_use(&mut self, _player: u128) -> bool {
        false
    }
    /// Swings the player's arm in `hand` (`LivingEntity.swing`): the arm moves on their own screen and, for a
    /// guest, the players around them are told. False when the swing could not be queued (no player, no host).
    /// The swing plays when the game next reaches the player — inside their break, or at their next tick.
    fn swing_hand(&mut self, _player: u128, _hand: super::events_interaction_v0::Hand) -> bool {
        false
    }
    /// Broadcasts that the stack in `slot` broke (`LivingEntity.onEquippedItemBroken`): the break statistic,
    /// the break sound and crumbs where the player stands, and the item-break event for the players around.
    /// False when the broadcast could not be queued. The game's own wear already broadcasts the breaks it
    /// applies to the stacks it breaks itself (a vanilla item worn by mining or hurt armor), so a break the
    /// game applied is never broadcast this way — it would sound the break twice. Every break of a stack
    /// the game never breaks is the port's to broadcast: a Tinkers tool's mirror keeps its damage one short
    /// of the game's break point, so the consumer whose wear broke the tool broadcasts it — the wear the
    /// game's wear event asked for (where the port's replacement is the stack the game keeps) included,
    /// not only a modifier's own extra wear.
    fn broadcast_item_break(
        &mut self,
        _player: u128,
        _slot: super::entity_events_v1::StackSlot,
        _stack: &StackData,
    ) -> bool {
        false
    }
    /// The value of the host attribute `attribute` for `player` with every modifier on it
    /// (armour, effects, `/attribute`), as `LivingEntity.getAttributeValue` answers it:
    /// `minecraft:block_interaction_range` is the range a use that aims traces over,
    /// `minecraft:max_health` the health cap. None when the game cannot answer — the
    /// attribute is not one players have, or the player is not reached or between bodies
    /// (just respawned, not yet ticked) — or when this backend cannot read attributes
    /// (the default). The caller falls back to the attribute's base value then, as the
    /// game's own range reads do.
    fn attribute_value(&mut self, _player: u128, _attribute: &str) -> Option<f64> {
        None
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
    /// The stack's `minecraft:custom_model_data` component, which item models select on; None for none.
    pub custom_model_data: Option<CustomModelData>,
    /// The stack's own `minecraft:enchantments` component as enchantment id to level (such as `minecraft:fortune` to 3); empty when
    /// the stack has none of its own or the game did not report them. Enchantments the item itself grants through its data (a tool's
    /// modifier-granted levels kept in `custom_data`) are not listed here; this is the component the game itself applies.
    pub enchantments: std::collections::BTreeMap<String, i32>,
}

impl StackData {
    /// The level of enchantment `id` on this stack (0 when it has none), from its own `minecraft:enchantments` component.
    pub fn enchantment_level(&self, id: &str) -> i32 {
        self.enchantments.get(id).copied().unwrap_or(0)
    }
}

/// The values an item model can select on (Minecraft: the `minecraft:custom_model_data` component, `CustomModelData`). An item definition
/// reads them by index: a `select` on `custom_model_data` compares `strings[index]`, a `range_dispatch` uses `floats[index]`, a `condition`
/// uses `flags[index]` and a tint source uses `colors[index]`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CustomModelData {
    /// Strings an item definition's `select` matches against.
    pub strings: Vec<String>,
    /// Numbers a `range_dispatch` reads.
    pub floats: Vec<f32>,
    /// Booleans a `condition` reads.
    pub flags: Vec<bool>,
    /// RGB colours (0xRRGGBB) a tint source reads.
    pub colors: Vec<i32>,
}

impl CustomModelData {
    /// Data with only `strings`.
    pub fn with_strings<S: Into<String>>(strings: impl IntoIterator<Item = S>) -> Self {
        Self { strings: strings.into_iter().map(Into::into).collect(), ..Self::default() }
    }

    /// True when all four lists are empty; such data is not stored on a stack.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty() && self.floats.is_empty() && self.flags.is_empty() && self.colors.is_empty()
    }

    /// The saved form of the component: a compound with `strings`, `floats`, `flags` (bytes 0 or 1) and `colors` lists, each left out when
    /// empty, as the game writes it.
    pub fn to_nbt(&self) -> crate::pure::base::NbtCompound {
        use crate::pure::base::Nbt;
        let mut out = crate::pure::base::NbtCompound::new();
        if !self.strings.is_empty() { out.insert("strings".into(), Nbt::List(self.strings.iter().cloned().map(Nbt::String).collect())); }
        if !self.floats.is_empty() { out.insert("floats".into(), Nbt::List(self.floats.iter().copied().map(Nbt::Float).collect())); }
        if !self.flags.is_empty() { out.insert("flags".into(), Nbt::List(self.flags.iter().map(|flag| Nbt::Byte(i8::from(*flag))).collect())); }
        if !self.colors.is_empty() { out.insert("colors".into(), Nbt::List(self.colors.iter().copied().map(Nbt::Int).collect())); }
        out
    }

    /// Reads the saved form back. Lists may also be int or byte arrays, a flag is any non-zero number, and a colour may be three floats
    /// from 0 to 1 (red, green, blue); entries of another type are skipped.
    pub fn from_nbt(nbt: &crate::pure::base::NbtCompound) -> Self {
        use crate::pure::base::Nbt;
        let entries = |key: &str| -> Vec<Nbt> {
            match nbt.get(key) {
                Some(Nbt::List(list)) => list.clone(),
                Some(Nbt::IntArray(values)) => values.iter().map(|v| Nbt::Int(*v)).collect(),
                Some(Nbt::ByteArray(values)) => values.iter().map(|v| Nbt::Byte(*v as i8)).collect(),
                _ => Vec::new(),
            }
        };
        let float = |value: &Nbt| match value {
            Nbt::Float(v) => Some(f64::from(*v)),
            Nbt::Double(v) => Some(*v),
            other => other.as_i64().map(|v| v as f64),
        };
        let channel = |value: Option<&Nbt>| value.and_then(float).map(|v| (((v as f32) * 255.0).floor() as i32) & 255);
        Self {
            strings: entries("strings").iter().filter_map(|v| match v { Nbt::String(text) => Some(text.clone()), _ => None }).collect(),
            floats: entries("floats").iter().filter_map(float).map(|v| v as f32).collect(),
            flags: entries("flags").iter().filter_map(Nbt::as_i64).map(|v| v != 0).collect(),
            colors: entries("colors").iter().filter_map(|v| match v {
                Nbt::List(rgb) if rgb.len() == 3 => Some((channel(rgb.first())? << 16) | (channel(rgb.get(1))? << 8) | channel(rgb.get(2))?),
                other => other.as_i64().map(|v| v as i32),
            }).collect(),
        }
    }
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
    /// Whether the effect's icon shows in the GUI (vanilla's `MobEffectInstance.showIcon`,
    /// `/effect`'s default true). The read's flag is published by PlutoLib's effect-icon
    /// reach at the head of every HUD draw; giving an effect records the flag through
    /// PlutoLib's effect-apply reach, and the game puts it on the instance as the host's
    /// add merges it, so an effect can be given with its icon hidden.
    pub show_icon: bool,
}

impl EffectInstance {
    /// An effect of level I lasting `duration` ticks (20 are one second; -1 never ends), with particles and its icon showing.
    pub fn new(id: &str, duration: i32) -> Self {
        Self { id: id.to_owned(), amplifier: 0, duration, ambient: false, particles: true, show_icon: true }
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

    /// Sets the stack carried in a slot named by the carried numbering, creating it in an empty slot, or
    /// clearing it with None. The host answers for the equipment slots (feet 100, legs 101, chest 102,
    /// head 103, offhand -106) and for the main inventory slots (0 to 35); see [`PlayerOps::set_stack`].
    /// False outside a callback or for another slot.
    pub fn set_stack(&self, slot: i32, stack: Option<&StackData>) -> bool {
        with_current(false, |ops| ops.set_stack(self.uuid, slot, stack))
    }

    /// The stack the player carries in `slot` (see [`PlayerOps::stack_named`]); None for an empty slot
    /// or outside a callback. Unlike [`Self::stack`], this names the player, so it answers in a packet
    /// receiver too.
    pub fn stack_named(&self, slot: i32) -> Option<StackData> {
        with_current(None, |ops| ops.stack_named(self.uuid, slot))
    }

    /// Opens a mod menu for the player's item (see [`PlayerOps::open_item_menu`]); the window id, or
    /// None outside a callback, for an unknown player or menu type, or inside the player's own turn.
    pub fn open_item_menu(&self, menu: &str, title: &str, items: Vec<Option<crate::pure::block_entity_api_v1::ItemSlot>>, extra: Vec<u8>) -> Option<i32> {
        with_current(None, |ops| ops.open_item_menu(self.uuid, menu, title, items, extra))
    }

    /// Sets one equipment slot's stack by its [`StackSlot`](crate::pure::entity_events_v1::StackSlot), creating
    /// it in an empty slot or clearing it with None: the carried-numbering form of [`ServerPlayer::set_stack`].
    /// The main hand has no equipment write and answers false (use [`ServerPlayer::update_stack`] there).
    pub fn set_equipment(&self, slot: crate::pure::entity_events_v1::StackSlot, stack: Option<&StackData>) -> bool {
        if matches!(slot, crate::pure::entity_events_v1::StackSlot::MainHand) {
            return false;
        }
        // The main hand returned above, so the selected slot the numbering reads only for it is moot here.
        self.set_stack(crate::pure::entity_events_v1::carried_slot_number(slot, 0), stack)
    }

    /// The hotbar slot the player has selected (0 to 8); None outside a callback that carries this player.
    pub fn selected_slot(&self) -> Option<i32> {
        with_current(None, |ops| ops.selected_slot(self.uuid))
    }

    /// Whether `item` is on the player's item cooldown (the item's own `use_cooldown`
    /// component, else the item's default). False for an item with no cooldown, or outside a
    /// callback that carries this player.
    pub fn is_item_on_cooldown(&self, item: &str) -> bool {
        with_current(false, |ops| ops.is_item_on_cooldown(self.uuid, item))
    }

    /// Starts the player's cooldown for `item` for `ticks` ticks (the item's own
    /// `use_cooldown` component, else the item's default, names the group). Nothing happens
    /// outside a callback that carries this player, or for an item with no cooldown.
    pub fn start_item_cooldown(&self, item: &str, ticks: i32) {
        with_current((), |ops| ops.start_item_cooldown(self.uuid, item, ticks))
    }

    /// Ends this player's item use in progress as `LivingEntity.releaseUsingItem` does: the use's own
    /// release runs first (a callback's Stop may skip that part, never the end itself), then the use is
    /// over — what letting go of the use button does, asked for from inside a callback such as a use
    /// tick. The game applies the end at its next tick seam; a use that already ended is untouched.
    /// False outside a callback that carries the player, or when the game cannot end uses.
    pub fn release_use(&self) -> bool {
        with_current(false, |ops| ops.release_use(self.uuid))
    }

    /// Ends this player's item use in progress as `LivingEntity.stopUsingItem` does: the use is simply
    /// over, with no release run, as when the item in use breaks in the hand. Applied at the game's
    /// next tick seam like [`ServerPlayer::release_use`]. False outside a callback that carries the player.
    pub fn stop_use(&self) -> bool {
        with_current(false, |ops| ops.stop_use(self.uuid))
    }

    /// Swings the player's arm in `hand` (`LivingEntity.swing`): the arm moves on their own screen and, for a
    /// guest, the players around them are told. False outside a callback that carries this player, or when the
    /// game could not queue the swing. The swing plays when the game next reaches the player — inside their
    /// break, or at their next tick.
    pub fn swing_hand(&self, hand: super::events_interaction_v0::Hand) -> bool {
        with_current(false, |ops| ops.swing_hand(self.uuid, hand))
    }

    /// Broadcasts that the stack in `slot` broke (`LivingEntity.onEquippedItemBroken`): the break statistic,
    /// the break sound and crumbs where the player stands, and the item-break event for the players around.
    /// `stack` is the stack as it broke. False outside a callback that carries this player, or when the game
    /// could not queue the broadcast. Only a break the port itself applied is broadcast this way — the game's
    /// own wear already broadcasts the breaks it applies to the stacks it breaks itself; a Tinkers tool is
    /// never one of those (its mirror stops one short of the game's break point), so every break of one is
    /// broadcast by the consumer whose wear broke it.
    pub fn broadcast_item_break(
        &self,
        slot: super::entity_events_v1::StackSlot,
        stack: &StackData,
    ) -> bool {
        with_current(false, |ops| ops.broadcast_item_break(self.uuid, slot, stack))
    }
    /// The value of the host attribute `attribute` for this player with every modifier on it,
    /// as `LivingEntity.getAttributeValue` answers it (see [`PlayerOps::attribute_value`]).
    /// None outside a callback that carries the player, or when the game cannot answer.
    pub fn attribute_value(&self, attribute: &str) -> Option<f64> {
        with_current(None, |ops| ops.attribute_value(self.uuid, attribute))
    }

    /// The stack in the player's selected hotbar slot (their main hand); None for an empty slot or outside a callback.
    pub fn main_hand_stack(&self) -> Option<StackData> {
        self.selected_slot().and_then(|slot| self.stack(slot))
    }

    /// Empty slots in the 36-slot main inventory (Fabric: counting `PlayerInventory.getEmptySlot`). 0 outside a callback.
    pub fn empty_slots(&self) -> u32 {
        with_current(0, |ops| ops.empty_slots(self.uuid))
    }
}

/// A key of the player's own keyboard or mouse changed state (NeoForge: `InputEvent.Key`; Bukkit has no equivalent). It is heard on
/// the client, for the local player only: `key` is the game's own name for the key (the names key bindings use, such as
/// `key.keyboard.left.shift` or `key.mouse.left`), repeats are not reported, and `pressed` says which way it moved. A callback that
/// needs the server to act (swapping what worn armor holds, say) sends its own packet with the networking API, as a client key press
/// can only ever be a hint to the server; the worn stacks themselves are read through the usual player and entity events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyPress {
    /// The local player; None when the game names none.
    pub player: Option<ServerPlayer>,
    /// The game's name for the key.
    pub key: String,
    /// True when the key went down, false when it came up.
    pub pressed: bool,
}

static KEY_PRESS: Event<KeyPress, ()> = Event::void();

/// Key presses of the local player (see [`KeyPress`]).
pub struct PlayerKeyEvents;

impl PlayerKeyEvents {
    /// The event: every callback runs, in registration order.
    pub const EVENT: &'static Event<KeyPress, ()> = &KEY_PRESS;
}

/// Whether any callback hears key presses (the host adapter skips its work when none does).
pub(crate) fn key_heard() -> bool {
    !KEY_PRESS.is_empty()
}

/// Reports a key press to the callbacks.
pub(crate) fn key_press(press: &KeyPress) {
    KEY_PRESS.invoke(press);
}

/// A player's item broke (NeoForge's `PlayerDestroyItemEvent` carries the original stack and a hand, which
/// armour has not; Bukkit's `PlayerItemBreakEvent`). Heard on a server when the game's own wear breaks a
/// player's stack — the host names the break only for a guest (its `player_equipment_broke` fires the hook
/// from a guest's turn, never for the host player, whose break still sounds and shows its crumbs locally).
/// `item` is the item that broke, `slot` the equipment slot it broke in. A break the port itself applied
/// and broadcast (see [`PlayerOps::broadcast_item_break`]) is heard the same way, once the game plays it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemBreak {
    /// The player whose item broke.
    pub player: ServerPlayer,
    /// The item that broke, such as `tconstruct:pickaxe`.
    pub item: String,
    /// The equipment slot the stack broke in.
    pub slot: super::entity_events_v1::StackSlot,
}

static ITEM_BREAK: Event<ItemBreak, ()> = Event::void();

/// A player's item breaking (see [`ItemBreak`]).
pub struct PlayerItemBreakEvents;

impl PlayerItemBreakEvents {
    /// The event: every callback runs, in registration order.
    pub const EVENT: &'static Event<ItemBreak, ()> = &ITEM_BREAK;
}

/// Whether any callback hears item breaks (the host adapter skips its work when none does).
pub(crate) fn item_break_heard() -> bool {
    !ITEM_BREAK.is_empty()
}

/// Reports a player's broken item to the callbacks.
pub(crate) fn item_break(broken: &ItemBreak) {
    ITEM_BREAK.invoke(broken);
}

/// One inventory slot of a player ticked (NeoForge: `PlayerTickEvent` has no slot-level counterpart; the closest is
/// Bukkit's inventory events — the shape follows the game's own per-slot inventory tick, the hook an item's
/// `inventoryTick` answers in the original). Heard on a server, once per tick for each carried slot the game ticks:
/// the stack the slot holds, whether the slot is the selected hotbar slot, and the player carrying it. Callbacks that
/// need to change the world act through the scopes the host opens around the event (the entity and world reaches).
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryTick {
    /// The player carrying the slot; None when the game names none.
    pub player: Option<ServerPlayer>,
    /// The slot's own number in the player's inventory, in the game's numbering.
    pub slot: i32,
    /// Whether the slot is the selected hotbar slot.
    pub selected: bool,
    /// The stack in the slot, with its damage, max damage, custom data and enchantments; None when the game names
    /// none (an empty slot, or a stack it did not read).
    pub stack: Option<StackData>,
}

static INVENTORY_TICK: Event<InventoryTick, ()> = Event::void();

/// Inventory slot ticks of players (see [`InventoryTick`]).
pub struct PlayerInventoryEvents;

impl PlayerInventoryEvents {
    /// The event: every callback runs, in registration order.
    pub const TICK: &'static Event<InventoryTick, ()> = &INVENTORY_TICK;
}

/// Whether any callback hears inventory ticks (the host adapter skips its work when none does).
pub(crate) fn inventory_tick_heard() -> bool {
    !INVENTORY_TICK.is_empty()
}

/// Reports an inventory slot tick to the callbacks.
pub(crate) fn inventory_tick(tick: &InventoryTick) {
    INVENTORY_TICK.invoke(tick);
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
    /// JSON per (dimension, callback-local entity ID, attachment ID). Tests
    /// simulating entity-ID reuse must clear the departed holder explicitly.
    pub entity_attachments: std::collections::HashMap<(String, u32, String), String>,
    /// One immutable sync policy per attachment ID, shared across holder kinds.
    pub attachment_policies: std::collections::HashMap<String, bool>,
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
    /// The stacks set through the carried numbering (equipment slots and any other slot `set_stack` was
    /// given), by `(player, carried slot)`. Kept apart from `stacks`: the main inventory vector is indexed by
    /// its own numbering, and the memory backend keeps no equipment restriction of its own.
    pub carried: std::collections::BTreeMap<(u128, i32), StackData>,
    /// The selected hotbar slot of every player (0 to 8).
    pub selected_slot: i32,
    /// The item cooldowns started for each player, by `(player, item id)` to ticks left.
    /// The memory backend does not tick them down; a started cooldown stays on until a test
    /// clears it.
    pub cooldowns: std::collections::HashMap<(u128, String), i32>,
    /// Every player whose use a `release_use` ended, in the order they were ended.
    pub released_uses: Vec<u128>,
    /// Every player whose use a `stop_use` ended, in the order they were ended.
    pub stopped_uses: Vec<u128>,
    /// Every break broadcast, as (player, slot, item id of the stack as it broke), in order.
    pub breaks: Vec<(u128, super::entity_events_v1::StackSlot, String)>,
    /// The item menus opened, in order, as (player, menu type, title, items, extra data).
    pub opened_item_menus: Vec<OpenedItemMenu>,
    /// The host attribute values a test set for a player, by `(player, attribute)`: what
    /// `attribute_value` answers. A player with nothing set answers None, as a player the
    /// game cannot reach does.
    pub attributes: std::collections::BTreeMap<(u128, String), f64>,
}

/// An item menu opened through the memory backend, as [`MemoryPlayerOps::opened_item_menus`] records it.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenedItemMenu {
    /// The player it opened for.
    pub player: u128,
    /// The menu type, such as `tconstruct:tool_container_3`.
    pub menu: String,
    /// The title shown.
    pub title: String,
    /// The items in its slots, in order.
    pub items: Vec<Option<crate::pure::block_entity_api_v1::ItemSlot>>,
    /// Its menu-extra data.
    pub extra: Vec<u8>,
}

impl MemoryPlayerOps {
    fn attachment_policy(&mut self, id: &str, sync: bool) -> bool {
        *self.attachment_policies.entry(id.to_owned()).or_insert(sync) == sync
    }
}

impl PlayerOps for MemoryPlayerOps {
    fn tell(&mut self, player: u128, line: &str) -> bool {
        self.told.push((player, line.to_owned()));
        true
    }
    fn broadcast_item_break(&mut self, player: u128, slot: super::entity_events_v1::StackSlot, stack: &StackData) -> bool {
        self.breaks.push((player, slot, stack.item.clone()));
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
    fn attachment_get_with_policy(&mut self, player: u128, id: &str, sync: bool) -> Option<String> {
        if !self.attachment_policy(id, sync) { return None; }
        self.attachment_get(player, id)
    }
    fn attachment_set_with_policy(&mut self, player: u128, id: &str, json: &str, sync: bool) -> bool {
        self.attachment_policy(id, sync) && self.attachment_set(player, id, json)
    }
    fn entity_attachment_get(&mut self, entity: u32, dimension: &str, id: &str, sync: bool) -> Option<String> {
        if !self.attachment_policy(id, sync) { return None; }
        self.entity_attachments.get(&(dimension.to_owned(), entity, id.to_owned())).cloned()
    }
    fn entity_attachment_set(&mut self, entity: u32, dimension: &str, id: &str, json: &str, sync: bool) -> bool {
        if !self.attachment_policy(id, sync) { return false; }
        self.entity_attachments.insert((dimension.to_owned(), entity, id.to_owned()), json.to_owned()); true
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
    fn set_stack(&mut self, player: u128, slot: i32, stack: Option<&StackData>) -> bool {
        // The memory backend models the carried slots apart from the main inventory vector and, unlike the
        // host, refuses nothing: a test that needs the host's equipment-only rule checks the answer against
        // the slot numbers the trait documents.
        match stack {
            Some(stack) if stack.count > 0 => {
                self.carried.insert((player, slot), stack.clone());
            }
            _ => {
                self.carried.remove(&(player, slot));
            }
        }
        true
    }
    fn stack_named(&mut self, player: u128, slot: i32) -> Option<StackData> {
        // What `set_stack` wrote answers first (the carried map is where its writes land); the main
        // inventory vector answers the rest, as `stack` reads it.
        if let Some(stack) = self.carried.get(&(player, slot)) {
            return Some(stack.clone()).filter(|stack| stack.count > 0);
        }
        self.stack(player, slot)
    }
    fn open_item_menu(&mut self, player: u128, menu: &str, title: &str, items: Vec<Option<crate::pure::block_entity_api_v1::ItemSlot>>, extra: Vec<u8>) -> Option<i32> {
        self.opened_item_menus.push(OpenedItemMenu { player, menu: menu.to_owned(), title: title.to_owned(), items, extra });
        Some(self.opened_item_menus.len() as i32)
    }
    fn selected_slot(&mut self, _player: u128) -> Option<i32> {
        Some(self.selected_slot)
    }
    fn is_item_on_cooldown(&mut self, player: u128, item: &str) -> bool {
        self.cooldowns.get(&(player, item.to_owned())).is_some_and(|ticks| *ticks > 0)
    }
    fn start_item_cooldown(&mut self, player: u128, item: &str, ticks: i32) {
        self.cooldowns.insert((player, item.to_owned()), ticks);
    }

    fn release_use(&mut self, player: u128) -> bool {
        self.released_uses.push(player);
        true
    }
    fn stop_use(&mut self, player: u128) -> bool {
        self.stopped_uses.push(player);
        true
    }
    fn attribute_value(&mut self, player: u128, attribute: &str) -> Option<f64> {
        self.attributes.get(&(player, attribute.to_owned())).copied()
    }
}

#[cfg(test)]
mod stack_tests {
    use super::*;
    use crate::pure::base::Nbt;

    fn pickaxe(damage: u32) -> StackData {
        StackData { item: "tconstruct:pickaxe".into(), count: 1, damage, max_damage: Some(60), custom_data: [("stats".to_owned(), Nbt::Int(60))].into(), custom_model_data: None, enchantments: Default::default() }
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
    fn an_item_cooldown_starts_and_reads_back_until_cleared() {
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 1, name: "Alex".into() };
        assert!(!player.is_item_on_cooldown("minecraft:ender_pearl"));
        with_ops(&mut ops, || {
            assert!(!player.is_item_on_cooldown("minecraft:ender_pearl"));
            player.start_item_cooldown("minecraft:ender_pearl", 10);
            assert!(player.is_item_on_cooldown("minecraft:ender_pearl"));
            assert!(!player.is_item_on_cooldown("minecraft:chorus_fruit"), "another item is not cooling down");
        });
        assert_eq!(ops.cooldowns[&(1, "minecraft:ender_pearl".to_owned())], 10);
        // Outside a callback nothing starts, and a cleared entry reads as off cooldown.
        player.start_item_cooldown("minecraft:chorus_fruit", 10);
        assert!(!ops.cooldowns.contains_key(&(1, "minecraft:chorus_fruit".to_owned())));
        ops.cooldowns.remove(&(1, "minecraft:ender_pearl".to_owned()));
        assert!(!with_ops(&mut ops, || player.is_item_on_cooldown("minecraft:ender_pearl")));
    }

    #[test]
    fn custom_model_data_saves_as_the_games_compound_and_reads_every_form_the_game_writes() {
        let data = CustomModelData { strings: vec!["wood".into(), String::new()], floats: vec![0.5], flags: vec![true, false], colors: vec![0x00a0_a0a0] };
        let saved = data.to_nbt();
        assert_eq!(saved["strings"], Nbt::List(vec![Nbt::String("wood".into()), Nbt::String(String::new())]));
        assert_eq!(saved["flags"], Nbt::List(vec![Nbt::Byte(1), Nbt::Byte(0)]));
        assert_eq!(CustomModelData::from_nbt(&saved), data);
        // lists left empty are not written, and nothing at all is empty data
        assert_eq!(CustomModelData::with_strings(["rose_gold"]).to_nbt().keys().collect::<Vec<_>>(), ["strings"]);
        assert!(CustomModelData::default().is_empty() && CustomModelData::default().to_nbt().is_empty());
        // the game saves int and byte lists as arrays, and a colour may be three floats
        let game: crate::pure::base::NbtCompound = [
            ("flags".to_owned(), Nbt::ByteArray(vec![1, 0])), ("colors".to_owned(), Nbt::IntArray(vec![7])),
            ("floats".to_owned(), Nbt::List(vec![Nbt::Double(2.0), Nbt::Int(3)])),
        ].into();
        let read = CustomModelData::from_nbt(&game);
        assert_eq!((read.flags, read.colors, read.floats), (vec![true, false], vec![7], vec![2.0, 3.0]));
        let triple: crate::pure::base::NbtCompound = [("colors".to_owned(), Nbt::List(vec![Nbt::List(vec![Nbt::Float(0.0), Nbt::Float(1.0), Nbt::Float(0.0)])]))].into();
        assert_eq!(CustomModelData::from_nbt(&triple).colors, vec![0x0000_ff00]);
    }

    #[test]
    fn a_carried_stack_can_be_created_in_an_empty_equipment_slot_and_cleared() {
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 7, name: "Alex".into() };
        let pack = StackData { item: "tconstruct:piggy_backpack".into(), count: 1, ..StackData::default() };
        with_ops(&mut ops, || {
            // the chest slot (carried number 102) starts empty; set_stack creates the stack there
            assert!(player.set_equipment(crate::pure::entity_events_v1::StackSlot::Chest, Some(&pack)));
            // the main hand has no equipment write
            assert!(!player.set_equipment(crate::pure::entity_events_v1::StackSlot::MainHand, Some(&pack)));
        });
        assert_eq!(ops.carried.get(&(7, 102)), Some(&pack));
        // growing the carried stack replaces it, and None clears it
        let grown = StackData { count: 3, ..pack.clone() };
        with_ops(&mut ops, || assert!(player.set_stack(102, Some(&grown))));
        assert_eq!(ops.carried.get(&(7, 102)), Some(&grown));
        with_ops(&mut ops, || assert!(player.set_stack(102, None)));
        assert!(!ops.carried.contains_key(&(7, 102)));
    }

    #[test]
    fn a_stack_keeps_its_custom_model_data_through_give_and_read() {
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 1, name: "Alex".into() };
        let stack = StackData { custom_model_data: Some(CustomModelData::with_strings(["wood", "stone"])), ..pickaxe(0) };
        with_ops(&mut ops, || {
            assert_eq!(player.give_stack(stack.clone()), None);
            assert_eq!(player.stack(0).unwrap().custom_model_data, stack.custom_model_data);
        });
    }

    #[test]
    fn a_named_player_stack_reads_what_set_stack_wrote_in_any_carried_slot() {
        let mut ops = MemoryPlayerOps::default();
        ops.stacks.insert(1, vec![None, Some(pickaxe(0))]);
        let player = ServerPlayer { uuid: 1, name: "Alex".into() };
        with_ops(&mut ops, || {
            // the main inventory vector answers through the named read too
            assert_eq!(player.stack_named(1).unwrap().damage, 0);
            assert_eq!(player.stack_named(0), None, "an empty main slot");
            assert_eq!(player.stack_named(103), None, "an empty head slot");
            // a whole-stack write lands where the named read looks, main and equipment alike
            let torch = StackData { item: "minecraft:torch".into(), count: 5, ..StackData::default() };
            assert!(player.set_stack(5, Some(&torch)));
            assert_eq!(player.stack_named(5), Some(torch));
            let helmet = StackData { item: "tconstruct:helmet".into(), count: 1, ..StackData::default() };
            assert!(player.set_stack(103, Some(&helmet)));
            assert_eq!(player.stack_named(103), Some(helmet));
            // clearing takes the named read with it
            assert!(player.set_stack(5, None));
            assert_eq!(player.stack_named(5), None);
        });
        assert_eq!(player.stack_named(1), None, "outside a callback there is nothing");
    }

    #[test]
    fn an_item_menu_open_is_recorded_with_its_menu_and_items() {
        use crate::pure::base::Identifier;
        use crate::pure::block_entity_api_v1::ItemSlot;
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 3, name: "Alex".into() };
        let items = vec![Some(ItemSlot::new(Identifier::parse("minecraft:torch").unwrap(), 5)), None];
        let window = with_ops(&mut ops, || player.open_item_menu("tconstruct:tool_container_1", "Pickaxe", items.clone(), vec![b'P', b'L', b'U', b'T', b'O', 0x01, 103]));
        assert_eq!(window, Some(1));
        assert_eq!(ops.opened_item_menus.len(), 1);
        let opened = &ops.opened_item_menus[0];
        assert_eq!((opened.player, opened.menu.as_str(), opened.title.as_str()), (3, "tconstruct:tool_container_1", "Pickaxe"));
        assert_eq!(opened.items, items);
        assert_eq!(opened.extra, vec![b'P', b'L', b'U', b'T', b'O', 0x01, 103]);
        assert_eq!(with_ops(&mut ops, || player.open_item_menu("tconstruct:tool_container_2", "Pickaxe", vec![], vec![])), Some(2), "a second open gets the next window");
        assert_eq!(player.open_item_menu("tconstruct:tool_container_1", "Pickaxe", vec![], vec![]), None, "outside a callback nothing opens");
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
    fn an_effect_carries_whether_its_icon_shows() {
        assert!(EffectInstance::new("minecraft:speed", 100).show_icon, "a fresh effect shows its icon, as vanilla defaults it");
        let mut ops = MemoryPlayerOps::default();
        with_ops(&mut ops, || {
            assert!(player().add_effect(&EffectInstance { show_icon: false, ..EffectInstance::new("minecraft:darkness", 100) }));
            assert_eq!(player().effects().iter().map(|effect| effect.show_icon).collect::<Vec<_>>(), [false], "the read keeps a hidden effect hidden");
        });
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

    #[test]
    fn inventory_ticks_reach_their_callbacks_with_the_slot_and_stack() {
        use std::sync::Mutex;
        static SEEN: Mutex<Vec<(i32, bool, Option<String>)>> = Mutex::new(Vec::new());
        PlayerInventoryEvents::TICK.register(|tick| {
            SEEN.lock().unwrap().push((tick.slot, tick.selected, tick.stack.as_ref().map(|stack| stack.item.clone())));
        });
        assert!(inventory_tick_heard());
        inventory_tick(&InventoryTick { player: None, slot: 3, selected: true, stack: None });
        inventory_tick(&InventoryTick { player: None, slot: 100, selected: false, stack: Some(StackData { item: "minecraft:torch".into(), count: 12, damage: 0, max_damage: None, custom_data: crate::pure::base::NbtCompound::new(), custom_model_data: None, enchantments: std::collections::BTreeMap::new() }) });
        let seen = SEEN.lock().unwrap().clone();
        assert_eq!(seen, vec![(3, true, None), (100, false, Some("minecraft:torch".into()))]);
    }

    #[test]
    fn item_breaks_reach_their_callbacks_with_player_item_and_slot() {
        use std::sync::Mutex;
        static SEEN: Mutex<Vec<(u128, String, String)>> = Mutex::new(Vec::new());
        PlayerItemBreakEvents::EVENT.register(|broken| {
            SEEN.lock().unwrap().push((
                broken.player.uuid,
                broken.item.clone(),
                format!("{:?}", broken.slot),
            ));
        });
        assert!(item_break_heard());
        item_break(&ItemBreak {
            player: player(),
            item: "tconstruct:pickaxe".into(),
            slot: crate::pure::entity_events_v1::StackSlot::MainHand,
        });
        assert_eq!(
            SEEN.lock().unwrap().clone(),
            vec![(7, "tconstruct:pickaxe".to_string(), "MainHand".to_string())]
        );
    }

    #[test]
    fn a_swing_and_a_break_broadcast_answer_false_without_a_host() {
        let mut ops = Recorder::default();
        let held = player();
        assert!(!with_ops(&mut ops, || held.swing_hand(crate::pure::events_interaction_v0::Hand::Off)));
        let stack = StackData {
            item: "tconstruct:pickaxe".into(),
            count: 1,
            damage: 60,
            max_damage: Some(60),
            custom_data: crate::pure::base::NbtCompound::new(),
            custom_model_data: None,
            enchantments: std::collections::BTreeMap::new(),
        };
        assert!(!with_ops(&mut ops, || held
            .broadcast_item_break(crate::pure::entity_events_v1::StackSlot::MainHand, &stack)));
        assert!(!held.swing_hand(crate::pure::events_interaction_v0::Hand::Main), "outside a callback nothing swings");
    }

    #[test]
    fn use_ends_are_recorded_by_the_memory_game_and_unheard_outside_a_scope() {
        let player = player();
        assert!(!player.release_use(), "outside a scope nothing is ended");
        assert!(!player.stop_use(), "outside a scope nothing is ended");
        let mut game = MemoryPlayerOps::default();
        with_ops(&mut game, || {
            assert!(player.release_use());
            assert!(player.stop_use());
            assert!(player.release_use());
        });
        assert_eq!(game.released_uses, [7u128, 7u128], "every release is recorded, in order");
        assert_eq!(game.stopped_uses, [7u128]);
    }

    #[test]
    fn attribute_values_are_read_from_the_memory_game_and_unheard_outside_a_scope() {
        let player = player();
        assert_eq!(player.attribute_value("minecraft:block_interaction_range"), None, "outside a scope nothing is read");
        let mut game = MemoryPlayerOps::default();
        game.attributes.insert((7, "minecraft:block_interaction_range".to_owned()), 6.0);
        with_ops(&mut game, || {
            assert_eq!(player.attribute_value("minecraft:block_interaction_range"), Some(6.0));
            assert_eq!(player.attribute_value("minecraft:entity_interaction_range"), None, "an attribute nothing set stays unread");
        });
    }
}
