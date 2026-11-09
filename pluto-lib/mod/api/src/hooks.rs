//! The only file that names Horizon types: it turns Horizon's hook points into PlutoLib's Fabric-like events.
//!
//! Hook points are Horizon's stable, only-ever-added surface (`native-hooks.json`). A hook that can veto returns
//! `Flow::Cancel` when the PlutoLib callbacks say so.

use horizon_client::mod_api::{
    AttackEntityEvent, ChunkLoadEvent, ClientResourcesReloadedEvent, LootTableLoadEvent, Text, TooltipEvent, ChunkUnloadEvent, DestroyBlockEvent, EntityJoinLevelEvent, EntityLeaveLevelEvent, Flow, Hooks,
    Host, LevelTickPostEvent, LevelTickPreEvent, LivingIncomingDamageEvent, PlayerLoggedInEvent, PlayerLoggedOutEvent,
    UseAnimation, UseAnimationEvent, UseDurationEvent, NativeCommand, PerformCommandEvent, RegisterCommandsEvent as RegisterCmds, ConfigLoadingEvent, ConfigReloadingEvent, ConfigUnloadingEvent, NativeConfig, NativeConfigFile, NativeConfigKind, RegisterConfigsEvent,
    RightClickBlockEvent, ServerAboutToStartEvent, ServerStoppingEvent, ServerTickEvent, ServerTickPhase, LivingDeathEvent, BreakSpeedEvent, LivingDamagePreEvent, LivingEntityUseItemFinishEvent, BlockCloneItemEvent, UseItemEvent, UseItemOnEvent,
    HearsProjectileImpactsEvent, ProjectileImpactEvent, BlockEntityDraw, BlockEntitySubmitEvent, CubeFace, DisplayContext, MenuButtonV2Event, MenuSlotRulesEvent, MenuQuickMoveEvent, ContainerAcceptsEvent, ModBlockEntity, NativeScreen, NativeStack, ScreenClickEvent, ScreenDrawEvent, ScreenEvent, ScreenTooltipEvent, BlockEntityContainerEvent, BlockEntityFluidsEvent, ModContainer, ModFluidTanks, BlockEntityEvent, BlockEntityTickEvent, BlockEntityTicksEvent, BlockEntityUpdateTagEvent, AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Tag,
};

use crate::pure::entity_events_v1::{self, LivingDamage};
use crate::pure::player_api::{self, PlayerOps};
use horizon_client::items::ItemStack;
use crate::pure::base::{Identifier, Nbt, NbtCompound};
use crate::pure::block_entity_api_v1::{self, ItemSlot};
use crate::pure::block_entity_renderer_api_v1::{self, BlockEntityCanvas};
use crate::pure::screen_handler_api_v1::{self, ScreenCanvas};
use crate::pure::player_api::{EffectInstance, FoodStatus};
use crate::pure::world_api_v1::{self, BlockState, WorldOps};
use crate::pure::{projectile_events_v1, projectile_events_v1::ProjectileImpact};
use crate::pure::block_api_v1;
use crate::pure::command_api_v2;
use crate::pure::config_api::{self, ConfigKind, ModConfig};
use crate::pure::{events_interaction_v0, item_api_v1, lifecycle_events_v1, loot_api_v2, networking_api_v1, resource_loader_v0};

/// PlutoLib's entry point: the launcher calls it before the hooks of every mod that depends on PlutoLib.
pub fn hooks() -> Box<dyn Hooks> {
    crate::pure::i18n_api::set_host(crate::pure::i18n_api::HostLanguage {
        code: horizon_client::lang::code,
        get: |key| horizon_client::lang::has(key).then(|| horizon_client::lang::get(key)),
    });
    horizon_client::game::pluto_server_stopped::set_dispatcher(lifecycle_events_v1::server_stopped);
    horizon_client::entry::pluto_block::set_sink(block_sink());
    horizon_client::creative::pluto_creative::set_document(crate::pure::item_group_api_v1::document);
    item_api_v1::set_register_sink(|item, behavior| {
        horizon_client::entry::pluto_item_use::set(
            item,
            horizon_client::entry::pluto_item_use::Spec {
                food: behavior.food.map(|food| (food.nutrition, food.saturation)),
                can_always_eat: behavior.can_always_eat,
                remainder: behavior.remainder.as_ref().map(|(id, count)| (id.to_string(), *count)),
            },
        );
    });
    Box::new(PlutoHooks)
}

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
    screen_handler_api_v1::dynamic_slots(block, data, &items)
}

struct PlutoHooks;

/// What a mod's `hooks()` returns: the launcher's entry point type. A mod that registers everything through PlutoLib's events returns
/// [`no_hooks`], so its source never names a Horizon type.
pub type ModHooks = Box<dyn Hooks>;

struct NoHooks;
impl Hooks for NoHooks {}

/// Hooks that do nothing, for a mod whose behaviour is all registered through PlutoLib events.
pub fn no_hooks() -> ModHooks {
    Box::new(NoHooks)
}

/// PlutoLib's player operations over Horizon's `Host`, open only while a callback that carries a player runs.
/// A pointer to the host that is turned into a reference only for the length of one call, so the player scope and the world scope can both
/// hold it without two live `&mut`.
#[derive(Clone, Copy)]
struct HostHandle(*mut dyn Host);

impl HostHandle {
    fn new(host: &mut dyn Host) -> Self {
        // SAFETY: the handle is only used while the callback that gave the host runs, one call at a time.
        Self(unsafe { std::mem::transmute::<&mut dyn Host, *mut dyn Host>(host) })
    }

    #[allow(clippy::mut_from_ref)]
    fn get(&self) -> &mut dyn Host {
        // SAFETY: see `new`; no reference made here outlives the call that asked for it.
        unsafe { &mut *self.0 }
    }
}

/// A stack as PlutoLib's `StackData`: its damage, max damage and the `minecraft:custom_data` the stack carries.
fn stack_data_of(_host: &dyn Host, stack: &ItemStack) -> player_api::StackData {
    read_stack(stack)
}

fn read_stack(stack: &ItemStack) -> player_api::StackData {
    let custom_data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
        Some(Tag::Compound(compound)) => compound_from_tag(compound),
        _ => NbtCompound::new(),
    };
    let max_damage = match stack.data().and_then(|d| d.other("minecraft:max_damage")) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    };
    player_api::StackData { item: stack.item.to_string(), count: stack.count, damage: stack.damage, max_damage, custom_data }
}

/// Writes count, damage, max damage and custom data of `new` onto `stack` (the item is not changed).
fn write_stack(stack: &mut ItemStack, new: &player_api::StackData) {
    stack.count = new.count;
    stack.damage = new.damage;
    // the stack's own durability: a component the game reads before the item's
    stack.data_mut().set_other("minecraft:max_damage", new.max_damage.filter(|m| *m > 0).map(Tag::Int));
    stack.data_mut().set_other("minecraft:custom_data", (!new.custom_data.is_empty()).then(|| Tag::Compound(compound_to_tag(&new.custom_data))));
    stack.tidy();
}

struct HostOps(HostHandle);

impl HostOps {
    fn host(&mut self) -> &mut dyn Host {
        self.0.get()
    }
}

/// A world that only answers block tags, over a host that can only be read.
struct ReadOnlyTags<'a>(&'a dyn Host);

impl WorldOps for ReadOnlyTags<'_> {
    fn block_state(&mut self, _: &str, _: [i32; 3]) -> Option<String> {
        None
    }

    fn set_block_state(&mut self, _: &str, _: [i32; 3], _: &str) -> bool {
        false
    }

    fn play_sound(&mut self, _: &str, _: &str, _: [f64; 3], _: f32, _: f32) -> bool {
        false
    }

    fn block_in_tag(&mut self, block: &str, tag: &str) -> bool {
        self.0.block_name_in_tag(block, tag).unwrap_or(false)
    }
}

/// The world's blocks over the host.
struct HostWorld(HostHandle);

impl WorldOps for HostWorld {
    fn block_state(&mut self, dimension: &str, pos: [i32; 3]) -> Option<String> {
        let host = self.0.get();
        let level = host.level(dimension)?;
        host.level_block_state(level, pos)
    }

    fn play_sound(&mut self, _dimension: &str, sound: &str, at: [f64; 3], volume: f32, pitch: f32) -> bool {
        // source 4 is the block category
        self.0.get().play_sound(sound, at, 4, volume, pitch)
    }

    fn set_block_state(&mut self, dimension: &str, pos: [i32; 3], state: &str) -> bool {
        let Ok(parsed) = BlockState::parse(state) else { return false };
        let host = self.0.get();
        let Some(id) = host.default_state(&parsed.block.to_string()) else { return false };
        let changed = host.set_block(dimension, pos, id, 3);
        for (key, value) in &parsed.properties {
            host.set_block_property(pos, key, value);
        }
        changed
    }
}

/// Runs `f` with the player scope and the world scope both open over the host.
fn scoped<R>(host: &mut dyn Host, f: impl FnOnce() -> R) -> R {
    let handle = HostHandle::new(host);
    player_api::with_ops(&mut HostOps(handle), || world_api_v1::with_world(&mut HostWorld(handle), f))
}

impl PlayerOps for HostOps {
    fn tell(&mut self, player: u128, line: &str) -> bool {
        self.host().tell(player, Text::literal(line.to_owned()))
    }

    fn give(&mut self, player: u128, item: &str, count: u32) -> u32 {
        self.host().give_stack(player, ItemStack::new(item, count)).map_or(0, |left| left.count)
    }

    fn stack(&mut self, player: u128, slot: i32) -> Option<player_api::StackData> {
        let _ = player; // the host reaches the acting player, who is the one the handle names
        self.host().carried_stack(slot).filter(|s| s.count > 0).map(|s| read_stack(s))
    }

    fn give_stack(&mut self, player: u128, stack: player_api::StackData) -> Option<player_api::StackData> {
        let mut made = ItemStack::new(&stack.item, stack.count);
        write_stack(&mut made, &stack);
        self.host().give_stack(player, made).map(|left| player_api::StackData { count: left.count, ..stack })
    }

    fn update_stack(&mut self, player: u128, slot: i32, expected_item: &str, new: &player_api::StackData) -> bool {
        let _ = player;
        match self.host().carried_stack(slot).filter(|s| &*s.item == expected_item && s.count > 0) {
            Some(stack) => {
                write_stack(stack, new);
                true
            }
            None => false,
        }
    }

    fn count_item(&mut self, player: u128, item: &str) -> u32 {
        let _ = player; // inside a callback the host reaches the acting player, who is the one the PlutoLib handle names
        (0..36).filter_map(|slot| self.host().carried_stack(slot).filter(|s| &*s.item == item).map(|s| s.count)).sum()
    }

    fn take_item(&mut self, player: u128, item: &str, count: u32) -> u32 {
        let _ = player;
        self.host().take_items(item, count)
    }

    fn count_in_tag(&mut self, player: u128, tag: &str) -> u32 {
        let _ = player;
        let mut total = 0;
        for slot in 0..36 {
            let Some(stack) = self.host().carried_stack(slot).cloned() else { continue };
            if self.host().server_item_in_tag(&stack, tag).unwrap_or(false) {
                total += stack.count;
            }
        }
        total
    }

    fn take_in_tag(&mut self, player: u128, tag: &str, count: u32) -> u32 {
        let _ = player;
        let mut taken = 0;
        for slot in 0..36 {
            if taken >= count {
                break;
            }
            let Some(stack) = self.host().carried_stack(slot).cloned() else { continue };
            if self.host().server_item_in_tag(&stack, tag).unwrap_or(false) {
                let item = stack.item.to_string();
                taken += self.host().take_items(&item, (count - taken).min(stack.count));
            }
        }
        taken
    }

    fn empty_slots(&mut self, player: u128) -> u32 {
        let _ = player;
        (0..36).filter(|slot| self.host().carried_stack(*slot).is_none()).count() as u32
    }

    fn language(&mut self, player: u128) -> Option<String> {
        self.host().player_client(player).map(|client| client.language)
    }

    fn add_effect(&mut self, player: u128, effect: &EffectInstance) -> bool {
        self.host().add_effect(
            player,
            &horizon_client::game::NativeEffect {
                id: effect.id.clone(),
                amplifier: effect.amplifier,
                duration: effect.duration,
                ambient: effect.ambient,
                particles: effect.particles,
            },
        )
    }

    fn remove_effect(&mut self, player: u128, id: &str) -> bool {
        self.host().remove_effect(player, id)
    }

    fn effects(&mut self, player: u128) -> Option<Vec<EffectInstance>> {
        let effects = self.host().effects(player)?;
        Some(
            effects
                .into_iter()
                .map(|e| EffectInstance { id: e.id, amplifier: e.amplifier, duration: e.duration, ambient: e.ambient, particles: e.particles })
                .collect(),
        )
    }

    fn food(&mut self, player: u128) -> Option<FoodStatus> {
        self.host().player_food(player).map(|(level, saturation, exhaustion)| FoodStatus { level, saturation, exhaustion })
    }

    fn attachment_get(&mut self, player: u128, id: &str) -> Option<String> {
        let kind = json_attachment(id)?;
        match kind.get_existing(self.host(), Holder::Player(player)) {
            Ok(Some(json)) if !json.is_empty() => Some(json),
            _ => None,
        }
    }

    fn attachment_set(&mut self, player: u128, id: &str, json: &str) -> bool {
        match json_attachment(id) {
            Some(kind) => kind.set(self.host(), Holder::Player(player), json.to_owned()).is_ok(),
            None => false,
        }
    }
}

/// Horizon wants attachment keys as `'static` strs and a codec made of plain functions, while PlutoLib attachments are created at run time.
/// So each id gets one leaked `AttachmentType<String>` (made once, kept for the process) whose value is the JSON text in a string tag.
fn json_attachment(id: &str) -> Option<&'static AttachmentType<String>> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static KINDS: OnceLock<Mutex<HashMap<String, &'static AttachmentType<String>>>> = OnceLock::new();
    let (namespace, name) = id.split_once(':')?;
    let mut kinds = KINDS.get_or_init(Default::default).lock().ok()?;
    if let Some(kind) = kinds.get(id) {
        return Some(*kind);
    }
    let kind: &'static AttachmentType<String> = Box::leak(Box::new(AttachmentType {
        namespace: Box::leak(namespace.to_owned().into_boxed_str()),
        name: Box::leak(name.to_owned().into_boxed_str()),
        default: String::new,
        codec: Some(AttachmentCodec {
            encode: |json: &String| Ok([("json".into(), Tag::String(json.as_str().into()))].into()),
            decode: |data: &Compound| match data.get(&"json".into()) {
                Some(Tag::String(s)) => Ok(s.to_string_lossy()),
                _ => Err(AttachmentError::Decode),
            },
        }),
        copy_on_death: false,
        sync: false,
    }));
    kinds.insert(id.to_owned(), kind);
    Some(kind)
}

fn cancel_if(handled: bool) -> Flow {
    if handled { Flow::Cancel } else { Flow::Continue }
}

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

fn tag_from_nbt(nbt: &Nbt) -> Tag {
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

fn compound_from_tag(compound: &Compound) -> NbtCompound {
    compound.iter().filter_map(|(key, tag)| Some((key.to_string_lossy(), nbt_from_tag(tag)?))).collect()
}

fn compound_to_tag(compound: &NbtCompound) -> Compound {
    compound.iter().map(|(key, nbt)| (key.as_str().into(), tag_from_nbt(nbt))).collect()
}

/// A stack as an item slot: item, count, damage, max damage and custom data; None for an item id PlutoLib cannot name.
fn slot_of(stack: &ItemStack) -> Option<ItemSlot> {
    let data = read_stack(stack);
    Some(ItemSlot { item: crate::pure::base::Identifier::parse(&stack.item).ok()?, count: data.count, damage: data.damage, max_damage: data.max_damage, custom_data: data.custom_data })
}

/// Writes an item slot's count, damage, max damage and custom data onto a stack of the same item.
fn write_slot(stack: &mut ItemStack, slot: &ItemSlot) {
    write_stack(stack, &player_api::StackData { item: stack.item.to_string(), count: slot.count, damage: slot.damage, max_damage: slot.max_damage, custom_data: slot.custom_data.clone() });
}

/// Runs `f` on a block entity's data and item slots as PlutoLib's types and writes back what changed; a slot whose item is unchanged keeps
/// its data and only takes the new count. True if `f` did something (its own answer).
fn with_block_entity(entity: &mut ModBlockEntity, f: impl FnOnce(&mut NbtCompound, &mut Vec<Option<ItemSlot>>) -> bool) -> bool {
    let mut data = compound_from_tag(&entity.nbt);
    let before = data.clone();
    let original: Vec<Option<ItemStack>> = entity.items.clone();
    let mut items: Vec<Option<ItemSlot>> = original
        .iter()
        .map(|slot| slot.as_ref().and_then(slot_of))
        .collect();
    let before_items = items.clone();
    let ran = f(&mut data, &mut items);
    if ran {
        if let Some(Nbt::IntArray(values)) = data.get("pluto:menu_data") {
            entity.data = values.iter().take(256).map(|v| (*v).clamp(i16::MIN as i32, i16::MAX as i32)).collect();
        }
        if data != before {
            entity.nbt = compound_to_tag(&data);
        }
        if items != before_items {
            entity.items = items
                .iter()
                .zip(original.iter().chain(std::iter::repeat(&None)))
                .map(|(now, was)| match (now, was) {
                    (None, _) => None,
                    (Some(slot), Some(old)) if old.item.as_ref() == slot.item.to_string() => {
                        let mut kept = old.clone();
                        write_slot(&mut kept, slot);
                        Some(kept)
                    }
                    (Some(slot), _) => {
                        let mut made = ItemStack::new(&slot.item.to_string(), slot.count);
                        write_slot(&mut made, slot);
                        Some(made)
                    }
                })
                .collect();
        }
    }
    ran
}

/// What a block entity renderer draws on: the game's draw state for this block entity.
struct HostDraw<'a> {
    host: &'a dyn Host,
    draw: &'a mut BlockEntityDraw,
}

impl BlockEntityCanvas for HostDraw<'_> {
    fn push(&mut self) {
        self.draw.push();
    }

    fn pop(&mut self) {
        self.draw.pop();
    }

    fn translate(&mut self, by: [f32; 3]) {
        self.draw.translate(by);
    }

    fn rotate_about(&mut self, axis: [f32; 3], degrees: f32, pivot: [f32; 3]) {
        self.draw.rotate_about(axis, degrees, pivot);
    }

    fn scale(&mut self, by: [f32; 3]) {
        self.draw.scale(by);
    }

    fn quad(&mut self, sprite: &str, corners: [[f32; 3]; 4], uv: [f32; 4], tint: u32) -> bool {
        self.host.draw_quad(self.draw, sprite, corners, uv, tint)
    }

    fn cube(&mut self, from: [f32; 3], to: [f32; 3], sprite: &str, tint: u32) -> bool {
        let face = CubeFace { sprite, uv: None };
        self.host.draw_cube(self.draw, from, to, [Some(face); 6], tint)
    }

    fn block(&mut self, block: &str, properties: &[(&str, &str)]) -> bool {
        self.host.draw_block(self.draw, block, properties)
    }

    fn item(&mut self, item: &ItemSlot) -> bool {
        self.host.draw_item(self.draw, &ItemStack::new(&item.item.to_string(), item.count), DisplayContext::Fixed, None, 0)
    }

    fn model(&mut self, model: &str, tint: u32) -> bool {
        self.host.draw_model(self.draw, model, tint)
    }

    fn keep(&mut self) {
        self.draw.keep();
    }
}

/// What a screen draws on: the game's batch for the frame.
struct HostCanvas<'a> {
    host: &'a dyn Host,
    batch: &'a mut horizon_client::hud::HudBatch,
}

impl ScreenCanvas for HostCanvas<'_> {
    fn fill(&mut self, rect: [f32; 4], argb: u32) {
        self.host.fill(self.batch, rect, argb);
    }

    fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4]) -> bool {
        self.host.sprite(self.batch, name, src, dst)
    }

    fn text(&mut self, text: &str, at: [f32; 2], rgb: u32, shadow: bool) {
        self.host.text(self.batch, text, at, rgb, shadow);
    }

    fn text_width(&self, text: &str) -> f32 {
        self.host.text_width(text)
    }
}

fn model_config(file: &NativeConfigFile) -> ModConfig {
    let kind = match file.kind {
        NativeConfigKind::Client => ConfigKind::Client,
        NativeConfigKind::Common => ConfigKind::Common,
        NativeConfigKind::Server => ConfigKind::Server,
    };
    ModConfig { mod_id: file.mod_id.clone(), kind, file: file.file.clone(), text: file.text.clone() }
}

/// The game's view of the blocks around, as a behaviour sees it.
struct GameView<'a>(&'a dyn horizon_client::entry::pluto_block::LevelView);

fn to_state(info: &horizon_client::entry::pluto_block::StateInfo) -> Option<BlockState> {
    Some(BlockState { block: crate::pure::base::Identifier::parse(&info.block).ok()?, properties: info.properties.clone() })
}

fn to_info(state: &BlockState) -> horizon_client::entry::pluto_block::StateInfo {
    horizon_client::entry::pluto_block::StateInfo { block: state.block.to_string(), properties: state.properties.clone() }
}

impl block_api_v1::BlockView for GameView<'_> {
    fn block_state(&self, pos: [i32; 3]) -> Option<BlockState> {
        self.0.state(pos).as_ref().and_then(to_state)
    }

    fn is_air(&self, pos: [i32; 3]) -> bool {
        self.0.is_air(pos)
    }

    fn is_face_sturdy(&self, pos: [i32; 3], face: block_api_v1::Direction) -> bool {
        self.0.face_sturdy(pos, face as u8)
    }

    fn is_in_tag(&self, pos: [i32; 3], tag: &str) -> bool {
        self.0.in_tag(pos, tag)
    }
    fn brightness(&self, pos: [i32; 3]) -> u8 {
        self.0.brightness(pos)
    }

    fn light_passes_up(&self, pos: [i32; 3]) -> bool {
        self.0.light_passes_up(pos)
    }

    fn is_water(&self, pos: [i32; 3]) -> bool {
        self.0.is_water(pos)
    }

    fn is_fluid_full(&self, pos: [i32; 3]) -> bool {
        self.0.fluid_full(pos)
    }

    fn is_area_loaded(&self, pos: [i32; 3], radius: i32) -> bool {
        self.0.area_loaded(pos, radius)
    }

    fn is_full_block(&self, pos: [i32; 3]) -> bool {
        self.0.full_block(pos)
    }

    fn build_height(&self) -> [i32; 2] {
        self.0.build_height()
    }
}

/// The game's view of the blocks around, with the right to change them.
struct GameEditor<'a>(&'a mut dyn horizon_client::entry::pluto_block::LevelEdit);

impl block_api_v1::BlockView for GameEditor<'_> {
    fn block_state(&self, pos: [i32; 3]) -> Option<BlockState> {
        self.0.state(pos).as_ref().and_then(to_state)
    }

    fn is_air(&self, pos: [i32; 3]) -> bool {
        self.0.is_air(pos)
    }

    fn is_face_sturdy(&self, pos: [i32; 3], face: block_api_v1::Direction) -> bool {
        self.0.face_sturdy(pos, face as u8)
    }

    fn is_in_tag(&self, pos: [i32; 3], tag: &str) -> bool {
        self.0.in_tag(pos, tag)
    }
    fn brightness(&self, pos: [i32; 3]) -> u8 {
        self.0.brightness(pos)
    }

    fn light_passes_up(&self, pos: [i32; 3]) -> bool {
        self.0.light_passes_up(pos)
    }

    fn is_water(&self, pos: [i32; 3]) -> bool {
        self.0.is_water(pos)
    }

    fn is_fluid_full(&self, pos: [i32; 3]) -> bool {
        self.0.fluid_full(pos)
    }

    fn is_area_loaded(&self, pos: [i32; 3], radius: i32) -> bool {
        self.0.area_loaded(pos, radius)
    }

    fn is_full_block(&self, pos: [i32; 3]) -> bool {
        self.0.full_block(pos)
    }

    fn build_height(&self) -> [i32; 2] {
        self.0.build_height()
    }
}

impl block_api_v1::BlockEditor for GameEditor<'_> {
    fn set_block_state(&mut self, pos: [i32; 3], state: &BlockState) -> bool {
        self.0.set_state(pos, &to_info(state), 3)
    }

    fn random_int(&mut self, bound: i32) -> i32 {
        self.0.random_int(bound)
    }

    fn random_float(&mut self) -> f32 {
        self.0.random_float()
    }

    fn schedule_tick(&mut self, pos: [i32; 3], block: &str, delay: i32) -> bool {
        self.0.schedule_tick(pos, block, delay)
    }

    fn place_feature(&mut self, feature: &str, pos: [i32; 3]) -> bool {
        self.0.place_feature(feature, pos)
    }
}

fn direction(face: u8) -> block_api_v1::Direction {
    block_api_v1::Direction::ALL[usize::from(face).min(5)]
}

fn block_sink() -> horizon_client::entry::pluto_block::Sink {
    use horizon_client::entry::pluto_block::{LevelEdit, LevelView, Placement, PlacementAnswer, ShapeUpdate, Sink, StateInfo};
    fn survives(view: &dyn LevelView, state: &StateInfo, pos: [i32; 3]) -> Option<bool> {
        let state = to_state(state)?;
        let block = state.block.to_string();
        block_api_v1::survives(&block, &block_api_v1::SurviveContext { state, pos, view: &GameView(view) })
    }
    fn placement(view: &dyn LevelView, request: &Placement) -> PlacementAnswer {
        let Some(state) = to_state(&request.state) else { return PlacementAnswer::Default };
        let block = state.block.to_string();
        let context = block_api_v1::PlacementContext {
            state,
            pos: request.pos,
            face: direction(request.face),
            yaw: request.yaw,
            pitch: request.pitch,
            sneaking: request.sneaking,
            in_water: request.in_water,
            item: request.item.clone(),
            replace_clicked: request.replace_clicked,
            view: &GameView(view),
        };
        match block_api_v1::placement(&block, &context) {
            block_api_v1::Placement::Default => PlacementAnswer::Default,
            block_api_v1::Placement::Refuse => PlacementAnswer::Refuse,
            block_api_v1::Placement::State(chosen) => PlacementAnswer::State(to_info(&chosen)),
        }
    }
    fn update_shape(view: &dyn LevelView, update: &ShapeUpdate) -> Option<StateInfo> {
        let state = to_state(&update.state)?;
        let block = state.block.to_string();
        let context = block_api_v1::ShapeUpdateContext {
            state,
            pos: update.pos,
            direction: direction(update.direction),
            neighbour_pos: update.neighbour_pos,
            neighbour: update.neighbour.as_ref().and_then(to_state),
            view: &GameView(view),
        };
        block_api_v1::shape_update(&block, &context).map(|changed| to_info(&changed))
    }
    fn entity_inside(dimension: &str, state: &StateInfo, pos: [i32; 3], entity: i32, kind: &str) {
        let Some(state) = to_state(state) else { return };
        let block = state.block.to_string();
        block_api_v1::inside(&block, &block_api_v1::EntityInsideContext {
            dimension: dimension.to_owned(), state, pos, entity, kind: kind.to_owned(),
        });
    }
    fn random_tick(world: &mut dyn LevelEdit, state: &StateInfo, pos: [i32; 3]) -> bool {
        let Some(state) = to_state(state) else { return false };
        let block = state.block.to_string();
        block_api_v1::random_tick(&block, &mut block_api_v1::TickContext { state, pos, world: &mut GameEditor(world) })
    }
    fn scheduled_tick(world: &mut dyn LevelEdit, state: &StateInfo, pos: [i32; 3]) -> bool {
        let Some(state) = to_state(state) else { return false };
        let block = state.block.to_string();
        block_api_v1::scheduled_tick(&block, &mut block_api_v1::TickContext { state, pos, world: &mut GameEditor(world) })
    }
    fn bonemeal_valid(view: &dyn LevelView, state: &StateInfo, pos: [i32; 3]) -> Option<bool> {
        let state = to_state(state)?;
        let block = state.block.to_string();
        block_api_v1::can_bonemeal(&block, &block_api_v1::BonemealContext { state, pos, view: &GameView(view) })
    }
    fn bonemeal_success(world: &mut dyn LevelEdit, state: &StateInfo, pos: [i32; 3]) -> Option<bool> {
        let state = to_state(state)?;
        let block = state.block.to_string();
        block_api_v1::bonemeal_success(&block, &mut block_api_v1::TickContext { state, pos, world: &mut GameEditor(world) })
    }
    fn bonemeal_perform(world: &mut dyn LevelEdit, state: &StateInfo, pos: [i32; 3]) {
        let Some(state) = to_state(state) else { return };
        let block = state.block.to_string();
        block_api_v1::on_bonemeal(&block, &mut block_api_v1::TickContext { state, pos, world: &mut GameEditor(world) });
    }
    fn may_place_on(view: &dyn LevelView, plant: &StateInfo, ground: &StateInfo, ground_pos: [i32; 3]) -> Option<bool> {
        let (plant, ground) = (to_state(plant)?, to_state(ground)?);
        let block = plant.block.to_string();
        block_api_v1::may_place_on(&block, &block_api_v1::PlaceOnContext { state: plant, ground, ground_pos, view: &GameView(view) })
    }
    Sink {
        has: |block| block_api_v1::BlockBehaviors::get(block).is_some(),
        survives,
        placement,
        update_shape,
        entity_inside,
        random_tick,
        scheduled_tick,
        bonemeal_valid,
        bonemeal_success,
        bonemeal_perform,
        may_place_on,
    }
}

impl Hooks for PlutoHooks {
    fn use_duration(&self, _host: &dyn Host, event: &mut UseDurationEvent<'_>) {
        event.duration = item_api_v1::use_duration(&event.stack.item, event.duration);
    }

    fn use_animation(&self, _host: &dyn Host, event: &mut UseAnimationEvent<'_>) {
        use item_api_v1::UseAction;
        if let Some(action) = item_api_v1::use_animation(&event.stack.item) {
            *event.animation = match action {
                UseAction::None => UseAnimation::None,
                UseAction::Eat => UseAnimation::Eat,
                UseAction::Drink => UseAnimation::Drink,
                UseAction::Block => UseAnimation::Block,
                UseAction::Bow => UseAnimation::Bow,
                UseAction::Trident => UseAnimation::Trident,
                UseAction::Crossbow => UseAnimation::Crossbow,
                UseAction::Spyglass => UseAnimation::Spyglass,
                UseAction::TootHorn => UseAnimation::TootHorn,
                UseAction::Brush => UseAnimation::Brush,
                UseAction::Bundle => UseAnimation::Bundle,
                UseAction::Spear => UseAnimation::Spear,
            };
        }
    }

    fn block_entity(&self, _host: &mut dyn Host, event: &mut BlockEntityEvent<'_>) -> Option<(String, Compound)> {
        block_entity_api_v1::create(event.block).map(|(id, data)| (id, compound_to_tag(&data)))
    }

    fn block_entity_container(&self, _host: &dyn Host, event: &mut BlockEntityContainerEvent<'_>) -> Option<ModContainer> {
        let slots = block_entity_api_v1::item_slots(event.block)? as usize;
        let all: Vec<usize> = (0..slots).collect();
        Some(ModContainer { slots, faces: std::array::from_fn(|_| all.clone()), inputs: all.clone(), outputs: all, menu: block_entity_api_v1::menu(event.block) })
    }

    fn block_entity_fluids(&self, _host: &dyn Host, event: &mut BlockEntityFluidsEvent<'_>) -> Option<ModFluidTanks> {
        let tanks = block_entity_api_v1::fluid_tanks(event.block)?;
        let all: Vec<usize> = (0..tanks.len()).collect();
        Some(ModFluidTanks { tanks: tanks.iter().map(|&c| c as i32).collect(), faces: std::array::from_fn(|_| all.clone()), inputs: all.clone(), outputs: all })
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        if block_entity_api_v1::ticks(event.block) {
            event.ticks = true;
        }
    }

    fn block_entity_tick(&self, _host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
        let (pos, id) = (event.pos, event.entity.id.clone());
        with_block_entity(event.entity, |data, items| block_entity_api_v1::tick(&id, pos, data, items));
    }

    fn menu_button_v2(&self, _host: &mut dyn Host, event: &mut MenuButtonV2Event<'_>) {
        let (pos, button) = (event.pos, event.id);
        let Some(block) = block_entity_api_v1::block_of(&event.entity.id) else { return };
        let mut carried: Option<ItemSlot> = event.carried.as_ref().and_then(slot_of);
        let before = carried.clone();
        with_block_entity(event.entity, |data, items| screen_handler_api_v1::press(&block, pos, button, data, items, &mut carried));
        if carried != before {
            *event.carried = carried.map(|s| ItemStack::new(&s.item.to_string(), s.count));
        }
    }

    fn menu_slot_rules(&self, event: &mut MenuSlotRulesEvent<'_>) {
        let Some(block) = block_entity_api_v1::block_of_menu(event.menu) else { return };
        screen_handler_api_v1::slot_rules(&block, event.slot, &event.stack.item, event.stack.count, &mut event.may_place, &mut event.max_stack);
    }

    fn menu_quick_move(&self, _host: &mut dyn Host, event: &mut MenuQuickMoveEvent<'_>) -> Flow {
        let Some(block) = block_entity_api_v1::block_of(&event.entity.id) else { return Flow::Continue };
        let Ok(item) = crate::pure::base::Identifier::parse(&event.stack.item) else { return Flow::Continue };
        let mut moved = ItemSlot::new(item, event.stack.count);
        let (pos, slot, container_slots) = (event.pos, event.slot, event.container_slots);
        let mut handled = false;
        with_block_entity(event.entity, |data, items| {
            handled = screen_handler_api_v1::quick_move(&block, pos, slot, container_slots, &mut moved, data, items);
            handled
        });
        if handled {
            event.stack.count = moved.count;
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn container_accepts(&self, _host: &dyn Host, event: &mut ContainerAcceptsEvent<'_>) -> Flow {
        let Some(block) = block_entity_api_v1::block_of(&event.entity.id) else { return Flow::Continue };
        let face = event.face.map(|d| direction(d as u8));
        if screen_handler_api_v1::accepts(&block, event.pos, event.slot, &event.stack.item, event.stack.count, face) { Flow::Continue } else { Flow::Cancel }
    }

    fn block_entity_submit(&self, host: &dyn Host, event: &mut BlockEntitySubmitEvent<'_>) {
        if !block_entity_renderer_api_v1::BlockEntityRendererRegistry::has(event.block) {
            return;
        }
        let data = compound_from_tag(&event.entity.nbt);
        let mut canvas = HostDraw { host, draw: &mut *event.draw };
        block_entity_renderer_api_v1::render(event.block, event.pos, &data, &mut canvas);
    }

    fn screen(&self, _host: &dyn Host, event: &mut ScreenEvent<'_>) -> Option<NativeScreen> {
        let layout = screen_handler_api_v1::layout(event.block)?;
        let mut screen = NativeScreen::default();
        screen.width = layout.width;
        screen.height = layout.height;
        screen.slots = layout.slots;
        screen.inventory_y = layout.inventory_y;
        screen.inventory_x = layout.inventory_x;
        screen.own_labels = layout.own_labels;
        screen.own_background = layout.own_background;
        if screen_handler_api_v1::has_dynamic_slots(event.block) {
            horizon_client::game::pluto_dynamic_screen_slots::set_resolver(resolve_dynamic_screen_slots);
        }
        Some(screen)
    }

    fn screen_draw(&self, host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
        let mut canvas = HostCanvas { host, batch: &mut *event.batch };
        screen_handler_api_v1::draw(event.block, event.origin, event.data, &mut canvas);
    }

    fn screen_click(&self, _host: &dyn Host, event: &mut ScreenClickEvent<'_>) -> Option<i32> {
        screen_handler_api_v1::click(event.block, event.at, event.right, event.data)
    }

    fn screen_tooltip(&self, _host: &dyn Host, event: &mut ScreenTooltipEvent<'_>) {
        screen_handler_api_v1::tooltip(event.block, event.at, event.data, event.lines);
    }

    fn block_entity_update_tag(&self, event: &mut BlockEntityUpdateTagEvent<'_>) -> Option<Compound> {
        block_entity_api_v1::update_tag(&event.entity.id, &compound_from_tag(&event.entity.nbt)).map(|tag| compound_to_tag(&tag))
    }

    fn hears_projectile_impacts(&self, _host: &dyn Host, event: &mut HearsProjectileImpactsEvent) {
        if projectile_events_v1::heard() {
            event.hears = true;
        }
    }

    fn projectile_impact(&self, host: &mut dyn Host, event: &mut ProjectileImpactEvent<'_>) -> Flow {
        let hit = ProjectileImpact {
            dimension: event.dimension.to_owned(),
            projectile: event.projectile,
            kind: event.kind.to_owned(),
            owner: i64::from(event.owner),
            entity: event.entity,
            entity_kind: event.entity_kind.to_owned(),
            block: event.block,
            face: event.face,
            at: event.at,
        };
        let allowed = scoped(host, || projectile_events_v1::impact(&hit));
        if allowed { Flow::Continue } else { Flow::Cancel }
    }

    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCmds<'_>) {
        for (usage, level) in command_api_v2::collect() {
            event.commands.push(if level == 0 { NativeCommand::new(&usage) } else { NativeCommand::op(&usage, level) });
        }
    }

    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent<'_>) -> Flow {
        let (uuid, name, line) = (event.player.uuid, event.player.name.clone(), event.command.to_owned());
        let ran = scoped(host, || command_api_v2::execute(uuid, name, &line));
        cancel_if(ran)
    }

    fn register_configs(&self, _host: &mut dyn Host, event: &mut RegisterConfigsEvent<'_>) {
        for spec in config_api::registered() {
            let kind = match spec.kind {
                ConfigKind::Client => NativeConfigKind::Client,
                ConfigKind::Common => NativeConfigKind::Common,
                ConfigKind::Server => NativeConfigKind::Server,
            };
            let config = NativeConfig::new(&spec.mod_id, kind, &spec.default_text);
            event.configs.push(match &spec.file {
                Some(file) => config.file(file),
                None => config,
            });
        }
    }

    fn config_loading(&self, _host: &mut dyn Host, event: &mut ConfigLoadingEvent<'_>) {
        config_api::loading(model_config(event.config));
    }

    fn config_reloading(&self, _host: &mut dyn Host, event: &mut ConfigReloadingEvent<'_>) {
        config_api::reloading(model_config(event.config));
    }

    fn config_unloading(&self, _host: &mut dyn Host, event: &mut ConfigUnloadingEvent<'_>) {
        config_api::unloading(model_config(event.config));
    }

    fn server_about_to_start(&self, _host: &mut dyn Host, event: &mut ServerAboutToStartEvent) {
        lifecycle_events_v1::server_starting(event.dedicated);
    }

    fn server_stopping(&self, _host: &mut dyn Host, event: &mut ServerStoppingEvent) {
        lifecycle_events_v1::server_stopping(event.dedicated);
    }

    fn server_tick(&self, _host: &mut dyn Host, event: &mut ServerTickEvent<'_>) {
        match event.phase {
            ServerTickPhase::ServerTickStart { .. } => {
                crate::pure::i18n_api::poll_language();
                lifecycle_events_v1::server_tick(true)
            }
            ServerTickPhase::ServerTickEnd { .. } => lifecycle_events_v1::server_tick(false),
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }

    fn level_tick_pre(&self, _host: &mut dyn Host, event: &mut LevelTickPreEvent<'_>) {
        lifecycle_events_v1::world_tick(true, event.dimension);
    }

    fn level_tick_post(&self, _host: &mut dyn Host, event: &mut LevelTickPostEvent<'_>) {
        lifecycle_events_v1::world_tick(false, event.dimension);
    }

    fn chunk_load(&self, _host: &mut dyn Host, event: &mut ChunkLoadEvent<'_>) {
        lifecycle_events_v1::chunk_loaded(event.dimension, event.chunk, event.new);
    }

    fn chunk_unload(&self, _host: &mut dyn Host, event: &mut ChunkUnloadEvent<'_>) {
        lifecycle_events_v1::chunk_unloaded(event.dimension, event.chunk);
    }

    fn player_logged_in(&self, host: &mut dyn Host, event: &mut PlayerLoggedInEvent<'_>) {
        let (uuid, name) = (event.player, event.name.to_string());
        scoped(host, || networking_api_v1::player_joined(uuid, name));
    }

    fn player_logged_out(&self, host: &mut dyn Host, event: &mut PlayerLoggedOutEvent<'_>) {
        let (uuid, name) = (event.player, event.name.to_string());
        scoped(host, || networking_api_v1::player_left(uuid, name));
    }

    fn entity_join_level(&self, _host: &mut dyn Host, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        entity_events_v1::entity_loaded(event.entity, event.kind, event.dimension);
        Flow::Continue
    }

    fn entity_leave_level(&self, _host: &mut dyn Host, event: &mut EntityLeaveLevelEvent<'_>) {
        entity_events_v1::entity_unloaded(event.entity, event.kind, event.dimension);
    }

    fn use_item(&self, _host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        cancel_if(events_interaction_v0::use_item(&event.stack.item, event.stack.count))
    }

    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        let Some(player) = host.acting_player() else { return Flow::Continue };
        let context = events_interaction_v0::UseItemOnBlockContext {
            item: event.stack.item.to_string(),
            count: event.stack.count,
            pos: event.pos,
            face: event.face,
            dimension: player.dimension.clone(),
            creative: player.game_mode == 1,
            player: player_api::ServerPlayer { uuid: player.uuid, name: player.name.clone() },
        };
        cancel_if(scoped(host, || events_interaction_v0::use_item_on_block(&context)))
    }

    fn living_entity_use_item_finish(&self, host: &mut dyn Host, event: &mut LivingEntityUseItemFinishEvent<'_>) {
        if event.entity.is_some() { return; }
        let Some(player) = host.acting_player() else { return };
        let context = events_interaction_v0::UseItemFinishContext {
            item: event.stack.item.to_string(),
            player: player_api::ServerPlayer { uuid: player.uuid, name: player.name.clone() },
        };
        scoped(host, || events_interaction_v0::use_item_finish(&context));
    }

    fn break_speed(&self, host: &dyn Host, event: &mut BreakSpeedEvent) {
        let Some(level) = host.level(host.dimension()) else { return };
        let Some(block) = host.level_block_state(level, event.pos).and_then(|text| BlockState::parse(&text).ok()) else { return };
        let held = host.acting_player().and_then(|player| host.held_stack(player.uuid)).map(|stack| stack_data_of(host, &stack));
        // the callbacks may ask whether the block is in a tag: a read-only world over the host for the length of the call
        let mut tags = ReadOnlyTags(host);
        event.speed = world_api_v1::with_world(&mut tags, || events_interaction_v0::break_speed(event.pos, block, event.original, event.speed, held));
    }

    fn block_clone_item(&self, host: &dyn Host, event: &mut BlockCloneItemEvent<'_>) -> Option<ItemStack> {
        let level = host.level(host.dimension())?;
        let state = BlockState::parse(&host.level_block_state(level, event.pos)?).ok()?;
        let picked = block_api_v1::pick(event.block, &block_api_v1::PickContext { state, pos: event.pos })?;
        // data components of the picked stack are not carried over yet
        Some(ItemStack::new(&picked.item.to_string(), picked.count))
    }

    fn right_click_block(&self, _host: &mut dyn Host, event: &mut RightClickBlockEvent) -> Flow {
        cancel_if(events_interaction_v0::use_block(event.pos, event.face))
    }

    fn attack_entity(&self, host: &mut dyn Host, event: &mut AttackEntityEvent<'_>) -> Flow {
        let held = host.acting_player().and_then(|player| host.held_stack(player.uuid)).map(|stack| stack_data_of(host, &stack));
        entity_events_v1::attack_started(event.entity, held);
        cancel_if(events_interaction_v0::attack_entity(event.entity, event.kind, event.pos))
    }

    fn destroy_block(&self, _host: &mut dyn Host, event: &mut DestroyBlockEvent) -> Flow {
        cancel_if(!events_interaction_v0::before_block_break(event.pos))
    }

    fn tooltip(&self, _host: &dyn Host, event: &mut TooltipEvent<'_>) {
        for line in item_api_v1::item_tooltip(read_stack(event.stack)) {
            event.lines.push((if line.translate { Text::translate(&line.text, &[]) } else { Text::literal(line.text) }, line.color));
        }
    }

    fn loot_table_load(&self, event: &mut LootTableLoadEvent<'_>) {
        let edited = loot_api_v2::modify(event.id, event.table.clone());
        *event.table = edited;
    }

    fn client_resources_reloaded(&self, _host: &mut dyn Host, _event: &mut ClientResourcesReloadedEvent) {
        resource_loader_v0::client_resources_reloaded();
    }

    fn living_death(&self, host: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        let death = LivingDamage {
            entity: event.entity as i32,
            kind: event.kind.to_owned(),
            damage_type: event.damage_type.to_owned(),
            attacker: if event.attacker == 0 { -1 } else { event.attacker as i32 },
            direct: -1,
            amount: 0.0,
        };
        scoped(host, || entity_events_v1::after_death(&death));
        Flow::Continue
    }

    fn living_damage_pre(&self, event: &mut LivingDamagePreEvent<'_>) {
        let damage = LivingDamage {
            entity: event.entity,
            kind: event.kind.to_owned(),
            damage_type: event.damage_type.to_owned(),
            attacker: event.causing,
            direct: event.direct,
            amount: event.amount,
        };
        event.amount = entity_events_v1::modify_damage(&damage);
    }

    fn living_incoming_damage(&self, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        let damage = LivingDamage {
            entity: event.entity,
            kind: event.kind.to_owned(),
            damage_type: event.damage_type.to_owned(),
            attacker: event.causing,
            direct: event.direct,
            amount: event.amount,
        };
        cancel_if(!entity_events_v1::allow_damage(&damage))
    }
}
