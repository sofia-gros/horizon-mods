//! The Sunrise bridge: it turns Sunrise's mod_api hook points into PlutoLib's Fabric-like events.
//!
//! The direction is the inverse of the old woven bridge. Nothing in this crate names a game crate:
//! the hook points arrive as `mod_api` events, and the woven support modules come and register
//! themselves into the pure registries this crate owns (see SUNRISE.md, L2). What a hook body
//! cannot reach through `mod_api` stays unwired and is listed in the L2 disposition table rather
//! than faked.

use mod_api::{
    AttachmentCodec, AttachmentDescriptor, AttachmentError, AttachmentKey, Compound, Holder,
    PayloadBytes, PayloadId, Registrar, RegistrationContext, SoundSource, Tag, Text,
};

use crate::pure::entity_events_v1::{self, LivingDamage};
use crate::pure::living_fall_v1::{self, LivingFall};
use crate::pure::recipe_api_v1;
use crate::pure::player_api::{self, PlayerOps};
use mod_api::ItemStack;
use crate::pure::base::{Identifier, Nbt, NbtCompound};
use crate::pure::block_entity_api_v1::{self, FluidStack, ItemSlot};
use crate::pure::neighbour_api_v1::{self, NeighbourOps, NeighbourSlot, NeighbourTank};
use crate::pure::entity_api_v1::{self, EntityOps};
use crate::pure::dispense_api_v1::{self, DispenseBehavior};
use crate::pure::fishing_api_v1::{self, Fishing, Grapple};
use crate::pure::fov_api_v1::{self, FovSlot, FovStack};
use crate::pure::highlight_api_v1;
use crate::pure::fluid_storage_api_v1::{BlockEntityStore, FluidStorage};
use crate::pure::item_storage_api_v1::ItemStorage;
use crate::pure::block_entity_renderer_api_v1::{self, BlockEntityCanvas, ItemDisplayContext, ModelPart, ModelPolygon, ModelVertex, PartPose};
use crate::pure::screen_handler_api_v1::{self, ScreenCanvas};
use crate::pure::player_api::{EffectInstance, FoodStatus};
use crate::pure::world_api_v1::{self, BlockState, WorldOps};
use crate::pure::book_api_v1::{self, BookOps, BookView};
use crate::pure::projectile_api_v1;
use crate::pure::{projectile_events_v1, projectile_events_v1::ProjectileImpact};
use crate::pure::block_api_v1;
use crate::pure::command_api_v2;
use crate::pure::config_api::{self, ConfigKind, ModConfig};
use crate::pure::{events_interaction_v0, hanging_api_v1, hanging_api_v1::HangingRegistry, hud_api_v1, item_api_v1, lifecycle_events_v1, loot_api_v2, mob_split_api, networking_api_v1, resource_loader_v0};

fn resolve_dynamic_screen_slots(
    block: &str,
    data: &[i32],
    stacks: &[Option<ItemStack>],
) -> Option<Vec<Option<[f32; 2]>>> {
    let items = stacks
        .iter()
        .map(|stack| match stack {
            Some(stack) => slot_of(stack).map(Some),
            None => Some(None),
        })
        .collect::<Option<Vec<_>>>()?;
    // The refresh runs for every open mod menu (the resolver is set unconditionally), so this
    // is also where the client's view of the menu's state is kept for the draw and scroll
    // callbacks: the items a side panel reads, the data a scroll reads.
    screen_handler_api_v1::note_menu_state(block, data, &items);
    screen_handler_api_v1::dynamic_slots(block, data, &items)
}

/// What a mod's `hooks()` returns: the launcher's entry point type. A mod that registers everything through PlutoLib's events returns
/// [`no_hooks`], so its source never names a Horizon type.
pub type ModHooks = Box<dyn mod_api::Hooks>;

#[derive(Default)]
struct NoHooks;
#[mod_api::sunrise_mod]
impl mod_api::Hooks for NoHooks {}

/// Hooks that do nothing, for a mod whose behaviour is all registered through PlutoLib events.
pub fn no_hooks() -> ModHooks {
    Box::new(NoHooks)
}

/// PlutoLib's player operations over Horizon's `Host`, open only while a callback that carries a player runs.
/// A pointer to the host that is turned into a reference only for the length of one call, so the player scope and the world scope can both
/// hold it without two live `&mut`.
fn component_tag<'a>(stack: &'a ItemStack, id: &str) -> Option<&'a Tag> {
    stack.components()?.as_nbt().get(&id.into())
}

/// The stack's item id as text; an item the registry cannot name reads as air.
fn stack_item_name(stack: &ItemStack) -> String {
    mod_api::__private::item_key_id(stack.item()).unwrap_or("minecraft:air").to_owned()
}

pub(crate) fn read_stack(stack: &ItemStack) -> player_api::StackData {
    let custom_data = match component_tag(stack, "minecraft:custom_data") {
        Some(Tag::Compound(compound)) => compound_from_tag(compound),
        _ => NbtCompound::new(),
    };
    let max_damage = match component_tag(stack, "minecraft:max_damage") {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    };
    player_api::StackData { item: stack_item_name(stack), count: stack.count(), damage: stack.damage(), max_damage, custom_data, custom_model_data: read_custom_model_data(stack), enchantments: read_enchantments(stack).unwrap_or_default() }
}

/// The stack's own `minecraft:custom_model_data` component as PlutoLib's `CustomModelData`, None when it has none.
fn read_custom_model_data(stack: &ItemStack) -> Option<player_api::CustomModelData> {
    match component_tag(stack, "minecraft:custom_model_data") {
        Some(Tag::Compound(compound)) => Some(player_api::CustomModelData::from_nbt(&compound_from_tag(compound))),
        _ => None,
    }
}

/// The stack's own `minecraft:profile` component (a player head's owner), None when it has none.
fn read_profile(stack: &ItemStack) -> Option<NbtCompound> {
    match component_tag(stack, "minecraft:profile") {
        Some(Tag::Compound(compound)) => Some(compound_from_tag(compound)),
        _ => None,
    }
}

/// The component value for `data`: None (remove the component) when there is none or it is empty.
fn custom_model_data_tag(data: &Option<player_api::CustomModelData>) -> Option<Tag> {
    data.as_ref().filter(|data| !data.is_empty()).map(|data| Tag::Compound(compound_to_tag(&data.to_nbt())))
}


/// A pointer to the server host that is turned into a reference only for the length of one
/// call, so the player scope and the world scope can both hold it without two live `&mut`.
#[derive(Clone, Copy)]
pub(crate) struct ServerHandle(*mut dyn mod_api::ServerHost);

impl ServerHandle {
    pub(crate) fn new(host: &mut dyn mod_api::ServerHost) -> Self {
        Self(unsafe { core::mem::transmute::<&mut dyn mod_api::ServerHost, *mut dyn mod_api::ServerHost>(host) })
    }

    pub(crate) fn get(&mut self) -> &mut dyn mod_api::ServerHost {
        unsafe { &mut *self.0 }
    }
}

/// The client host, held the same way for the client-side scopes.
#[derive(Clone, Copy)]
pub(crate) struct ClientHandle(*mut dyn mod_api::ClientHost);

impl ClientHandle {
    pub(crate) fn new(host: &mut dyn mod_api::ClientHost) -> Self {
        Self(unsafe { core::mem::transmute::<&mut dyn mod_api::ClientHost, *mut dyn mod_api::ClientHost>(host) })
    }

    pub(crate) fn get(&mut self) -> &mut dyn mod_api::ClientHost {
        unsafe { &mut *self.0 }
    }
}

/// Runs `f` with the player scope, the world scope and the book scope all open over the host.
pub(crate) fn scoped<R>(host: &mut dyn mod_api::ServerHost, f: impl FnOnce() -> R) -> R {
    let mut handle = ServerHandle::new(host);
    player_api::with_ops(&mut HostOps(handle), || {
        world_api_v1::with_world(&mut HostWorld(handle), || book_api_v1::with_books(&mut HostBookOps(handle), f))
    })
}

fn intern_sound(id:&str)->&'static str {
    static INTERN:std::sync::Mutex<Vec<&'static str>>=std::sync::Mutex::new(Vec::new());
    let mut table=INTERN.lock().unwrap();
    if let Some(found)=table.iter().find(|known|**known==id) { return found; }
    let leaked:&'static str=Box::leak(id.to_string().into_boxed_str());
    table.push(leaked);
    leaked
}

pub(crate) fn native_slot(slot: Option<ItemSlot>) -> Option<ItemStack> {
    slot.filter(|slot| slot.count > 0).and_then(|slot| {
        let key = mod_api::__private::item_key(&slot.item.to_string())?;
        let mut stack = ItemStack::new(key, slot.count)?;
        write_slot(&mut stack, &slot);
        Some(stack)
    })
}

struct HostOps(ServerHandle);

struct HostWorld(ServerHandle);


impl HostOps {
    fn host(&mut self) -> &mut dyn mod_api::ServerHost {
        self.0.get()
    }
}

/// The JSON attachment map a holder carries under one of the multiplexed keys: consumer
/// attachment id to its JSON text, as one JSON object.
fn attachment_map(host: &mut dyn mod_api::ServerHost, holder: Holder, sync: bool) -> Option<serde_json::Map<String, serde_json::Value>> {
    let key = json_key(sync)?;
    let text = key.get_existing(host, holder, |value| value.cloned()).ok()??;
    if text.is_empty() {
        return Some(serde_json::Map::new());
    }
    match serde_json::from_str(&text) {
        Ok(serde_json::Value::Object(map)) => Some(map),
        _ => None,
    }
}

fn attachment_read(host: &mut dyn mod_api::ServerHost, holder: Holder, id: &str, sync: bool) -> Option<String> {
    let map = attachment_map(host, holder, sync)?;
    match map.get(id) {
        Some(serde_json::Value::String(text)) if !text.is_empty() => Some(text.clone()),
        _ => None,
    }
}

fn attachment_write(host: &mut dyn mod_api::ServerHost, holder: Holder, id: &str, json: &str, sync: bool) -> bool {
    let Some(key) = json_key(sync) else { return false };
    let Some(mut map) = attachment_map(host, holder, sync) else { return false };
    map.insert(id.to_owned(), serde_json::Value::String(json.to_owned()));
    let text = serde_json::Value::Object(map).to_string();
    key.set(host, holder, text).is_ok()
}

impl PlayerOps for HostOps {
    fn net_send_to_player(&mut self, player: u128, channel: &str, data: &[u8]) -> Result<(), String> {
        let host = self.host();
        let target = host.player(player).ok_or_else(|| "no such player".to_owned())?;
        target.send_payload(host, &PayloadId(channel.to_owned()), PayloadBytes(data.to_vec()))
            .map(|_| ())
            .map_err(|error| format!("{error:?}"))
    }

    fn tell(&mut self, player: u128, line: &str) -> bool {
        let host = self.host();
        let Some(target) = host.player(player) else { return false };
        target.tell(host, Text::literal(line.to_owned())).unwrap_or(false)
    }

    fn give(&mut self, player: u128, item: &str, count: u32) -> u32 {
        let host = self.host();
        let Some(target) = host.player(player) else { return count };
        let Some(key) = mod_api::__private::item_key(item) else { return count };
        let Some(stack) = ItemStack::new(key, count) else { return count };
        match target.give_stack(host, stack) {
            Ok(Some(rest)) => rest.count(),
            Ok(None) => 0,
            Err(_) => count,
        }
    }

    fn give_stack(&mut self, player: u128, stack: player_api::StackData) -> Option<player_api::StackData> {
        let host = self.host();
        let target = host.player(player)?;
        let key = mod_api::__private::item_key(&stack.item)?;
        let mut native = ItemStack::new(key, stack.count.max(1))?;
        let mut slot = ItemSlot::new(Identifier::parse(&stack.item).ok()?, stack.count);
        slot.damage = stack.damage;
        slot.max_damage = stack.max_damage;
        slot.custom_data = stack.custom_data.clone();
        slot.custom_model_data = stack.custom_model_data.clone();
        slot.enchantments = Some(stack.enchantments.clone());
        write_slot(&mut native, &slot);
        match target.give_stack(host, native) {
            Ok(Some(rest)) => Some(read_stack(&rest)),
            Ok(None) => None,
            Err(_) => Some(stack),
        }
    }

    fn open_item_menu(&mut self, player: u128, menu: &str, title: &str, items: Vec<Option<crate::pure::block_entity_api_v1::ItemSlot>>, extra: Vec<u8>) -> Option<i32> {
        let host = self.host();
        let target = host.player(player)?;
        let key = {
            let host_ref: &dyn mod_api::Host = host;
            mod_api::__private::menu_keys().into_iter()
                .find(|key| key.id(host_ref).is_some_and(|name| name.as_str() == menu))?
        };
        let mut open = mod_api::ModMenuOpen::new(key, Text::literal(title.to_owned()));
        open.extra = extra;
        // The open call itself carries no contents; the seed moves into the
        // live menu from the player_container_open hook (parts::player_container_open).
        crate::parts::note_item_menu_seed(player, items);
        let reference = target.open_mod_menu(host, open).ok()??;
        reference.container_id(host)
    }

    fn attachment_get(&mut self, player: u128, id: &str) -> Option<String> {
        self.attachment_get_with_policy(player, id, false)
    }

    fn attachment_set(&mut self, player: u128, id: &str, json: &str) -> bool {
        self.attachment_set_with_policy(player, id, json, false)
    }

    fn attachment_get_with_policy(&mut self, player: u128, id: &str, sync: bool) -> Option<String> {
        attachment_read(self.host(), Holder::Player(player), id, sync)
    }

    fn attachment_set_with_policy(&mut self, player: u128, id: &str, json: &str, sync: bool) -> bool {
        attachment_write(self.host(), Holder::Player(player), id, json, sync)
    }

    fn entity_attachment_get(&mut self, entity: u32, _dimension: &str, id: &str, sync: bool) -> Option<String> {
        // The attachment store the host lends is already the level in context, so the old
        // dimension check has nothing left to compare against.
        attachment_read(self.host(), Holder::Entity(entity), id, sync)
    }

    fn entity_attachment_set(&mut self, entity: u32, _dimension: &str, id: &str, json: &str, sync: bool) -> bool {
        attachment_write(self.host(), Holder::Entity(entity), id, json, sync)
    }

    fn count_item(&mut self, player: u128, item: &str) -> u32 {
        let host = self.host();
        let Some(target) = host.player(player) else { return 0 };
        target.with_inventory(host, |view| {
            view.iter().filter(|(_, stack)| mod_api::__private::item_key_id(stack.item()) == Some(item)).map(|(_, stack)| stack.count()).sum()
        }).unwrap_or(0)
    }

    fn take_item(&mut self, player: u128, item: &str, count: u32) -> u32 {
        let host = self.host();
        let Some(target) = host.player(player) else { return 0 };
        let occupied: Vec<(u32, ItemStack)> = target.with_inventory(host, |view| {
            view.iter().filter(|(_, stack)| mod_api::__private::item_key_id(stack.item()) == Some(item)).filter_map(|(slot, stack)| stack.project().map(|owned| (slot, owned))).collect()
        }).unwrap_or_default();
        let mut left = count;
        for (slot, stack) in occupied {
            if left == 0 {
                break;
            }
            let taken = left.min(stack.count());
            left -= taken;
            let rest = stack.count() - taken;
            let replacement = if rest == 0 {
                None
            } else {
                let mut shrunk = stack.clone();
                let _ = shrunk.set_count(rest);
                Some(shrunk)
            };
            if target.set_inventory_slot(host, slot, replacement).is_err() {
                left += taken;
                break;
            }
        }
        count - left
    }

    fn empty_slots(&mut self, player: u128) -> u32 {
        let host = self.host();
        let Some(target) = host.player(player) else { return 0 };
        target.with_inventory(host, |view| (0..43).filter(|slot| view.get(*slot).is_none()).count() as u32).unwrap_or(0)
    }

    fn language(&mut self, _player: u128) -> Option<String> {
        // No server-side locale read exists in mod_api; the client language lives on the
        // client host. See the L2 disposition table.
        None
    }

    fn add_effect(&mut self, player: u128, effect: &EffectInstance) -> bool {
        let host = self.host();
        let Some(target) = host.player(player) else { return false };
        let Some(living) = host.living(target.entity()) else { return false };
        let Some(key) = mod_api::__private::effect_key(&effect.id) else { return false };
        let active = mod_api::ActiveEffect::new(key, effect.duration, i32::from(effect.amplifier), effect.ambient, effect.particles, effect.show_icon);
        living.add_effect(host, &active)
    }

    fn remove_effect(&mut self, player: u128, id: &str) -> bool {
        let host = self.host();
        let Some(target) = host.player(player) else { return false };
        let Some(living) = host.living(target.entity()) else { return false };
        let Some(key) = mod_api::__private::effect_key(id) else { return false };
        living.remove_effect(host, key)
    }

    fn effects(&mut self, player: u128) -> Option<Vec<EffectInstance>> {
        let host = self.host();
        let target = host.player(player)?;
        let living = host.living(target.entity())?;
        let mut found = Vec::new();
        let _ = living.visit_effects(host, &mut |effect| {
            found.push(effect);
            core::ops::ControlFlow::Continue(())
        });
        Some(found.into_iter().map(|effect| EffectInstance {
            id: mod_api::__private::effect_key_id(effect.id).unwrap_or_default().to_owned(),
            amplifier: u8::try_from(effect.amplifier).unwrap_or(u8::MAX),
            duration: effect.duration,
            ambient: effect.ambient,
            particles: effect.visible,
            show_icon: effect.show_icon,
        }).collect())
    }

    fn food(&mut self, player: u128) -> Option<FoodStatus> {
        let host = self.host();
        let target = host.player(player)?;
        target.food(host).map(|food| FoodStatus { level: food.level, saturation: food.saturation, exhaustion: food.exhaustion })
    }

    fn add_food(&mut self, player: u128, nutrition: i32, saturation: f32) -> bool {
        let Some(before) = self.food(player) else { return false };
        let Some(after) = player_api::fed_food(before, nutrition, saturation) else { return false };
        let host = self.host();
        let Some(target) = host.player(player) else { return false };
        target.set_food_level(host, after.level)
            && target.set_saturation(host, after.saturation)
            && target.set_exhaustion(host, after.exhaustion)
    }

    fn selected_slot(&mut self, player: u128) -> Option<i32> {
        let host = self.host();
        let target = host.player(player)?;
        target.selected_slot(host).map(|slot| slot as i32)
    }
}

impl WorldOps for HostWorld {
    fn block_state(&mut self, dimension: &str, pos: block_entity_api_v1::BlockPos) -> Option<String> {
        let host = self.0.get();
        let level = host.level(dimension)?;
        let state = level.block_state(host, pos)?;
        host.block_state_string(state)
    }

    fn set_block_state(&mut self, dimension: &str, pos: block_entity_api_v1::BlockPos, state: &str) -> bool {
        let host = self.0.get();
        let Some(level) = host.level(dimension) else { return false };
        let Some(state) = host.block_state_id(state) else { return false };
        level.set_block(host, pos, state, mod_api::BlockUpdateFlags::NEIGHBORS | mod_api::BlockUpdateFlags::CLIENTS).unwrap_or(false)
    }

    fn play_sound(&mut self, dimension: &str, sound: &str, at: [f64; 3], volume: f32, pitch: f32) -> bool {
        let host = self.0.get();
        let Some(level) = host.level(dimension) else { return false };
        level.play_sound(host, sound, at, SoundSource::Blocks, volume, pitch, None)
    }

    fn build_height(&mut self, dimension: &str) -> Option<[i32; 2]> {
        let host = self.0.get();
        let level = host.level(dimension)?;
        level.height(host).map(|(min, max)| [min, max])
    }

    fn block_in_tag(&mut self, block: &str, tag: &str) -> bool {
        let host = self.0.get();
        let Some(block) = mod_api::__private::block_key(block) else { return false };
        let Some(tag_id) = mod_api::Identifier::parse_namespaced(tag) else { return false };
        let Some(tag) = mod_api::__private::tag_key(&tag_id) else { return false };
        host.tags().block_in_tag(block, tag).unwrap_or(false)
    }
}

const PLUTO_MOD_ID: &str = "pluto_lib";

/// The book the reader is reading now, kept on the host side because the screen hooks that draw it and
/// turn its pages do not run inside a callback scope. Cleared when the book screen closes.
static BOOK_VIEW: std::sync::Mutex<Option<BookView>> = std::sync::Mutex::new(None);

/// PlutoLib's book reach over Horizon's mod screen: opening asks the game to open PlutoLib's own screen
/// over the world, and the screen hooks below draw the current page into it. Opening is a client act —
/// off the client thread the host answers false and nothing opens.
struct HostBookOps(ServerHandle);

impl BookOps for HostBookOps {
    // Sunrise has no host-opened mod screen: a book screen is a client ScreenPart the mod
    // registers itself (L6). Until that part exists the book reach answers unavailable, and
    // the pure book registry keeps the data. See the L2 disposition table.
    fn open_book(&mut self, _book: &Identifier) -> bool {
        false
    }

    fn close_book(&mut self) -> bool {
        false
    }

    fn open_book_id(&mut self) -> Option<Identifier> {
        None
    }
}

/// Whether the screen showing is PlutoLib's own mod screen.
fn open_book_view() -> Option<(book_api_v1::Book, BookView)> {
    let view = BOOK_VIEW.lock().ok()?.clone()?;
    let book = book_api_v1::BookRegistry::get(&view.book)?;
    Some((book, view))
}

fn set_book_view(view: BookView) {
    if let Ok(mut held) = BOOK_VIEW.lock() {
        *held = Some(view);
    }
}

/// The host's face for a PlutoLib one (the same six, in the same order).
fn host_face(face: block_api_v1::Direction) -> mod_api::Direction {
    mod_api::Direction::ALL[face as usize]
}

/// A PlutoLib face for the host's one (the same six, in the same order).
pub(crate) fn pure_face(face: mod_api::Direction) -> block_api_v1::Direction {
    block_api_v1::Direction::ALL[face as usize]
}

fn fluid_stack_of(fluid: &str, amount: i32) -> Option<FluidStack> {
    let fluid = Identifier::parse(fluid).ok()?;
    u32::try_from(amount).ok().filter(|&amount| amount > 0).map(|amount| FluidStack { fluid, amount, tag: None })
}

/// How much of `offered` the block entity at `pos` takes by its own fluid rule (`BlockEntityType::fluid_filler`); `None` when its block
/// has none. With `execute` the filler's changes are kept (and sent to the clients), otherwise they are thrown away.
struct HostBlockEntities {
    host: ServerHandle,
    dimension: String,
}

impl BlockEntityStore for HostBlockEntities {
    // Sunrise reaches a block entity's data and slots through its Part state and capability
    // providers, not through a host-side data/slot mirror; the store is re-based on those by
    // the block-entity work (L3/L6). Until then the store answers unavailable. See the L2
    // disposition table.
    fn block_at(&mut self, _pos: [i32; 3]) -> Option<String> {
        None
    }

    fn read(&mut self, _pos: [i32; 3]) -> Option<(NbtCompound, Vec<Option<ItemSlot>>)> {
        None
    }

    fn write(&mut self, _pos: [i32; 3], _data: NbtCompound, _items: Vec<Option<ItemSlot>>) {}
}

/// The containers, tanks and block states beside a ticking block entity, over the host.
struct HostNeighbours {
    host: ServerHandle,
    dimension: String,
}

impl HostNeighbours {
    /// The block entity store of the level in place, for the fluid queries answered through a block's own hooks.
    fn store(&mut self) -> HostBlockEntities {
        HostBlockEntities { host: self.host, dimension: self.dimension.clone() }
    }

    /// Whether the block at `pos` holds its fluid storage in its saved data, read through its type's hooks: those
    /// queries are answered through [`FluidStorage`] over the store, because the host's own tank calls cannot serve
    /// them (see `block_entity_api_v1::fluid_served_from_data`).
    fn served_from_data(&mut self, pos: [i32; 3]) -> bool {
        let _ = pos;
        false
    }

    /// Whether the block at `pos` holds its item storage in its saved data, answered through its type's hooks (an item
    /// forward, a sided access or a transfer rule): those queries are answered through [`ItemStorage`] over the store,
    /// because the host's own container calls open every slot every way and cannot serve them (see
    /// `block_entity_api_v1::item_served_from_data`).
    fn item_served_from_data(&mut self, pos: [i32; 3]) -> bool {
        let _ = pos;
        false
    }
}

impl NeighbourOps for HostNeighbours {
    // Neighbour item/fluid transfer in Sunrise runs through the capability providers a block
    // entity Part declares (the A1/A8 surface), not through host container calls. The ops
    // answer unavailable until the capability-based store lands (L3/L6); see the L2
    // disposition table.
    fn item_slots(&mut self, _pos: [i32; 3], _face: block_api_v1::Direction) -> Option<Vec<NeighbourSlot>> {
        None
    }

    fn extract_item(&mut self, _pos: [i32; 3], _face: block_api_v1::Direction, _slot: usize, _count: u32) -> Option<ItemSlot> {
        None
    }

    fn insert_item(&mut self, _pos: [i32; 3], _face: block_api_v1::Direction, _stack: ItemSlot) -> Option<ItemSlot> {
        None
    }

    fn fluid_tanks(&mut self, _pos: [i32; 3], _face: block_api_v1::Direction) -> Option<Vec<NeighbourTank>> {
        None
    }

    fn drain_fluid(&mut self, _pos: [i32; 3], _face: block_api_v1::Direction, _amount: u32) -> Option<FluidStack> {
        None
    }

    fn fill_fluid(&mut self, _pos: [i32; 3], _face: block_api_v1::Direction, _stack: &FluidStack, _execute: bool) -> u32 {
        0
    }

    fn block_state(&mut self, _pos: [i32; 3]) -> Option<String> {
        None
    }
}

/// Whether an effect id names fire resistance (the host lists a mob's effects with or without the namespace).
fn is_fire_resistance(effect: &str) -> bool {
    effect == "minecraft:fire_resistance" || effect == "fire_resistance"
}

/// The entities of the level in place, for a ticker: item entities with their stacks, and living entities with the
/// facts a caller's own rules decide with (fire immunity, invulnerability, fire resistance).
struct HostEntities {
    host: ServerHandle,
    dimension: String,
}

impl HostEntities {
    /// The numeric id to queue a push for, when the entity can be named: the host's entity call names
    /// it, or it is a player the player list names (the entity call does not name every player; see
    /// `position_of`). The push is queued either way, and the game's own resolution at its rider
    /// synchronisation has the last word.
fn push_target(&mut self, id: u64) -> Option<u32> {
        // The host's entity naming the Horizon bridge used has no mod_api counterpart; the
        // queue carries the numeric id and the game-side rider synchronisation resolves it.
        u32::try_from(id).ok()
    }
}

struct PendingBreak { pos: [i32; 3], dimension: String, state: Option<BlockState>, player: player_api::ServerPlayer, equipment: Vec<(String, player_api::StackData)>, face: Option<block_api_v1::Direction>, yaw: f32, pitch: f32, creative: bool, may_build: bool, offhand_place_on: Vec<String>, offhand_place_predicates: Vec<events_interaction_v0::AdventurePlacePredicate>, mined_block_entity: Option<NbtCompound> }
static PENDING_BREAKS: std::sync::Mutex<Vec<PendingBreak>> = std::sync::Mutex::new(Vec::new());

/// The face each player last started digging, by uuid: the break hook itself is not told the face, but the dig's start is
/// (`left_click_block`), and a break belongs to the dig the player last started.
static LAST_DIG_FACE: std::sync::Mutex<std::collections::BTreeMap<u128, block_api_v1::Direction>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// The last redstone strength reported to each block entity that hears signal changes, by dimension and position: a
/// change is what is reported, and the first sight of a block entity counts as one.
static LAST_SIGNAL: std::sync::Mutex<std::collections::BTreeMap<(String, [i32; 3]), u8>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// The acting player, their dimension and main-hand stack, and the block at `pos`, as the block break events report them.
fn adventure_place_entries(stack: &ItemStack) -> Vec<String> {
    let Some(tag) = component_tag(stack, "minecraft:can_place_on") else { return Vec::new() };
    let Some(nbt) = nbt_from_tag(tag) else { return Vec::new() };
    let mut entries = Vec::new();
    match &nbt {
        Nbt::Compound(component) => {
            if let Some(Nbt::List(predicates)) = component.get("blocks") {
                for predicate in predicates { adventure_place_entry(predicate, &mut entries); }
            }
        }
        Nbt::List(predicates) => { for predicate in predicates { adventure_place_entry(predicate, &mut entries); } }
        Nbt::String(spec) => entries.push(spec.clone()),
        _ => {}
    }
    entries
}

/// One block predicate of a place tag, as its text entries (see `adventure_place_entries`): a compound names its
/// blocks (one id or a list) and may narrow them with a state test, whose entries ride as the suffix.
fn adventure_place_entry(predicate: &Nbt, entries: &mut Vec<String>) {
    match predicate {
        Nbt::String(spec) => entries.push(spec.clone()),
        Nbt::Compound(predicate) => {
            if predicate.get("nbt").is_some() { return; }
            let suffix = match predicate.get("state") {
                Some(Nbt::Compound(state)) if !state.is_empty() => {
                    let mut pairs = Vec::new();
                    for (key, value) in state.iter() {
                        let Nbt::String(value) = value else { return };
                        pairs.push(format!("{key}={value}"));
                    }
                    format!("[{}]", pairs.join(","))
                }
                _ => String::new(),
            };
            match predicate.get("blocks") {
                Some(Nbt::String(spec)) => entries.push(format!("{spec}{suffix}")),
                Some(Nbt::List(specs)) => { for spec in specs { if let Nbt::String(spec) = spec { entries.push(format!("{spec}{suffix}")); } } }
                _ => {}
            }
        }
        _ => {}
    }
}

/// The break the game's break-completion seam reports (see `pluto_break_complete_access`): the break at
/// `completed.pos` in `completed.dimension` went through and the block is gone. Runs its `AFTER` now, inside
/// the break, while the breaking player is still the acting one — before the removal is sent to the players
/// around. A pending break the seam does not name stays queued for the next tick, as before.
fn nbt_from_tag(tag: &Tag) -> Option<Nbt> {
    Some(match tag {
        Tag::End => return None,
        Tag::Byte(v) => Nbt::Byte(*v),
        Tag::Short(v) => Nbt::Short(*v),
        Tag::Int(v) => Nbt::Int(*v),
        Tag::Long(v) => Nbt::Long(*v),
        Tag::Float(bits) => Nbt::Float(f32::from_bits(*bits)),
        Tag::Double(bits) => Nbt::Double(f64::from_bits(*bits)),
        Tag::ByteArray(v) => Nbt::ByteArray(v.clone()),
        Tag::String(s) => Nbt::String(s.to_string_lossy()),
        Tag::List(items) => Nbt::List(items.iter().filter_map(nbt_from_tag).collect()),
        Tag::Compound(c) => Nbt::Compound(compound_from_tag(c)),
        Tag::IntArray(v) => Nbt::IntArray(v.clone()),
        Tag::LongArray(v) => Nbt::LongArray(v.clone()),
    })
}

pub(crate) fn tag_from_nbt(nbt: &Nbt) -> Tag {
    match nbt {
        Nbt::Byte(v) => Tag::Byte(*v),
        Nbt::Short(v) => Tag::Short(*v),
        Nbt::Int(v) => Tag::Int(*v),
        Nbt::Long(v) => Tag::Long(*v),
        Nbt::Float(v) => Tag::Float(v.to_bits()),
        Nbt::Double(v) => Tag::Double(v.to_bits()),
        Nbt::ByteArray(v) => Tag::ByteArray(v.clone()),
        Nbt::String(s) => Tag::String(s.as_str().into()),
        Nbt::List(items) => Tag::List(items.iter().map(tag_from_nbt).collect()),
        Nbt::Compound(c) => Tag::Compound(compound_to_tag(c)),
        Nbt::IntArray(v) => Tag::IntArray(v.clone()),
        Nbt::LongArray(v) => Tag::LongArray(v.clone()),
    }
}

/// Edits the custom data of the stack the acting player carries in `slot` (`Host::carried_stack` by the inventory's slot numbers: the selected
/// hotbar slot, 100 to 103 for armor, -106 for the off hand), when it is still of `item`; the whole stack goes back through the same
/// `slot_of` / `write_slot` pair the item-wear adapter uses, so every other component stays as the game had it.
pub(crate) fn compound_from_tag(compound: &Compound) -> NbtCompound {
    compound.iter().filter_map(|(key, tag)| Some((key.to_string_lossy(), nbt_from_tag(tag)?))).collect()
}

pub(crate) fn compound_to_tag(compound: &NbtCompound) -> Compound {
    compound.iter().map(|(key, nbt)| (key.as_str().into(), tag_from_nbt(nbt))).collect()
}

/// A custom data compound from its JSON form, for transports that carry data as JSON (the item
/// behaviour transport). JSON has no byte or null: booleans read as 0/1 bytes and null as an
/// empty string; the queries this feeds inspect keys, structure and strings, which JSON keeps.
fn nbt_from_json(value: &serde_json::Value) -> NbtCompound {
    match value {
        serde_json::Value::Object(map) => map.iter().map(|(key, value)| (key.clone(), nbt_value_from_json(value))).collect(),
        _ => NbtCompound::new(),
    }
}

fn nbt_value_from_json(value: &serde_json::Value) -> Nbt {
    match value {
        serde_json::Value::Null => Nbt::String(String::new()),
        serde_json::Value::Bool(flag) => Nbt::Byte(i8::from(*flag)),
        serde_json::Value::Number(number) => {
            if let Some(int) = number.as_i64() {
                if let Ok(int) = i32::try_from(int) { Nbt::Int(int) } else { Nbt::Long(int) }
            } else {
                Nbt::Double(number.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(text) => Nbt::String(text.clone()),
        serde_json::Value::Array(values) => Nbt::List(values.iter().map(nbt_value_from_json).collect()),
        serde_json::Value::Object(_) => Nbt::Compound(nbt_from_json(value)),
    }
}

/// A stack as an item slot: item, count, damage, max damage and custom data; None for an item id PlutoLib cannot name.
/// The face of `hit`'s block the view enters it through, as a unit offset: the hit
/// point lies on that face's plane, so it is the face whose plane is nearest the
/// point. A point tied between faces (an edge or a corner) reads as the first of
/// them in west, east, down, up, north, south order.
pub(crate) fn slot_of(stack: &ItemStack) -> Option<ItemSlot> {
    let data = read_stack(stack);
    let mut slot = ItemSlot::new(crate::pure::base::Identifier::parse(&data.item).ok()?, data.count);
    slot.damage = data.damage;
    slot.max_damage = data.max_damage;
    slot.custom_data = data.custom_data;
    slot.tool = read_tool(stack);
    slot.attribute_modifiers = read_attribute_modifiers(stack);
    slot.enchantments = read_enchantments(stack);
    slot.custom_model_data = data.custom_model_data;
    slot.profile = read_profile(stack);
    slot.map_id = match component_tag(stack, "minecraft:map_id") {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    };
    slot.capture(stack.clone());
    Some(slot)
}

/// A custom data compound in its JSON form, for transports that carry data as JSON (the loot
/// function's stack keeps its components as JSON). The mirror of `nbt_from_json`: bytes and
/// booleans have no JSON form of their own, so a byte rides as its number.
fn json_from_compound(compound: &NbtCompound) -> serde_json::Value {
    serde_json::Value::Object(compound.iter().map(|(key, value)| (key.clone(), json_value_from_nbt(value))).collect())
}

fn json_value_from_nbt(nbt: &Nbt) -> serde_json::Value {
    match nbt {
        Nbt::Byte(value) => serde_json::Value::from(*value),
        Nbt::Short(value) => serde_json::Value::from(*value),
        Nbt::Int(value) => serde_json::Value::from(*value),
        Nbt::Long(value) => serde_json::Value::from(*value),
        Nbt::Float(value) => serde_json::Number::from_f64(f64::from(*value)).map_or(serde_json::Value::Null, serde_json::Value::Number),
        Nbt::Double(value) => serde_json::Number::from_f64(*value).map_or(serde_json::Value::Null, serde_json::Value::Number),
        Nbt::ByteArray(values) => serde_json::Value::Array(values.iter().map(|value| serde_json::Value::from(*value)).collect()),
        Nbt::String(value) => serde_json::Value::from(value.clone()),
        Nbt::List(values) => serde_json::Value::Array(values.iter().map(json_value_from_nbt).collect()),
        Nbt::Compound(compound) => json_from_compound(compound),
        Nbt::IntArray(values) => serde_json::Value::Array(values.iter().map(|value| serde_json::Value::from(*value)).collect()),
        Nbt::LongArray(values) => serde_json::Value::Array(values.iter().map(|value| serde_json::Value::from(*value)).collect()),
    }
}

/// A loot roll's stack as an item slot: item, count, damage, enchantments and the
/// `minecraft:custom_data` component (the one a loot function of a mod's own kind reads and
/// writes); None for an item id PlutoLib cannot name.
fn write_slot(stack: &mut ItemStack, slot: &ItemSlot) {
    let item = slot.item.to_string();
    let captured = slot.captured::<ItemStack>().filter(|old| stack_item_name(old) == item).cloned();
    let built = captured.or_else(|| mod_api::__private::item_key(&item).and_then(|key| ItemStack::new(key, slot.count.max(1))));
    let Some(built) = built else { return };
    *stack = built;
    if slot.count > 0 && stack.count() != slot.count {
        let _ = stack.set_count(slot.count);
    }
    if stack.damage() != slot.damage {
        if slot.damage == 0 {
            stack.clear_damage();
        } else {
            stack.set_damage(slot.damage);
        }
    }
    let before = read_stack(stack);
    let mut patch = Compound::new();
    let mut set_component = |patch: &mut Compound, id: &str, value: Option<Tag>| match value {
        Some(tag) => {
            patch.insert(id.into(), tag);
        }
        None => {
            let removal = format!("!{id}");
            patch.insert(removal.as_str().into(), Tag::Compound(Compound::new()));
        }
    };
    if slot.max_damage != before.max_damage {
        set_component(&mut patch, "minecraft:max_damage", slot.max_damage.filter(|m| *m > 0).map(Tag::Int));
    }
    if slot.custom_data != before.custom_data {
        set_component(&mut patch, "minecraft:custom_data", (!slot.custom_data.is_empty()).then(|| Tag::Compound(compound_to_tag(&slot.custom_data))));
    }
    // The stack's own tool component; the native encoder/decoder and mining rules read it from here.
    if slot.tool != read_tool(stack) {
        set_component(&mut patch, "minecraft:tool", slot.tool.as_ref().map(|tool| Tag::Compound(compound_to_tag(tool))));
    }
    // The stack's own attribute modifiers: the host applies them to a player holding or wearing the stack.
    if slot.attribute_modifiers != read_attribute_modifiers(stack) {
        set_component(&mut patch, "minecraft:attribute_modifiers", slot.attribute_modifiers.as_ref()
            .map(|list| Tag::List(list.iter().map(|modifier| Tag::Compound(compound_to_tag(modifier))).collect())));
    }
    // The stack's own enchantments: the host applies them (fortune, silk touch, respiration...) to the holder or wearer.
    if slot.enchantments != read_enchantments(stack) {
        let component = enchantments_component(stack);
        set_component(&mut patch, component, slot.enchantments.as_ref().map(|levels| {
            let compound: NbtCompound = levels.iter().map(|(id, level)| (id.clone(), Nbt::Int(*level))).collect();
            Tag::Compound(compound_to_tag(&compound))
        }));
    }
    // The stack's own model selection: the item definition's `select`/`range_dispatch`/`condition` read it from here.
    if slot.custom_model_data != read_custom_model_data(stack) {
        set_component(&mut patch, "minecraft:custom_model_data", custom_model_data_tag(&slot.custom_model_data));
    }
    // The stack's own profile: a player head's owner, whose skin the head shows.
    if slot.profile != read_profile(stack) {
        set_component(&mut patch, "minecraft:profile", slot.profile.as_ref().map(|profile| Tag::Compound(compound_to_tag(profile))));
    }
    // The stack's own map id: a filled map's identity, which the map tick and the map face read.
    let before_map = match component_tag(stack, "minecraft:map_id") {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    };
    if slot.map_id != before_map {
        set_component(&mut patch, "minecraft:map_id", slot.map_id.map(Tag::Int));
    }
    if !patch.is_empty() {
        let _ = stack.apply_components(&patch);
    }
}

/// The component a stack's enchantments live in: vanilla keeps an enchanted book's in
/// `minecraft:stored_enchantments` (EnchantedBookItem reads and writes that one), everything
/// else's in `minecraft:enchantments`. A slot's enchantments are the stack's, whichever it is.
fn enchantments_component(stack: &ItemStack) -> &'static str {
    if stack_item_name(stack) == "minecraft:enchanted_book" { "minecraft:stored_enchantments" } else { "minecraft:enchantments" }
}

/// The stack's own enchantments component as enchantment id to level, None when it has none of its own.
fn read_enchantments(stack: &ItemStack) -> Option<std::collections::BTreeMap<String, i32>> {
    match component_tag(stack, enchantments_component(stack)) {
        Some(Tag::Compound(levels)) => Some(compound_from_tag(levels).into_iter().filter_map(|(id, level)| match level { Nbt::Int(level) => Some((id, level)), _ => None }).collect()),
        _ => None,
    }
}

/// The stack's own `minecraft:attribute_modifiers` component as public NBT, None when it has none of its own.
fn read_attribute_modifiers(stack: &ItemStack) -> Option<Vec<NbtCompound>> {
    match component_tag(stack, "minecraft:attribute_modifiers") {
        Some(Tag::List(items)) => Some(items.iter().filter_map(|item| match item { Tag::Compound(compound) => Some(compound_from_tag(compound)), _ => None }).collect()),
        _ => None,
    }
}

/// The stack's own `minecraft:tool` component as public NBT, None when it has none of its own.
fn read_tool(stack: &ItemStack) -> Option<NbtCompound> {
    match component_tag(stack, "minecraft:tool") {
        Some(Tag::Compound(compound)) => Some(compound_from_tag(compound)),
        _ => None,
    }
}

/// Runs `f` on public data/slots and writes back changes, retaining each moved slot's source components.
/// True if `f` did something (its own answer).
fn layer_to_native(layer: &block_entity_renderer_api_v1::LayerDefinition) -> mod_api::LayerDefinition {
    use mod_api::{CubeDeformation, CubeListBuilder, MeshDefinition, PartId};
    fn append(mesh: &mut MeshDefinition, parent: PartId, parts: &[(String, block_entity_renderer_api_v1::PartDefinition)]) {
        for (name, part) in parts {
            let mut cubes = CubeListBuilder::create();
            for cube in &part.cubes {
                let [x, y, z] = cube.deformation;
                cubes = cubes.tex_offs(cube.texture_offset[0], cube.texture_offset[1])
                    .set_mirror(cube.mirror)
                    .add_box_with(cube.origin, cube.dimensions, CubeDeformation::xyz(x, y, z));
            }
            let child = mesh.add_or_replace_child(parent, name, cubes, pose_to_native(part.pose));
            append(mesh, child, &part.children);
        }
    }
    let mut mesh = MeshDefinition::new();
    let root = mesh.root();
    append(&mut mesh, root, &layer.parts);
    mod_api::LayerDefinition::create(mesh, layer.texture_size[0], layer.texture_size[1])
}

fn pose_from_native(pose: mod_api::PartPose) -> PartPose {
    PartPose { translation: [pose.x, pose.y, pose.z], rotation: [pose.x_rot, pose.y_rot, pose.z_rot],
        scale: [pose.x_scale, pose.y_scale, pose.z_scale] }
}
fn pose_to_native(pose: PartPose) -> mod_api::PartPose {
    let mut native = mod_api::PartPose::offset_and_rotation(pose.translation[0], pose.translation[1], pose.translation[2], pose.rotation[0], pose.rotation[1], pose.rotation[2]);
    native.x_scale = pose.scale[0];
    native.y_scale = pose.scale[1];
    native.z_scale = pose.scale[2];
    native
}
fn model_from_native(part: &mod_api::ModelPart) -> ModelPart {
    let polygons = part.polygons.iter().map(|polygon| ModelPolygon {
        vertices: polygon.vertices.map(|v| ModelVertex { pos: v.pos, uv: [v.u, v.v] }),
        normal: polygon.normal,
    }).collect();
    let children = part.children().map(|(name, child)| (name.to_owned(), model_from_native(child))).collect();
    let mut model = ModelPart::new(polygons, children, pose_from_native(part.initial_pose()));
    model.pose = pose_from_native(part.store_pose());
    model.visible = part.visible;
    model.skip_draw = part.skip_draw;
    model
}
/// A worn stack as the item renderer's merged components describe it: the same fields the item colour provider hands a mod.
fn model_to_native(part: &ModelPart) -> mod_api::ModelPart {
    let polygons = part.polygons.iter().map(|polygon| mod_api::ModelPolygon::new(
        polygon.vertices.map(|v| mod_api::ModelVertex::new(v.pos, v.uv[0], v.uv[1])),
        polygon.normal,
    )).collect();
    let children = part.children.iter().map(|(name, child)| (name.clone(), model_to_native(child))).collect();
    let mut model = mod_api::ModelPart::new(polygons, children, pose_to_native(part.initial_pose));
    model.load_pose(pose_to_native(part.pose));
    model.visible = part.visible;
    model.skip_draw = part.skip_draw;
    model
}

/// What a block entity renderer draws on: the game's draw state for this block entity.
fn direction(face: u8) -> block_api_v1::Direction {
    block_api_v1::Direction::ALL[usize::from(face).min(5)]
}

static LOGGED_IN: std::sync::Mutex<Vec<(u128, String)>> = std::sync::Mutex::new(Vec::new());

fn remember_player(uuid: u128, name: &str, joined: bool) {
    let Ok(mut players) = LOGGED_IN.lock() else { return };
    players.retain(|(known, _)| *known != uuid);
    if joined { players.push((uuid, name.to_string())); }
}

/// The only player logged in (a single-player world's), None when there are none or several.
fn sole_player() -> Option<(u128, String)> {
    let players = LOGGED_IN.lock().ok()?;
    if players.len() == 1 { players.first().cloned() } else { None }
}

/// The two issued keys for PlutoLib's JSON attachments, filled by [`register`].
///
/// Horizon let the bridge mint an `AttachmentType` per consumer id at run time; Sunrise
/// attachments are declared once in `register`. So PlutoLib declares exactly two attachments —
/// `pluto_lib:json` and `pluto_lib:json_synced` — whose value is one JSON object text holding
/// every consumer attachment by id (the multiplex keeps the public per-id API unchanged).
static JSON_KEYS: std::sync::OnceLock<(AttachmentKey<String>, AttachmentKey<String>)> = std::sync::OnceLock::new();

fn json_codec() -> AttachmentCodec<String> {
    AttachmentCodec {
        encode: |json: &String| Ok([("json".into(), Tag::String(json.as_str().into()))].into()),
        decode: |data: &Compound| match data.get(&"json".into()) {
            Some(Tag::String(s)) => Ok(s.to_string_lossy()),
            _ => Err(AttachmentError::Decode),
        },
    }
}

/// The `Hooks::register` body: declare the JSON attachments and keep their issued keys.
pub(crate) fn register(registrar: &mut dyn Registrar, _cx: &RegistrationContext) {
    let plain = AttachmentDescriptor::<String>::new("pluto_lib:json", String::new).serialize(json_codec());
    let synced = AttachmentDescriptor::<String>::new("pluto_lib:json_synced", String::new).serialize(json_codec()).sync(json_codec());
    if let (Ok(plain), Ok(synced)) = (registrar.register_attachment_type(plain), registrar.register_attachment_type(synced)) {
        let _ = JSON_KEYS.set((plain, synced));
    }
    // The menu / block-entity / skull Parts and declarations of whatever the
    // pure registries already hold (consumers re-run it with their own
    // registrar after registering their types; duplicates are refused
    // harmlessly by the game).
    crate::parts::register_parts(registrar);
    // The BlockParts for every behaviour block the pure block registry holds.
    crate::block_part::register_parts(registrar);
}

/// The issued key for the JSON attachment of the given sync class, once registered.
pub(crate) fn json_key(sync: bool) -> Option<&'static AttachmentKey<String>> {
    JSON_KEYS.get().map(|(plain, synced)| if sync { synced } else { plain })
}

// ---- Hook bodies (called from the `#[sunrise_mod]` impl in lib.rs) ----

/// Actions a pure callback queued against a player (a shear's outcome, a loot
/// drop's rewards), applied through the host. Effect grants land directly;
/// the remaining kinds were handed to game-side module queues in the Horizon
/// bridge and are the module side's to pull under the inversion (L3/L6).
fn apply_player_actions(host: &mut dyn mod_api::ServerHost, uuid: u128, actions: Vec<entity_events_v1::EntityAction>) {
    for action in actions {
        if let entity_events_v1::EntityAction::AddEffect { effect, duration, amplifier, .. } = action {
            if let Some(player) = host.player(uuid) {
                if let Some(living) = host.living(player.entity()) {
                    if let Some(key) = mod_api::__private::effect_key(&effect) {
                        living.add_effect(host, &mod_api::ActiveEffect::new(key, duration, amplifier, false, true, true));
                    }
                }
            }
        }
    }
}

pub(crate) fn living_fall(event: &mut mod_api::LivingFallEvent<'_>) -> mod_api::Flow {
    if !living_fall_v1::fall_heard() {
        return mod_api::Flow::Continue;
    }
    let fall = living_fall_v1::LivingFall {
        entity: event.entity,
        kind: event.kind.to_owned(),
        distance: std::cell::Cell::new(event.distance),
        multiplier: std::cell::Cell::new(event.multiplier),
    };
    let allowed = living_fall_v1::fall(&fall);
    event.distance = fall.distance.get();
    event.multiplier = fall.multiplier.get();
    cancel_if(!allowed)
}

pub(crate) fn living_jump(event: &mut mod_api::LivingJumpEvent<'_>) {
    entity_events_v1::living_jump(&entity_events_v1::LivingJump { entity: event.entity as i32, kind: event.kind.to_owned() });
}

pub(crate) fn living_death(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::LivingDeathEvent<'_>) -> mod_api::Flow {
    let damage = entity_events_v1::LivingDamage {
        entity: event.entity as i32,
        kind: event.kind.to_owned(),
        damage_type: event.source.damage_type.to_owned(),
        attacker: if event.source.causing == 0 { -1 } else { event.source.causing },
        direct: -1,
        amount: 0.0,
    };
    scoped(host, || entity_events_v1::after_death(&damage));
    mod_api::Flow::Continue
}

pub(crate) fn living_damage_pre(event: &mut mod_api::LivingDamagePreEvent<'_>) {
    // The killing blow's direct entity, remembered for the experience report
    // that follows in the same call.
    entity_events_v1::note_killing_direct(event.entity, event.source.direct);
    let damage = entity_events_v1::LivingDamage {
        entity: event.entity,
        kind: event.kind.to_owned(),
        damage_type: event.source.damage_type.to_owned(),
        attacker: event.source.causing,
        direct: event.source.direct,
        amount: event.container.new_damage(),
    };
    let amount = entity_events_v1::modify_damage(&damage);
    let _ = event.container.set_new_damage(amount);
}

pub(crate) fn living_incoming_damage(host: &dyn mod_api::HitHost, event: &mut mod_api::LivingIncomingDamageEvent<'_>) -> mod_api::Flow {
    // The ender dragon's body immunity reads the victim's live health (and,
    // in the woven bridge, its attack damage) at the incoming stage: the
    // note is taken here, where the hit context names the victim, and the
    // pure damage-pre stage of this same hit consumes it (the
    // `pluto-damage-dragon` replacement). The victim's attack damage has no
    // reader in the hit context, so the fact carries health only.
    if event.kind == "minecraft:ender_dragon" {
        if let Some(victim) = host.victim() {
            entity_events_v1::note_victim_facts(
                victim.entity().id() as i32,
                entity_events_v1::VictimFacts { health: victim.health().unwrap_or(0.0), attack_damage: None },
            );
        }
    }
    let damage = entity_events_v1::LivingDamage {
        entity: event.entity,
        kind: event.kind.to_owned(),
        damage_type: event.source.damage_type.to_owned(),
        attacker: event.source.causing,
        direct: event.source.direct,
        amount: event.container.new_damage(),
    };
    if !entity_events_v1::allow_damage(&damage) {
        event.cancelled = true;
        return mod_api::Flow::Cancel;
    }
    // The hurt stage, as the damage arrives and before armor and enchantment
    // protection take their share.
    let amount = entity_events_v1::modify_hurt(&damage);
    let _ = event.container.set_new_damage(amount);
    // The wearer-hit report resolves here, as in the Horizon bridge; the effect
    // grants it computes were pushed into the game-side mob-effect module from
    // the old bridge and are the module side's to pull under the inversion
    // (L3/L6) — the pure resolution itself still runs, so no report is left
    // pending.
    let _grants = entity_events_v1::resolve_wearer_hit(&damage);
    mod_api::Flow::Continue
}

pub(crate) fn living_experience_drop(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::LivingExperienceDropEvent<'_>) -> mod_api::Flow {
    // The killer the death loot report in the same call named, and the killing
    // blow's direct entity (a projectile there names the stack it was launched
    // from), taken before the heard check so an unheard drop leaves no note.
    let noted_attacker = entity_events_v1::take_experience_attacker(event.entity as i32);
    let direct = entity_events_v1::take_killing_direct(event.entity as i32);
    let projectile_tool = direct.filter(|direct| *direct >= 0).and_then(|direct| u32::try_from(direct).ok()).and_then(entity_events_v1::take_projectile_tool);
    if !entity_events_v1::experience_drop_heard() {
        return mod_api::Flow::Continue;
    }
    // Sunrise names the attacking player on the event itself; the loot
    // report's note is the fallback for a report that names none. The dying
    // entity's own effects come through the entity-by-id reader (L3b).
    let effects: Vec<(String, i32)> = u32::try_from(event.entity)
        .ok()
        .and_then(|id| host.entity(id))
        .map(|entity| entity_effects(host, entity).into_iter().map(|(id, amplifier, _)| (id, amplifier)).collect())
        .unwrap_or_default();
    let attacker = event.attacking_player.or(noted_attacker);
    let player = attacker.and_then(|uuid| host.player(uuid).map(|player| (uuid, player.entity().id() as i32)));
    let looting_slot = attacker.and_then(entity_events_v1::take_looting_slot);
    let drop = entity_events_v1::ExperienceDrop {
        entity: event.entity as i32,
        kind: event.kind.to_owned(),
        player,
        effects,
        projectile_tool,
        looting_slot,
        experience: std::cell::Cell::new(event.amount),
    };
    event.amount = scoped(host, || entity_events_v1::experience_drop(&drop));
    mod_api::Flow::Continue
}

pub(crate) fn player_attack_cooldown_reset(event: &mut mod_api::PlayerAttackCooldownResetEvent<'_>) {
    // The swing the host reports after the attack resolved: the attack ticker
    // resets, so the next hit reads its ticks from a fresh swing.
    entity_events_v1::reset_attack_ticks(event.player);
}

pub(crate) fn player_shear_entity(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerShearEntityEvent<'_>) -> mod_api::Flow {
    let player = event.player;
    // The held stack the event does not carry: the off hand is inventory slot
    // 40, the main hand the selected slot.
    let held = host.player(player).and_then(|player_ref| {
        let slot = if event.off_hand { Some(40u32) } else { player_ref.selected_slot(host).map(|slot| slot as u32) };
        slot.and_then(|slot| player_ref.with_inventory(host, |inventory| inventory.get(slot).and_then(|view| view.project())))
    }).flatten();
    let attempt = entity_events_v1::ShearAttempt {
        player,
        name: event.name.to_owned(),
        entity: event.entity_id,
        off_hand: event.off_hand,
        tool: held.as_ref().map(read_stack),
        cancel: std::cell::Cell::new(false),
        actions: std::cell::RefCell::new(Vec::new()),
    };
    let (allowed, actions) = scoped(host, || entity_events_v1::shear(&attempt));
    if !actions.is_empty() {
        apply_player_actions(host, player, actions);
    }
    cancel_if(!allowed)
}

pub(crate) fn loot_drops(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::LootDropsEvent<'_, '_>) {
    let context = event.context;
    // The experience report that follows in the same call names no player in
    // the Horizon shape; the killer this context names is remembered for it.
    if let (Some((id, _)), Some(player)) = (context.entity, context.player) {
        entity_events_v1::note_experience_attacker(id as i32, player);
    }
    if !loot_api_v2::drops_heard() {
        return;
    }
    let stacks: Vec<mod_api::ItemStack> = event.drops.iter().collect();
    let Some(slots) = stacks.iter().map(slot_of).collect::<Option<Vec<ItemSlot>>>() else { return };
    let before = slots.clone();
    let drops = loot_api_v2::LootDrops {
        table: context.table.to_owned(),
        dimension: context.dimension.to_owned(),
        origin: context.origin,
        block: context.block.map(|(pos, _)| pos),
        entity: context.entity.map(|(id, kind)| (id, kind.to_owned())),
        // The entity facts the Horizon bridge read off the host (fire
        // immunity, invulnerability) have no reader from the context alone.
        entity_facts: None,
        player: context.player,
        tool: context.tool.map(read_stack),
        explosion: context.explosion,
        used: context.used,
        drops: std::cell::RefCell::new(slots),
        actions: std::cell::RefCell::new(Vec::new()),
    };
    let after = scoped(host, || loot_api_v2::modify_drops(&drops));
    let actions = drops.actions.take();
    if let (Some(player), false) = (drops.player, actions.is_empty()) {
        apply_player_actions(host, player, actions);
    }
    if after != before {
        event.drops.clear();
        for slot in after {
            if let Some(stack) = native_slot(Some(slot)) {
                let _ = event.drops.push(stack);
            }
        }
    }
}

pub(crate) fn menu_links(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::MenuLinksEvent<'_>) {
    let Some(block) = block_entity_api_v1::block_of_menu(event.menu) else { return };
    if screen_handler_api_v1::has_menu_links(&block) {
        let dimension = host.dimension().to_owned();
        let handle = ServerHandle::new(host);
        let named = world_api_v1::with_world(&mut HostWorld(handle), || {
            neighbour_api_v1::with_neighbours(&mut HostNeighbours { host: handle, dimension }, || screen_handler_api_v1::menu_links(&block, event.pos))
        });
        if let Some(positions) = named {
            event.links.extend(positions);
            return;
        }
    }
    for offset in screen_handler_api_v1::neighbour_sources(&block) {
        event.links.push([event.pos[0] + offset[0], event.pos[1] + offset[1], event.pos[2] + offset[2]]);
    }
}

pub(crate) fn left_click_block(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::LeftClickBlockEvent) -> mod_api::Flow {
    // The break hook is not told the face; the dig's start is, and a break
    // belongs to the dig the player last started.
    if let Some(player) = host.players().acting_player() {
        let face = match event.face {
            [0, -1, 0] => Some(block_api_v1::Direction::Down),
            [0, 1, 0] => Some(block_api_v1::Direction::Up),
            [0, 0, -1] => Some(block_api_v1::Direction::North),
            [0, 0, 1] => Some(block_api_v1::Direction::South),
            [-1, 0, 0] => Some(block_api_v1::Direction::West),
            [1, 0, 0] => Some(block_api_v1::Direction::East),
            _ => None,
        };
        if let (Some(face), Ok(mut last)) = (face, LAST_DIG_FACE.lock()) {
            last.insert(player.uuid(), face);
        }
    }
    mod_api::Flow::Continue
}

pub(crate) fn ingredient(event: &mut mod_api::IngredientEvent<'_>) -> Option<mod_api::IngredientDefinition> {
    let data = json_from_compound(&compound_from_tag(event.data));
    let items = recipe_api_v1::ingredient_items(event.kind, &data)?;
    let kind = event.kind.to_owned();
    // The Horizon pair (ingredient + ingredient_test) is one definition here:
    // the candidate items plus a test closing over the kind and its data. A
    // stack the pure test has no opinion on is accepted (it is on the list).
    Some(mod_api::IngredientDefinition::new(items, move |_context, stack| {
        let custom = match stack.stored_component("minecraft:custom_data") {
            Some(mod_api::Tag::Compound(compound)) => Some(Nbt::Compound(compound_from_tag(compound))),
            _ => None,
        };
        let view = recipe_api_v1::StackView { id: stack.item_id(), count: stack.count(), damage: stack.damage().unwrap_or(0), custom_data: custom.as_ref() };
        recipe_api_v1::ingredient_test(&kind, &data, &view).unwrap_or(true)
    }))
}

pub(crate) fn player_destroy_item(event: &mut mod_api::PlayerDestroyItemEvent<'_>) {
    if !player_api::item_break_heard() {
        return;
    }
    // The event names only the hand; armor breaks are not reported in this
    // shape. The player's name comes from the login table (the hook carries
    // no host to ask).
    let Some(slot) = (match event.hand {
        Some(mod_api::Hand::Main) => Some(entity_events_v1::StackSlot::MainHand),
        Some(mod_api::Hand::Off) => Some(entity_events_v1::StackSlot::OffHand),
        None => None,
    }) else {
        return;
    };
    let name = LOGGED_IN.lock().ok().and_then(|players| players.iter().find(|(uuid, _)| *uuid == event.player).map(|(_, name)| name.clone())).unwrap_or_default();
    let broken = player_api::ItemBreak {
        player: player_api::ServerPlayer { uuid: event.player, name },
        item: stack_item_name(event.original),
        slot,
    };
    player_api::item_break(&broken);
}

pub(crate) fn level_tick_pre(event: &mut mod_api::LevelTickPreEvent) {
    lifecycle_events_v1::world_tick(true, event.level.dimension_id());
    // The level's build range, which the Horizon body published here, is
    // published from `chunk_load` in Sunrise: this hook carries no host to
    // measure the level with.
}

pub(crate) fn level_tick_post(event: &mut mod_api::LevelTickPostEvent) {
    lifecycle_events_v1::world_tick(false, event.level.dimension_id());
    // The drop actions damage callbacks queued are handed to the game-side
    // damage-facts module, which pulls them under the inversion (L3/L6); the
    // queue is deliberately not drained here.
}

pub(crate) fn living_knockback(event: &mut mod_api::LivingKnockbackEvent<'_>) -> mod_api::Flow {
    let push = entity_events_v1::KnockbackModification {
        entity: event.entity,
        kind: event.kind.to_owned(),
        attacker: event.attacker,
        attacker_main: event.attacker_main.take().and_then(|view| view.project()).map(|stack| read_stack(&stack)),
        attacker_off: event.attacker_off.take().and_then(|view| view.project()).map(|stack| read_stack(&stack)),
        strength: std::cell::Cell::new(event.strength as f64),
        x: std::cell::Cell::new(event.x),
        z: std::cell::Cell::new(event.z),
        cancel: std::cell::Cell::new(false),
    };
    match entity_events_v1::modify_knockback(&push) {
        Some((strength, x, z)) => {
            event.strength = strength as f32;
            event.x = x;
            event.z = z;
            mod_api::Flow::Continue
        }
        None => mod_api::Flow::Cancel,
    }
}

pub(crate) fn player_fish(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerFishEvent<'_>) -> mod_api::Flow {
    if !events_interaction_v0::fishing_heard() {
        return mod_api::Flow::Continue;
    }
    // The event names no hand: the rod is the main hand's stack (the selected
    // slot), and the context reports the main hand.
    let rod = host.player(event.player).and_then(|player_ref| {
        let slot = player_ref.selected_slot(host)?;
        player_ref.with_inventory(host, |inventory| inventory.get(slot as u32).and_then(|view| view.project()))
    }).flatten();
    let context = events_interaction_v0::FishingContext {
        player: player_api::ServerPlayer { uuid: event.player, name: event.name.to_owned() },
        state: event.state,
        hand: events_interaction_v0::Hand::Main,
        rod: rod.as_ref().map(read_stack),
        cancel: std::cell::Cell::new(false),
        actions: std::cell::RefCell::new(Vec::new()),
    };
    scoped(host, || events_interaction_v0::fishing(&context));
    let actions = context.actions.take();
    if !actions.is_empty() {
        apply_player_actions(host, event.player, actions);
    }
    cancel_if(context.cancel.get())
}

pub(crate) fn register_layer_definitions(event: &mut mod_api::RegisterLayerDefinitionsEvent<'_>) {
    match block_entity_renderer_api_v1::model_layer_definitions() {
        Ok(definitions) => {
            for (id, layer) in definitions {
                event.register(id, move || layer_to_native(&layer));
            }
        }
        Err(error) => eprintln!("pluto_lib: model layer collection rejected: {error}"),
    }
}

/// One command argument as the word the pure matcher expects: text kinds are
/// exact, numbers and booleans render as typed, a player renders as their
/// name and an entity as its id. Selector and position kinds have no single
/// word and render empty (the pure usages are word-shaped).
fn command_argument_text(host: &dyn mod_api::ServerHost, value: &mod_api::CommandArgumentValue) -> String {
    match value {
        mod_api::CommandArgumentValue::String(text) => text.clone(),
        mod_api::CommandArgumentValue::Integer(value) => value.to_string(),
        mod_api::CommandArgumentValue::Long(value) => value.to_string(),
        mod_api::CommandArgumentValue::Number(value) => value.to_string(),
        mod_api::CommandArgumentValue::Boolean(value) => value.to_string(),
        mod_api::CommandArgumentValue::Player(player) => player.name(host).unwrap_or_default().to_string(),
        mod_api::CommandArgumentValue::Entity(entity) => entity.id().to_string(),
        mod_api::CommandArgumentValue::Item(stack) => stack_item_name(stack),
        _ => String::new(),
    }
}

pub(crate) fn register_commands(event: &mut mod_api::RegisterCommandsEvent<'_>) {
    for (usage, level) in command_api_v2::collect() {
        let command = if level == 0 { mod_api::Command::new(&usage) } else { mod_api::Command::op(&usage, level) };
        event.commands.push(command.executes(|host, source, invocation| {
            // The pure handlers are player commands: their context is a
            // player, and their feedback runs through the player scope.
            let mod_api::CommandSource::Player { player, .. } = source else {
                return Err(Box::new(mod_api::Text::literal("pluto_lib: this command is for players")));
            };
            let mut words: Vec<String> = invocation.literals.clone();
            for argument in &invocation.arguments {
                words.push(command_argument_text(host, &argument.value));
            }
            let line = words.join(" ");
            let (uuid, name) = (player.uuid(), player.name(host).unwrap_or_default().to_string());
            if scoped(host, || command_api_v2::execute(uuid, name, &line)) {
                Ok(1)
            } else {
                Err(Box::new(mod_api::Text::literal("pluto_lib: unknown command")))
            }
        }));
    }
}

pub(crate) fn input_key(event: &mut mod_api::InputKeyEvent<'_>) {
    if !player_api::key_heard() {
        return;
    }
    // The Horizon body named the acting player and ran under the host scopes;
    // the client host adapters are L6's, so the press is reported without a
    // player (a value the Horizon shape also produced when nobody was acting).
    let press = player_api::KeyPress { player: None, key: event.key.to_owned(), pressed: !matches!(event.action, mod_api::KeyAction::Release) };
    player_api::key_press(&press);
}

pub(crate) fn movement_input_update(event: &mut mod_api::MovementInputUpdateEvent<'_>) {
    // The local player's movement input, after the game read it and before it
    // applies it. The Horizon body also named the acting entity and the stack
    // being used; the client host adapters are L6's, so the input runs with
    // no entity and no stack here.
    let input = entity_events_v1::MovementInput {
        entity: -1,
        forward: std::cell::Cell::new(event.input.move_vector[1]),
        strafe: std::cell::Cell::new(event.input.move_vector[0]),
        jump: std::cell::Cell::new(event.input.key_presses.jump),
        sneak: std::cell::Cell::new(event.input.key_presses.sneak),
        using_item: false,
        using: None,
    };
    entity_events_v1::movement_input(&input);
    event.input.move_vector[1] = input.forward.get();
    event.input.move_vector[0] = input.strafe.get();
    event.input.key_presses.jump = input.jump.get();
    event.input.key_presses.sneak = input.sneak.get();
}

pub(crate) fn client_player_logging_in() {
    // The Horizon body ran the pure call under the host scopes; the client
    // host adapters are L6's.
    networking_api_v1::client_joined();
}

pub(crate) fn client_setup() {
    // The Horizon client-start body also refreshed the item prototypes from
    // the game's registry; that registry read has no mod_api counterpart.
    // The pure half — the resource reload listeners — runs here as on reload.
    resource_loader_v0::client_resources_reloaded();
}

/// The block whose menu a screen event names, when that menu is a mod menu
/// with a window: every container screen reports the id `inventory` and is
/// told apart by its menu alone.
fn menu_block(screen: &mod_api::ScreenView<'_>) -> Option<String> {
    if screen.id != "inventory" {
        return None;
    }
    block_entity_api_v1::block_of_menu(screen.menu?)
}

/// Whether a screen is this mod's own book screen (the view itself names the
/// owning mod in Sunrise, where the Horizon body asked the host).
fn book_screen_is_ours(screen: &mod_api::ScreenView<'_>) -> bool {
    screen.id == "mod" && screen.mod_screen == Some(PLUTO_MOD_ID)
}

/// A screen event's cursor position from its window's top left (the event's
/// `rect` is the container panel itself, so this is the same space the draw
/// callback draws in).
fn screen_at(screen: &mod_api::ScreenView<'_>) -> [f32; 2] {
    [screen.mouse[0] - screen.rect[0], screen.mouse[1] - screen.rect[1]]
}

pub(crate) fn screen_key_pressed_pre(event: &mut mod_api::ScreenKeyPressedPreEvent<'_>) -> mod_api::Flow {
    // A mod menu with a text field hears key presses (backspace, enter, ...)
    // while open.
    if let Some(block) = menu_block(event.screen) {
        return if screen_handler_api_v1::has_text_input(&block) && screen_handler_api_v1::key_pressed(&block, &event.key.to_ascii_lowercase()) {
            mod_api::Flow::Cancel
        } else {
            mod_api::Flow::Continue
        };
    }
    if !book_screen_is_ours(event.screen) {
        return mod_api::Flow::Continue;
    }
    let Some((book, mut view)) = open_book_view() else { return mod_api::Flow::Continue };
    let key = event.key.to_ascii_lowercase();
    let turned = match key.as_str() {
        "arrowright" | "right" | "pagedown" | "space" => view.next_page(&book),
        "arrowleft" | "left" | "pageup" => view.prev_page(&book),
        "home" => {
            let moved = view.page != 0;
            view.page = 0;
            moved
        }
        "end" => {
            let last = book.page_count().saturating_sub(1);
            let moved = view.page != last;
            view.page = last;
            moved
        }
        _ => return mod_api::Flow::Continue,
    };
    if turned {
        set_book_view(view);
    }
    mod_api::Flow::Cancel
}

pub(crate) fn screen_char_typed_pre(event: &mut mod_api::ScreenCharTypedPreEvent<'_>) -> mod_api::Flow {
    // A mod menu with a text field hears the typed characters while it is open.
    if let Some(block) = menu_block(event.screen) {
        return if screen_handler_api_v1::has_text_input(&block) && screen_handler_api_v1::char_typed(&block, event.c) {
            mod_api::Flow::Cancel
        } else {
            mod_api::Flow::Continue
        };
    }
    mod_api::Flow::Continue
}

pub(crate) fn screen_mouse_button_pressed_pre(event: &mut mod_api::ScreenMouseButtonPressedPreEvent<'_>) -> mod_api::Flow {
    if event.button != 0 || !book_screen_is_ours(event.screen) {
        return mod_api::Flow::Continue;
    }
    let Some((book, mut view)) = open_book_view() else { return mod_api::Flow::Continue };
    let turned = match book_api_v1::book_click(&book, &view, event.screen.rect, event.screen.mouse) {
        book_api_v1::BookClick::NextPage => view.next_page(&book),
        book_api_v1::BookClick::PrevPage => view.prev_page(&book),
        book_api_v1::BookClick::Section(section) => view.goto_section(&book, section),
        book_api_v1::BookClick::Page(page) => {
            let moved = page < book.page_count() && view.page != page;
            if moved {
                view.page = page;
            }
            moved
        }
        book_api_v1::BookClick::None => return mod_api::Flow::Continue,
    };
    if turned {
        set_book_view(view);
    }
    mod_api::Flow::Cancel
}

pub(crate) fn screen_mouse_scrolled_pre(event: &mut mod_api::ScreenMouseScrolledPreEvent<'_>) -> mod_api::Flow {
    // A mod menu that hears the wheel gets the roll first, over its own
    // scrolling state.
    if let Some(block) = menu_block(event.screen) {
        if screen_handler_api_v1::has_scroll(&block) && screen_handler_api_v1::scroll(&block, screen_at(event.screen), [event.scroll[0] as f32, event.scroll[1] as f32]) {
            return mod_api::Flow::Cancel;
        }
        return mod_api::Flow::Continue;
    }
    if !book_screen_is_ours(event.screen) {
        return mod_api::Flow::Continue;
    }
    let Some((book, mut view)) = open_book_view() else { return mod_api::Flow::Continue };
    // A negative vertical (the wheel rolled down) turns forward, a positive
    // one back.
    let turned = if event.scroll[1] < 0.0 {
        view.next_page(&book)
    } else if event.scroll[1] > 0.0 {
        view.prev_page(&book)
    } else {
        false
    };
    if turned {
        set_book_view(view);
    }
    mod_api::Flow::Cancel
}

pub(crate) fn screen_closing(event: &mut mod_api::ScreenClosingEvent<'_>) {
    // A closing container screen forgets the client's view of its menu's
    // state, so a later menu of the same block never draws with the earlier
    // one's items.
    if let Some(block) = menu_block(event.screen) {
        screen_handler_api_v1::forget_menu_state(&block);
    }
    if event.screen.id != "mod" {
        return;
    }
    // The reader closed the book (Escape, or the game moved on): forget where
    // they were. While the screen closes the game may already report no mod
    // screen, so a held view with no other owner named is treated as ours.
    if event.screen.mod_screen.is_none() || event.screen.mod_screen == Some(PLUTO_MOD_ID) {
        if let Ok(mut view) = BOOK_VIEW.lock() {
            *view = None;
        }
    }
}

pub(crate) fn player_logged_in(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerLoggedInEvent<'_>) {
    let (uuid, name) = (event.player, event.name.to_string());
    remember_player(uuid, &name, true);
    scoped(host, || networking_api_v1::player_joined(uuid, name));
}

pub(crate) fn player_logged_out(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerLoggedOutEvent<'_>) {
    let (uuid, name) = (event.player, event.name.to_string());
    remember_player(uuid, &name, false);
    entity_events_v1::forget_armor(uuid);
    entity_events_v1::forget_aim(uuid);
    entity_events_v1::forget_attack_ticks(uuid);
    // The base values are saved with the player, so what the armor added is taken off before they go (best effort: the host may no longer reach them).
    let living = host.player(uuid).and_then(|player| host.living(player.entity()));
    if let Some(living) = living {
        for (attribute, held) in entity_events_v1::forget_attributes(uuid) {
            if let Some(base) = living.attribute_base(host, &attribute) {
                living.set_attribute_base(host, &attribute, base - held);
            }
        }
        // A folded multiply-total payload is unfolded after the additions are taken off: the additions sat on top of the folded base,
        // so what is left is the folded base itself, which divides back to the plain base.
        for (attribute, factor) in entity_events_v1::forget_multiply_factors(uuid) {
            if factor != 1.0 {
                if let Some(base) = living.attribute_base(host, &attribute) {
                    living.set_attribute_base(host, &attribute, base / factor);
                }
            }
        }
    }
    scoped(host, || networking_api_v1::player_left(uuid, name));
}

pub(crate) fn server_about_to_start(event: &mut mod_api::ServerAboutToStartEvent) {
    lifecycle_events_v1::server_starting(event.dedicated);
}

pub(crate) fn server_stopping(event: &mut mod_api::ServerStoppingEvent) {
    lifecycle_events_v1::server_stopping(event.dedicated);
}

pub(crate) fn server_tick(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::ServerTickEvent<'_>) {
    scoped(host, || match event.phase {
        mod_api::ServerTickPhase::ServerTickStart { .. } => {
            crate::pure::i18n_api::poll_language();
            lifecycle_events_v1::server_tick(true)
        }
        mod_api::ServerTickPhase::ServerTickEnd { .. } => lifecycle_events_v1::server_tick(false),
        #[allow(unreachable_patterns)]
        _ => {}
    });
}

pub(crate) fn chunk_load(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::ChunkLoadEvent<'_>) {
    // The level-tick hook carries no host in Sunrise, so the level's build
    // range is published here, on a host-carrying hook of the same level, for
    // the readers that run outside a world callback.
    if let Some(level) = host.level(event.dimension) {
        if let Some((min, max)) = level.height(host) {
            world_api_v1::publish_level_height(event.dimension, [min, max]);
        }
    }
    lifecycle_events_v1::chunk_loaded(event.dimension, event.chunk, event.new);
}

pub(crate) fn chunk_unload(event: &mut mod_api::ChunkUnloadEvent<'_>) {
    lifecycle_events_v1::chunk_unloaded(event.dimension, event.chunk);
}

pub(crate) fn entity_join_level(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::EntityJoinLevelEvent<'_>) -> mod_api::Flow {
    let id = event.subject.id();
    scoped(host, || entity_events_v1::entity_loaded(id, event.kind, event.dimension));
    mod_api::Flow::Continue
}

pub(crate) fn entity_leave_level(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::EntityLeaveLevelEvent<'_>) {
    // The climb-access forget the Horizon bridge called here is a woven module's own state;
    // under the inverted bridge that module clears it itself (see the disposition table).
    scoped(host, || entity_events_v1::entity_unloaded(event.entity, event.kind, event.dimension));
}

pub(crate) fn attack_entity(event: &mut mod_api::AttackEntityEvent<'_>) -> mod_api::Flow {
    // Sunrise's attack hook names the acting player only through the host's acting player
    // (see the hook's doc), which this body does not need: the pure event starts with no
    // held stack, as the Horizon's ambient read has no equivalent in the event itself.
    entity_events_v1::attack_started(event.entity, None);
    cancel_if(events_interaction_v0::attack_entity(event.entity, event.kind, event.pos))
}


pub(crate) fn right_click_block(event: &mut mod_api::RightClickBlockEvent<'_>) -> mod_api::Flow {
    cancel_if(events_interaction_v0::use_block(event.click.pos, event.click.hit.face))
}

/// The hook-flow answer for a handled interaction: cancel when a PlutoLib callback claimed it.
pub(crate) fn cancel_if(handled: bool) -> mod_api::Flow {
    if handled { mod_api::Flow::Cancel } else { mod_api::Flow::Continue }
}

pub(crate) fn loot_table_load(event: &mut mod_api::LootTableLoadEvent<'_>) {
    let edited = loot_api_v2::modify(event.id, event.table.clone());
    *event.table = edited;
}




pub(crate) fn client_resources_reloaded() {
    // The prototype refresh the Horizon bridge ran here read the host's resource registries;
    // in Sunrise that refresh belongs to the client reload listeners (add_client_reload_listeners).
    resource_loader_v0::client_resources_reloaded();
}



// ---- L3b: the entity-by-id reader, the entity scope, and the adapters ----
// ---- for the spawn / damage / interaction hooks that name entities by id ----

/// A pure stack as a native stack (the `give_stack` conversion, shared).
fn native_stack(data: &player_api::StackData) -> Option<ItemStack> {
    let key = mod_api::__private::item_key(&data.item)?;
    let mut native = ItemStack::new(key, data.count.max(1))?;
    let mut slot = ItemSlot::new(Identifier::parse(&data.item).ok()?, data.count);
    slot.damage = data.damage;
    slot.max_damage = data.max_damage;
    slot.custom_data = data.custom_data.clone();
    slot.custom_model_data = data.custom_model_data.clone();
    slot.enchantments = Some(data.enchantments.clone());
    write_slot(&mut native, &slot);
    Some(native)
}

/// The type id of an entity, through its kind key's canonical name.
fn entity_kind_id(host: &dyn mod_api::Host, entity: mod_api::EntityRef) -> Option<Identifier> {
    let key = entity.kind(host)?;
    let name = mod_api::__private::entity_type_key_id(key)?;
    Identifier::parse(name.as_str()).ok()
}

/// The effects a living entity carries, as (id, amplifier, ticks left): the
/// entity-by-id reader the experience drop and the projectile impact read
/// through (`Host::entity` resolves the id an event carries, in place).
fn entity_effects(host: &dyn mod_api::Host, entity: mod_api::EntityRef) -> Vec<(String, i32, i32)> {
    let Some(living) = host.living(entity) else { return Vec::new() };
    let mut found = Vec::new();
    let _ = living.visit_effects(host, &mut |effect| {
        found.push(effect);
        core::ops::ControlFlow::Continue(())
    });
    found
        .into_iter()
        .map(|effect| {
            (
                mod_api::__private::effect_key_id(effect.id).unwrap_or_default().to_owned(),
                effect.amplifier,
                effect.duration,
            )
        })
        .collect()
}

/// Runs `f` with the player, world, book and entity scopes all open over the
/// host: the entity scope is what the pure entity events (ticks, generic
/// interactions, impacts) read and write through while their callbacks run.
pub(crate) fn scoped_entities<R>(
    host: &mut dyn mod_api::ServerHost,
    dimension: &str,
    f: impl FnOnce() -> R,
) -> R {
    let mut handle = ServerHandle::new(&mut *host);
    scoped(&mut *host, || {
        entity_api_v1::with_entities(
            &mut HostEntities { host: handle, dimension: dimension.to_owned() },
            f,
        )
    })
}

impl EntityOps for HostEntities {
    fn entities_in(&mut self, min: [f64; 3], max: [f64; 3]) -> Vec<entity_api_v1::WorldEntity> {
        let host = self.host.get();
        let Some(level) = host.level(&self.dimension) else { return Vec::new() };
        let mut refs = Vec::new();
        let _ = level.visit_entities_in(host, min, max, &mut |entity| {
            refs.push(entity);
            core::ops::ControlFlow::Continue(())
        });
        let mut found = Vec::new();
        for entity in refs {
            let Some(kind) = entity_kind_id(host, entity) else { continue };
            if let Some(item) = host.item_entity(entity) {
                let stack = item.with_stack(host, |view| view.project()).flatten();
                found.push(entity_api_v1::WorldEntity {
                    id: u64::from(entity.id()),
                    kind,
                    item: stack.and_then(|stack| slot_of(&stack)),
                    living: None,
                });
                continue;
            }
            if host.living(entity).is_some() {
                let effects = entity_effects(host, entity);
                found.push(entity_api_v1::WorldEntity {
                    id: u64::from(entity.id()),
                    kind,
                    item: None,
                    living: Some(entity_api_v1::LivingFacts {
                        fire_immune: entity.fire_immune(host).unwrap_or(false),
                        invulnerable: entity.permanently_invulnerable(host).unwrap_or(false),
                        fire_resistant: effects.iter().any(|(id, _, _)| is_fire_resistance(id)),
                    }),
                });
            }
        }
        found
    }

    fn discard(&mut self, id: u64) -> bool {
        let host = self.host.get();
        let Ok(id) = u32::try_from(id) else { return false };
        let Some(entity) = host.entity(id) else { return false };
        entity.discard(host)
    }

    fn set_item(&mut self, id: u64, stack: &ItemSlot) -> bool {
        let host = self.host.get();
        let Ok(id) = u32::try_from(id) else { return false };
        let Some(entity) = host.entity(id) else { return false };
        let Some(item) = host.item_entity(entity) else { return false };
        let data = player_api::StackData {
            item: stack.item.to_string(),
            count: stack.count,
            damage: stack.damage,
            max_damage: stack.max_damage,
            custom_data: stack.custom_data.clone(),
            custom_model_data: stack.custom_model_data.clone(),
            enchantments: stack.enchantments.clone().unwrap_or_default(),
        };
        let Some(native) = native_stack(&data) else { return false };
        item.set_stack(host, Some(native)).unwrap_or(false)
    }

    fn hurt(&mut self, id: u64, amount: f32, damage_type: &str) -> bool {
        let host = self.host.get();
        let Ok(id) = u32::try_from(id) else { return false };
        let Some(entity) = host.entity(id) else { return false };
        entity.hurt(host, damage_type, amount, None, None).unwrap_or(false)
    }

    fn position_of(&mut self, id: u64) -> Option<[f64; 3]> {
        let host = self.host.get();
        host.entity(u32::try_from(id).ok()?).and_then(|entity| entity.position(host))
    }

    fn effects_of(&mut self, id: u64) -> Vec<(String, i32, i32)> {
        let host = self.host.get();
        let Some(entity) = u32::try_from(id).ok().and_then(|id| host.entity(id)) else { return Vec::new() };
        entity_effects(host, entity)
    }

    fn velocity_of(&mut self, id: u64) -> Option<[f64; 3]> {
        let host = self.host.get();
        host.entity(u32::try_from(id).ok()?).and_then(|entity| entity.velocity(host))
    }

    fn bounds_of(&mut self, id: u64) -> Option<[[f64; 3]; 2]> {
        let host = self.host.get();
        host.entity(u32::try_from(id).ok()?).and_then(|entity| entity.bounding_box(host))
    }

    fn bounding_box_of(&mut self, id: u64) -> Option<[[f64; 3]; 2]> {
        self.bounds_of(id)
    }

    fn set_velocity(&mut self, id: u64, velocity: [f64; 3]) -> bool {
        let host = self.host.get();
        let Ok(id) = u32::try_from(id) else { return false };
        let Some(entity) = host.entity(id) else { return false };
        entity.set_velocity(host, velocity)
    }

    fn teleport(&mut self, id: u64, pos: [f64; 3]) -> bool {
        let host = self.host.get();
        let Ok(id) = u32::try_from(id) else { return false };
        let Some(entity) = host.entity(id) else { return false };
        entity.teleport_to(host, pos, None)
    }

    fn passengers_of(&mut self, vehicle: u64) -> Vec<u64> {
        let host = self.host.get();
        let Some(entity) = u32::try_from(vehicle).ok().and_then(|id| host.entity(id)) else { return Vec::new() };
        let mut found = Vec::new();
        let _ = entity.visit_passengers(host, &mut |passenger| {
            found.push(u64::from(passenger.id()));
            core::ops::ControlFlow::Continue(())
        });
        found
    }

    fn vehicle_of(&mut self, passenger: u64) -> Option<u64> {
        let host = self.host.get();
        host.entity(u32::try_from(passenger).ok()?)
            .and_then(|entity| entity.vehicle(host))
            .map(|vehicle| u64::from(vehicle.id()))
    }

    fn start_riding(&mut self, passenger: u64, vehicle: u64) -> bool {
        let host = self.host.get();
        let (Ok(passenger), Ok(vehicle)) = (u32::try_from(passenger), u32::try_from(vehicle)) else { return false };
        let (Some(passenger), Some(vehicle)) = (host.entity(passenger), host.entity(vehicle)) else { return false };
        passenger.start_riding(host, vehicle, false).is_ok()
    }

    fn stop_riding(&mut self, passenger: u64) -> bool {
        let host = self.host.get();
        let Ok(id) = u32::try_from(passenger) else { return false };
        let Some(entity) = host.entity(id) else { return false };
        entity.stop_riding(host)
    }

    fn id_of_uuid(&mut self, uuid: u128) -> Option<u64> {
        let host = self.host.get();
        host.player(uuid).map(|player| u64::from(player.entity().id()))
    }

    fn kind_of(&mut self, id: u64) -> Option<Identifier> {
        let host = self.host.get();
        let entity = host.entity(u32::try_from(id).ok()?)?;
        entity_kind_id(host, entity)
    }

    fn knockback(&mut self, id: u64, strength: f64, x: f64, z: f64) -> bool {
        let host = self.host.get();
        let Ok(id) = u32::try_from(id) else { return false };
        let Some(entity) = host.entity(id) else { return false };
        let Some(living) = host.living(entity) else { return false };
        living.knockback(host, strength, [x, z])
    }

    fn push(&mut self, id: u64, delta: [f64; 3]) -> bool {
        let host = self.host.get();
        let Ok(id) = u32::try_from(id) else { return false };
        let Some(entity) = host.entity(id) else { return false };
        entity.push(host, delta)
    }

    fn add_effect(&mut self, id: u64, effect: &str, duration: i32, amplifier: u8) -> bool {
        let host = self.host.get();
        let Ok(id) = u32::try_from(id) else { return false };
        let Some(entity) = host.entity(id) else { return false };
        let Some(living) = host.living(entity) else { return false };
        let Some(key) = mod_api::__private::effect_key(effect) else { return false };
        living.add_effect(host, &mod_api::ActiveEffect::new(key, duration, i32::from(amplifier), false, true, true))
    }
}

/// The fancy armor stands' tick pass (the `pluto-hanging-stand-tick-head`
/// replacement, with its after-pass): a stand with a registered variant is
/// fire immune at the class level, so its fire ticks are cleared before its
/// tick and again after it, and the clear stand's invisibility follows its
/// equipment. Runs per stand from the entity tick events, which is the same
/// set the woven pass walked in the game's stand list.
fn fancy_stand_tick(host: &mut dyn mod_api::ServerHost, entity: u32, kind: &str, after: bool) {
    if kind != "minecraft:armor_stand" {
        return;
    }
    let Some(entity_ref) = host.entity(entity) else { return };
    let Some(nbt) = entity_ref.nbt(host) else { return };
    let nbt = compound_from_tag(&nbt);
    let variant = match nbt.get("Variant") {
        Some(Nbt::Int(value)) => *value,
        Some(Nbt::Byte(value)) => i32::from(*value),
        _ => return,
    };
    let Some(facts) = HangingRegistry::stand_for_variant(variant) else { return };
    let _ = entity_ref.clear_fire(host);
    if after {
        let equipped = host.living(entity_ref).is_some_and(|living| {
            mod_api::EquipmentSlot::ALL.iter().any(|slot| {
                living.with_equipment(host, *slot, |view| view.project()).is_some()
            })
        });
        let _ = entity_ref.set_invisible(host, facts.hidden(equipped));
    }
}

pub(crate) fn entity_tick_pre(host: mod_api::HostSide, event: &mut mod_api::EntityTickPreEvent<'_>) -> mod_api::Flow {
    let mod_api::HostSide::Server(host) = host else { return mod_api::Flow::Continue };
    fancy_stand_tick(host, event.entity, event.kind, false);
    if !entity_events_v1::entity_tick_heard() {
        return mod_api::Flow::Continue;
    }
    let info = entity_events_v1::EntityInfo { id: event.entity, kind: event.kind.to_owned(), dimension: event.dimension.to_owned() };
    let allowed = scoped_entities(host, event.dimension, || entity_events_v1::entity_tick_before(&info));
    if !allowed {
        event.cancelled = true;
        return mod_api::Flow::Cancel;
    }
    mod_api::Flow::Continue
}

pub(crate) fn entity_tick_post(host: mod_api::HostSide, event: &mut mod_api::EntityTickPostEvent<'_>) {
    let mod_api::HostSide::Server(host) = host else { return };
    fancy_stand_tick(host, event.entity, event.kind, true);
    if !entity_events_v1::entity_tick_heard() {
        return;
    }
    let info = entity_events_v1::EntityInfo { id: event.entity, kind: event.kind.to_owned(), dimension: event.dimension.to_owned() };
    scoped_entities(host, event.dimension, || entity_events_v1::entity_tick_after(&info));
}

pub(crate) fn entity_interact_specific(
    host: &mut dyn mod_api::ServerHost,
    event: &mut mod_api::EntityInteractSpecificEvent<'_>,
) -> mod_api::Flow {
    if !events_interaction_v0::entity_interact_heard() {
        return mod_api::Flow::Continue;
    }
    let context = events_interaction_v0::InteractEntityContext {
        entity: event.entity,
        kind: event.kind.to_owned(),
        stack: event.stack.map(read_stack),
        player: None,
    };
    let dimension = u32::try_from(event.entity)
        .ok()
        .and_then(|id| host.entity(id))
        .and_then(|entity| entity.level(host))
        .map(|level| level.dimension_id().to_owned())
        .unwrap_or_default();
    let handled = scoped_entities(host, &dimension, || events_interaction_v0::entity_interact(&context));
    if handled {
        event.cancelled = true;
        return mod_api::Flow::Cancel;
    }
    mod_api::Flow::Continue
}

pub(crate) fn projectile_impact(
    host: &mut dyn mod_api::ServerHost,
    event: &mut mod_api::ProjectileImpactEvent<'_>,
) -> mod_api::Flow {
    if !projectile_events_v1::heard() {
        return mod_api::Flow::Continue;
    }
    let projectile_ref = host.entity(event.projectile);
    let motion = projectile_ref.and_then(|entity| entity.velocity(host)).unwrap_or([0.0; 3]);
    let data = projectile_ref
        .and_then(|entity| entity.nbt(host))
        .map(|nbt| compound_from_tag(&nbt))
        .unwrap_or_default();
    let entity_effects = event
        .entity
        .and_then(|id| host.entity(id))
        .map(|entity| entity_effects(host, entity).into_iter().map(|(id, _, _)| id).collect())
        .unwrap_or_default();
    let impact = ProjectileImpact {
        dimension: event.dimension.to_owned(),
        projectile: event.projectile,
        kind: event.kind.to_owned(),
        owner: if event.owner == 0 { -1 } else { i64::from(event.owner) },
        entity: event.entity,
        entity_kind: event.entity_kind.to_owned(),
        entity_effects,
        block: event.block,
        face: event.face,
        at: event.at,
        motion,
        data,
        // The arrow's pierced list has no reader in mod_api (only its pierce
        // level does); the woven bridge read the live arrow's own list.
        piercing_ignored: Vec::new(),
        discard: std::cell::Cell::new(false),
        motion_after: std::cell::RefCell::new(None),
        ignore_entities: std::cell::RefCell::new(Vec::new()),
        actions: std::cell::RefCell::new(Vec::new()),
    };
    let allowed = scoped_entities(host, event.dimension, || projectile_events_v1::impact(&impact));
    if impact.discard.get() {
        if let Some(entity) = projectile_ref {
            let _ = entity.discard(host);
        }
    } else if let Some(motion) = impact.motion_after.borrow_mut().take() {
        if let Some(entity) = projectile_ref {
            let _ = entity.set_velocity(host, motion);
        }
    }
    // The ignore-list write-back has no call in mod_api either; the effect
    // grants land through the same application the player actions use.
    let actions = std::mem::take(&mut *impact.actions.borrow_mut());
    apply_entity_actions(host, event.entity, event.owner, actions);
    if !allowed {
        event.cancelled = true;
        return mod_api::Flow::Cancel;
    }
    mod_api::Flow::Continue
}

/// Entity actions a callback queued against a hit's parties: an effect grant
/// to a living party lands directly, as in `apply_player_actions`; the rest
/// were handed to game-side module queues in the woven bridge and are the
/// module side's to pull under the inversion (L3/L6).
fn apply_entity_actions(
    host: &mut dyn mod_api::ServerHost,
    victim: Option<u32>,
    attacker: u32,
    actions: Vec<entity_events_v1::EntityAction>,
) {
    for action in actions {
        let entity_events_v1::EntityAction::AddEffect { target, effect, duration, amplifier } = action else { continue };
        let target = match target {
            entity_events_v1::ActionTarget::Victim => victim,
            entity_events_v1::ActionTarget::Attacker => (attacker != 0).then_some(attacker),
        };
        let Some(target) = target else { continue };
        if let Some(living) = host.entity(target).and_then(|entity| host.living(entity)) {
            if let Some(key) = mod_api::__private::effect_key(&effect) {
                let _ = living.add_effect(host, &mod_api::ActiveEffect::new(key, duration, amplifier, false, true, true));
            }
        }
    }
}


// ---- L3b: the mob split adapter and the server reload adapter (A6) ----

/// One recorded child dressing from a split callback, applied to the game's
/// pending child after the pass: the pure child surface is write-only, so the
/// writes are collected during `inherit` and replayed one editor at a time
/// (the game's editors borrow the child list per call), as the woven bridge
/// recorded its Dress entries.
enum SplitDress {
    Equipment(mod_api::EquipmentSlot, Option<ItemStack>),
    Name(Option<mod_api::Text>),
    // Recorded but not applicable: PendingChildEditor has no no-ai or
    // invulnerability write (the woven bridge wrote the game's own fields).
    // The pure defaults are false, so only a callback asking for true differs.
    NoAi(bool),
    Invulnerable(bool),
}

struct SplitChild {
    index: usize,
    info: entity_events_v1::EntityInfo,
    dresses: std::rc::Rc<std::cell::RefCell<Vec<(usize, SplitDress)>>>,
}

impl mob_split_api::MobSplitChild for SplitChild {
    fn info(&self) -> entity_events_v1::EntityInfo {
        self.info.clone()
    }

    fn set_equipment(&mut self, slot: &str, stack: player_api::StackData) -> Result<(), String> {
        let slot = match slot {
            "mainhand" | "main_hand" => mod_api::EquipmentSlot::MainHand,
            "offhand" | "off_hand" => mod_api::EquipmentSlot::OffHand,
            "feet" => mod_api::EquipmentSlot::Feet,
            "legs" => mod_api::EquipmentSlot::Legs,
            "chest" => mod_api::EquipmentSlot::Chest,
            "head" => mod_api::EquipmentSlot::Head,
            "body" => mod_api::EquipmentSlot::Body,
            "saddle" => mod_api::EquipmentSlot::Saddle,
            other => return Err(format!("unknown equipment slot {other}")),
        };
        let native = native_stack(&stack).ok_or_else(|| format!("unbuildable split stack {}", stack.item))?;
        self.dresses.borrow_mut().push((self.index, SplitDress::Equipment(slot, Some(native))));
        Ok(())
    }

    fn set_custom_name(&mut self, name: Option<String>) -> Result<(), String> {
        self.dresses.borrow_mut().push((self.index, SplitDress::Name(name.map(mod_api::Text::literal))));
        Ok(())
    }

    fn set_no_ai(&mut self, no_ai: bool) -> Result<(), String> {
        self.dresses.borrow_mut().push((self.index, SplitDress::NoAi(no_ai)));
        Ok(())
    }

    fn set_invulnerable(&mut self, invulnerable: bool) -> Result<(), String> {
        self.dresses.borrow_mut().push((self.index, SplitDress::Invulnerable(invulnerable)));
        Ok(())
    }
}

/// The split's random source: the parent's own stream is lent only inside a
/// hit context in mod_api, so the adapter seeds a mod-owned legacy source
/// from one draw of the level's stream — the same distribution the woven
/// bridge drew from its clone of the parent mob's generator.
struct SplitRandom {
    source: mod_api::LegacyRandomSource,
}

impl mob_split_api::MobSplitRandom for SplitRandom {
    fn next_float(&mut self) -> f32 {
        mod_api::RandomSource::next_float(&mut self.source)
    }

    fn next_int(&mut self, bound: i32) -> Result<i32, String> {
        if bound <= 0 {
            return Err("random bound must be positive".into());
        }
        Ok(mod_api::RandomSource::next_int_bound(&mut self.source, bound))
    }
}

/// A text's plain characters: literal content and its children, in order.
/// Name tags in the game are literal components; anything richer reads as
/// its literal parts only.
fn text_plain(text: &mod_api::Text) -> String {
    let mut out = String::new();
    if let mod_api::text::TextContent::Literal(literal) = &text.content {
        out.push_str(literal);
    }
    for child in &text.children {
        out.push_str(&text_plain(child));
    }
    out
}

pub(crate) fn mob_split(host: &mut dyn mod_api::ServerHost, event: &mut mod_api::MobSplitEvent<'_>) -> mod_api::Flow {
    if !mob_split_api::MobSplitCallbacks::has_callbacks() {
        return mod_api::Flow::Continue;
    }
    let Some(parent_ref) = host.entity(event.parent) else { return mod_api::Flow::Continue };
    let context = mob_split_api::MobSplitContext {
        parent: entity_events_v1::EntityInfo { id: event.parent, kind: event.kind.to_owned(), dimension: event.dimension.to_owned() },
        position: parent_ref.position(host).unwrap_or([0.0; 3]),
        size: event.size,
        child_size: event.child_size,
    };
    let mut count = u32::try_from(event.children.len()).unwrap_or(0);
    let mut cancel = false;
    if let Err(error) = mob_split_api::MobSplitCallbacks::prepare(&context, &mut count, &mut cancel) {
        eprintln!("pluto-lib: mob split prepare failed: {error}");
        return mod_api::Flow::Continue;
    }
    if cancel {
        event.cancelled = true;
        return mod_api::Flow::Cancel;
    }
    while event.children.len() > count as usize {
        let last = event.children.len() - 1;
        event.children.remove(last);
    }
    while event.children.len() < count as usize {
        let (Ok(kind), Ok(dimension)) = (mod_api::Identifier::parse(event.kind), mod_api::Identifier::parse(event.dimension)) else { break };
        let spec = mod_api::PendingMobSpec::new(kind, dimension, context.position, 0.0, false);
        match event.children.construct(host, spec) {
            Ok(child) => {
                if event.children.push(child).is_err() {
                    break;
                }
                if let Some(mut editor) = event.children.get_mut(event.children.len() - 1) {
                    let _ = editor.set_size(u32::try_from(event.child_size.max(0)).unwrap_or(0));
                }
            }
            Err(_) => break,
        }
    }
    let head = host.living(parent_ref).and_then(|living| {
        living.with_equipment(host, mod_api::EquipmentSlot::Head, |view| view.project()).flatten().map(|stack| read_stack(&stack))
    });
    let custom_name = parent_ref
        .custom_name(host)
        .ok()
        .flatten()
        .map(|name| text_plain(&name))
        .filter(|name| !name.is_empty());
    let no_ai = host.mob(parent_ref).and_then(|mob| mob.no_ai(host)).unwrap_or(false);
    let invulnerable = parent_ref.permanently_invulnerable(host).unwrap_or(false);
    let seed = host
        .level(event.dimension)
        .and_then(|level| level.with_random(host, |rng| mod_api::RandomSource::next_long(rng)))
        .unwrap_or(0);
    let mut random = SplitRandom { source: mod_api::LegacyRandomSource::new(seed) };
    let dresses = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut children: Vec<Box<dyn mob_split_api::MobSplitChild>> = (0..event.children.len())
        .map(|index| {
            Box::new(SplitChild {
                index,
                // A pending child has no entity id yet; the woven bridge named
                // the game's own id space, which mod_api does not expose here.
                info: entity_events_v1::EntityInfo { id: 0, kind: event.kind.to_owned(), dimension: event.dimension.to_owned() },
                dresses: dresses.clone(),
            }) as Box<dyn mob_split_api::MobSplitChild>
        })
        .collect();
    let mut inheritance = mob_split_api::MobSplitInheritance {
        context: &context,
        head,
        // mod_api reads no equipment drop chance (only the drop-chance hook
        // writes one), so the head's chance is vanilla's default, as the
        // woven bridge's fallback was.
        head_drop_chance: 0.085,
        custom_name,
        no_ai,
        invulnerable,
        random: &mut random,
        children: &mut children,
    };
    if let Err(error) = mob_split_api::MobSplitCallbacks::inherit(&mut inheritance) {
        eprintln!("pluto-lib: mob split inheritance failed: {error}");
        return mod_api::Flow::Continue;
    }
    for (index, dress) in dresses.borrow_mut().drain(..) {
        let Some(mut editor) = event.children.get_mut(index) else { continue };
        match dress {
            SplitDress::Equipment(slot, stack) => {
                let _ = editor.set_equipment(slot, stack);
            }
            SplitDress::Name(name) => {
                let _ = editor.set_custom_name(name);
            }
            SplitDress::NoAi(_) | SplitDress::Invulnerable(_) => {}
        }
    }
    mod_api::Flow::Continue
}

/// The captured JSON folders of one reload pass, shared by every listener of
/// the pass: all prepares finish before any apply runs, so by apply time the
/// capture holds every folder and a handler reading another folder through
/// the pure helper does not depend on listener order (the A6 design).
type ReloadCapture = std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<String, Vec<resource_loader_v0::JsonResourceStack>>>>;

struct PureReloadListener {
    id: String,
    directory: String,
    capture: ReloadCapture,
    last: bool,
}

impl mod_api::ServerReloadListener for PureReloadListener {
    fn prepare(
        &self,
        context: &mod_api::ServerReloadContext,
    ) -> Result<Box<dyn FnOnce() + Send + 'static>, mod_api::ReloadResourceError> {
        // The context is scoped to this listener's converter and its read
        // returns the winning pack layer, so each resource contributes the
        // one layer the pure decoder expects to find on top.
        let records: Vec<(String, Vec<Result<Vec<u8>, String>>)> = context
            .list()
            .map(|id| {
                let layer = context.read(id).map(|bytes| bytes.to_vec()).ok_or_else(|| "resource vanished during reload".to_string());
                (id.to_string(), vec![layer])
            })
            .collect();
        let refs: Vec<(&str, &[Result<Vec<u8>, String>])> =
            records.iter().map(|(id, layers)| (id.as_str(), layers.as_slice())).collect();
        let stacks = resource_loader_v0::decode_json_stacks(refs).map_err(|error| {
            mod_api::ReloadResourceError::new(
                mod_api::Identifier::parse(&self.id).unwrap_or_else(|_| mod_api::Identifier::parse("pluto_lib:reload").unwrap()),
                std::io::Error::other(error),
            )
        })?;
        if let Ok(mut capture) = self.capture.lock() {
            capture.insert(self.directory.clone(), stacks);
        }
        let capture = self.capture.clone();
        let id = self.id.clone();
        let directory = self.directory.clone();
        let last = self.last;
        Ok(Box::new(move || {
            let folders = capture.lock().map(|capture| capture.clone()).unwrap_or_default();
            let stacks = folders.get(&directory).cloned().unwrap_or_default();
            // The effective file of a resource is its top layer; a broken top
            // layer drops the resource (vanilla logs and skips it), no lower
            // layer silently replaces it.
            let files: Vec<(String, serde_json::Value)> = stacks
                .iter()
                .filter_map(|stack| match stack.layers.last() {
                    Some(Ok(value)) => Some((stack.id.to_string(), value.clone())),
                    _ => None,
                })
                .collect();
            // Registry enumeration has no source in the reload context, so
            // the registry half of the pure context stays empty (a lookup
            // answers "unavailable"); the folder half is complete.
            resource_loader_v0::with_reload_folders(Default::default(), folders, || {
                // The pure handlers run here, at apply time, so a handler
                // failure cannot preserve this listener's previous data the
                // way a prepare failure preserves every listener's — the one
                // two-phase difference from the woven bridge (A6).
                if let Err(error) = resource_loader_v0::apply_json_reload_listener(
                    resource_loader_v0::ResourceType::ServerData,
                    &id,
                    &directory,
                    &files,
                ) {
                    eprintln!("pluto-lib: reload listener {id} failed: {error}");
                }
                if last {
                    // No reload-finished hook exists; the simple listeners
                    // fire from the last registered listener's apply, which
                    // runs after the other applies in registration order.
                    resource_loader_v0::server_data_reloaded();
                }
            });
        }))
    }
}

pub(crate) fn add_server_reload_listeners(event: &mut mod_api::AddServerReloadListenersEvent<'_>) {
    let Ok(listeners) = resource_loader_v0::json_reload_listeners(resource_loader_v0::ResourceType::ServerData) else { return };
    if listeners.is_empty() {
        return;
    }
    let capture: ReloadCapture = std::sync::Arc::new(std::sync::Mutex::new(std::collections::BTreeMap::new()));
    let total = listeners.len();
    for (index, (id, directory)) in listeners.into_iter().enumerate() {
        let Some(native_id) = mod_api::Identifier::parse_namespaced(&id.to_string()) else { continue };
        let listener = PureReloadListener { id: id.to_string(), directory: directory.clone(), capture: capture.clone(), last: index + 1 == total };
        let Ok(converter) = mod_api::FileToIdConverter::json(&directory) else { continue };
        let _ = event.listeners.add_listener(native_id, converter, std::sync::Arc::new(listener));
    }
}
