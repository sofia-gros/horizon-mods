//! The fluid effects' live level and targets: the production [`WorldAccess`] and [`EntityTarget`] the
//! cannon and the fluid projectile run the engine against (the engine's own doubles live in its tests).
//! Everything here is backed by PlutoLib's world and entity reaches inside the callback that opened
//! them (a block entity tick, a projectile impact); a reach the host does not have is answered the way
//! the engine's contract expects — the effect that needs it does not run — and each such answer is
//! named below rather than faked:
//!
//! - No explosion surface exists in PlutoLib's world API, so [`LiveWorld::explode`] does nothing: the
//!   explosion effects of the shipped fluids (magma, redstone, several molten metals) charge for their
//!   scale, as the engine's accounting does for any context, but no explosion happens until PlutoLib
//!   grows an explode operation.
//! - Particles (destroy particles, the cannon's shot particles) have no emission surface; sounds do.
//! - Potion contents are not read: no potion registry reach hands out a potion's effects, so
//!   `potion` / `potion_cloud` effects find no effects to give (their fluid's other effects still run).
//! - The vanilla interaction pipelines are not reachable, so `interact` effects answer Pass.
//! - Blocks cannot be piston-moved, so nothing is pushable through this context and move effects rest.
//! - An entity's active effects, air and equipment cannot be read by id; a target answers the
//!   conservative default (no effect read, full air, no helmet) and effect removal/curing do not land.
//!   Fire can be raised but not lowered (the actions vocabulary's SetFire only lengthens a burn;
//!   freezing a target puts its fire out as a side effect, as the game's own freeze does).
//! - A push by an arbitrary motion vector has no surface (the actions vocabulary's knockback is
//!   strength-and-direction based, not the same push), so `push` does nothing; hurts land through the
//!   entity reach, and for the entity a projectile directly hit, given effects, fire and freezing land
//!   through the impact's own actions, which PlutoLib does to the hit's parties after the callback.
//!   Area targets (an area effect's neighbours) are reached for hurts and facts only.
use crate::common;
use crate::fluid_effects::{
    ActiveEffect, BlockContext, BreakSpec, CloudSpec, EntityInteractSpec, EntityTarget, ExplosionSpec, Holder, InteractSpec,
    InteractionResult, NewEffect, PlaceSpec, PotionEffect, PushReaction, Stack, WorldAccess,
};
use crate::fluid_ingredient::{ItemOutput, Source};
use pluto_lib::{
    ActionTarget, BlockPos, BlockState, Direction, EntityAction, EntityReach, FluidRegistry, FluidStack, FluidWorld, Identifier,
    ItemSlot, Nbt, NbtCompound, NbtCompoundExt, ProjectileImpact, RegistryLookup, World, WorldEntity,
};
use serde_json::Value;
use std::cell::RefCell;
use std::sync::Arc;

/// An item slot as an engine stack: item, count, and the custom data as the stack's NBT (an empty
/// custom data is no NBT). The slot's damage is not part of an engine stack, as nowhere in the engine.
pub(crate) fn stack_of_slot(slot: &ItemSlot) -> Stack {
    Stack {
        item: slot.item.clone(),
        count: slot.count as i32,
        nbt: if slot.custom_data.is_empty() { None } else { Some(nbt_to_value(&slot.custom_data)) },
    }
}

/// Writes an engine stack back into a cannon's item slot after an application: what the context's
/// stack became is what the slot holds (FluidCannonBlockEntity's `itemHandler.setStack`). An emptied
/// stack clears the slot; a stack of another item replaces the slot whole; the same item keeps the
/// slot's damage facts, which an engine stack does not carry.
pub(crate) fn slot_write_back(slot: &mut Option<ItemSlot>, stack: &Stack) {
    if stack.is_empty() {
        *slot = None;
        return;
    }
    let custom_data = stack.nbt.as_ref().and_then(value_to_nbt).unwrap_or_default();
    match slot {
        Some(existing) if existing.item == stack.item => {
            existing.count = stack.count.max(0) as u32;
            existing.custom_data = custom_data;
        }
        _ => {
            let mut fresh = ItemSlot::new(stack.item.clone(), stack.count.max(0) as u32);
            fresh.custom_data = custom_data;
            *slot = Some(fresh);
        }
    }
}

/// A saved-data compound as a JSON value, the shape engine stacks carry NBT in. Numbers keep their
/// width where JSON can hold it; a float widens to a double, as JSON has no float.
pub(crate) fn nbt_to_value(tag: &NbtCompound) -> Value {
    Value::Object(tag.iter().map(|(key, value)| (key.clone(), nbt_value(value))).collect())
}

fn nbt_value(nbt: &Nbt) -> Value {
    match nbt {
        Nbt::Byte(value) => Value::from(*value),
        Nbt::Short(value) => Value::from(*value),
        Nbt::Int(value) => Value::from(*value),
        Nbt::Long(value) => Value::from(*value),
        Nbt::Float(value) => Value::from(*value as f64),
        Nbt::Double(value) => Value::from(*value),
        Nbt::ByteArray(values) => Value::Array(values.iter().map(|value| Value::from(*value)).collect()),
        Nbt::String(value) => Value::String(value.clone()),
        Nbt::List(values) => Value::Array(values.iter().map(nbt_value).collect()),
        Nbt::Compound(tag) => nbt_to_value(tag),
        Nbt::IntArray(values) => Value::Array(values.iter().map(|value| Value::from(*value)).collect()),
        Nbt::LongArray(values) => Value::Array(values.iter().map(|value| Value::from(*value)).collect()),
    }
}

/// A JSON value back to saved data; None for a value with no saved-data shape (null, a boolean, a
/// number JSON holds only approximately). Object keys ride along in order.
pub(crate) fn value_to_nbt(value: &Value) -> Option<NbtCompound> {
    let fields = value.as_object()?;
    let mut tag = NbtCompound::new();
    for (key, value) in fields {
        tag.insert(key.clone(), value_to_nbt_value(value)?);
    }
    Some(tag)
}

fn value_to_nbt_value(value: &Value) -> Option<Nbt> {
    Some(match value {
        Value::String(text) => Nbt::String(text.clone()),
        Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                if i32::try_from(value).is_ok() { Nbt::Int(value as i32) } else { Nbt::Long(value) }
            } else if let Some(value) = number.as_u64() {
                if value <= i32::MAX as u64 { Nbt::Int(value as i32) } else if value <= i64::MAX as u64 { Nbt::Long(value as i64) } else { return None }
            } else {
                Nbt::Double(number.as_f64()?)
            }
        }
        Value::Array(values) => Nbt::List(values.iter().map(value_to_nbt_value).collect::<Option<Vec<_>>>()?),
        Value::Object(_) => Nbt::Compound(value_to_nbt(value)?),
        _ => return None,
    })
}

/// The holder of a fluid application with no entity behind it: the cannon's shot and a projectile's
/// hit, where the original's context names a projectile (for the hit) and a fallback stack, and no
/// player, so nothing is build-restricted (FluidEffectContext with a null user).
pub(crate) fn holder(stack: Stack, projectile: Option<(i32, [f64; 3])>, position: [f64; 3]) -> Holder {
    Holder {
        stack,
        projectile: projectile.map(|(id, delta)| crate::fluid_effects::ProjectileFacts { id, delta }),
        position,
        ..Holder::default()
    }
}

/// The live level: PlutoLib's world of one dimension, plus the registry snapshots the facts come from.
pub(crate) struct LiveWorld {
    world: World,
    items: Option<Arc<RegistryLookup>>,
    entity_types: Option<Arc<RegistryLookup>>,
    fluids: Option<Arc<RegistryLookup>>,
    targets: Vec<LiveTarget<'static>>,
}

impl LiveWorld {
    pub(crate) fn new(world: World) -> Self {
        Self { world, items: common::item_snapshot(), entity_types: common::entity_type_snapshot(), fluids: common::fluid_snapshot(), targets: Vec::new() }
    }

    /// The world handle itself, for the operations outside the engine's vocabulary (spawning a
    /// projectile, reading a block entity's saved data).
    pub(crate) fn world(&self) -> &World { &self.world }

    fn state_at(&self, pos: BlockPos) -> Option<BlockState> { self.world.get_block_state(pos) }

    /// The item entity data of a dropped stack, in the game's saved layout (an `Item` compound of
    /// id, count and components, as an item slot describes them).
    fn drop(&self, at: [f64; 3], stack: &Stack) {
        if stack.is_empty() { return; }
        let mut item = NbtCompound::new();
        item.insert("id".to_owned(), Nbt::String(stack.item.to_string()));
        item.insert("count".to_owned(), Nbt::Int(stack.count));
        if let Some(nbt) = stack.nbt.as_ref().and_then(value_to_nbt) {
            if !nbt.is_empty() {
                let mut components = NbtCompound::new();
                components.insert("minecraft:custom_data".to_owned(), Nbt::Compound(nbt));
                item.insert("components".to_owned(), Nbt::Compound(components));
            }
        }
        let mut data = NbtCompound::new();
        data.insert("Item".to_owned(), Nbt::Compound(item));
        self.world.summon("minecraft:item", at, &data);
    }
}

/// Blocks a fluid may flow into or an effect may replace without a tool: air, the liquids, and the
/// vanilla vegetation and snow layer a placement pushes aside (BlockState.canBeReplaced's common set;
/// the game's own answer is not a PlutoLib query, so the set is the documented approximation).
const REPLACEABLE: [&str; 12] = [
    "minecraft:air", "minecraft:cave_air", "minecraft:void_air", "minecraft:water", "minecraft:lava",
    "minecraft:short_grass", "minecraft:tall_grass", "minecraft:fern", "minecraft:large_fern", "minecraft:snow",
    "minecraft:vine", "minecraft:glow_lichen",
];

/// The copper weathering chain (WeatheringCopper): each block that weathers, and the block it becomes;
/// the last of each family (the oxidized forms) and every waxed form do not weather.
const WEATHERING: [(&str, &str); 12] = [
    ("minecraft:copper_block", "minecraft:exposed_copper_block"),
    ("minecraft:exposed_copper_block", "minecraft:weathered_copper_block"),
    ("minecraft:weathered_copper_block", "minecraft:oxidized_copper_block"),
    ("minecraft:cut_copper", "minecraft:exposed_cut_copper"),
    ("minecraft:exposed_cut_copper", "minecraft:weathered_cut_copper"),
    ("minecraft:weathered_cut_copper", "minecraft:oxidized_cut_copper"),
    ("minecraft:cut_copper_stairs", "minecraft:exposed_cut_copper_stairs"),
    ("minecraft:exposed_cut_copper_stairs", "minecraft:weathered_cut_copper_stairs"),
    ("minecraft:weathered_cut_copper_stairs", "minecraft:oxidized_cut_copper_stairs"),
    ("minecraft:cut_copper_slab", "minecraft:exposed_cut_copper_slab"),
    ("minecraft:exposed_cut_copper_slab", "minecraft:weathered_cut_copper_slab"),
    ("minecraft:weathered_cut_copper_slab", "minecraft:oxidized_cut_copper_slab"),
];

impl WorldAccess for LiveWorld {
    fn block_state(&self, pos: BlockPos) -> Option<BlockState> { self.state_at(pos) }

    fn set_block(&mut self, pos: BlockPos, state: &BlockState) -> bool { self.world.set_block_state(pos, state) }

    fn destroy_particles(&mut self, _pos: BlockPos, _state: &BlockState) {
        // No particle emission surface; the block still goes away by the caller's own set.
    }

    fn play_sound(&mut self, at: [f64; 3], sound: &str, volume: f32, pitch: f32) {
        self.world.play_sound(sound, at, volume, pitch);
    }

    fn drop_item(&mut self, at: [f64; 3], stack: &Stack) { self.drop(at, stack); }

    fn explode(&mut self, _spec: &ExplosionSpec) {
        // No explosion operation exists in PlutoLib's world surface; see the module note.
    }

    fn spawn_cloud(&mut self, spec: &CloudSpec) {
        // An area effect cloud in the game's saved layout, with the lifetime the original sets up
        // (CloudSpec: radius 1, radius on use -0.5, wait 10, duration 600, radius per tick -1/600).
        let mut data = NbtCompound::new();
        data.insert("Radius".to_owned(), Nbt::Float(1.0));
        data.insert("RadiusOnUse".to_owned(), Nbt::Float(-0.5));
        data.insert("RadiusPerTick".to_owned(), Nbt::Float(-1.0 / 600.0));
        data.insert("WaitTime".to_owned(), Nbt::Int(10));
        data.insert("Duration".to_owned(), Nbt::Int(600));
        // The cloud's owner is an entity id here and a uuid in saved data; no id-to-uuid reach
        // exists, so the cloud flies ownerless (its effects do not credit a source).
        if !spec.effects.is_empty() {
            let effects: Vec<Nbt> = spec.effects.iter().map(|effect| {
                let mut entry = NbtCompound::new();
                entry.insert("id".to_owned(), Nbt::String(effect.effect.to_string()));
                entry.insert("amplifier".to_owned(), Nbt::Byte(effect.amplifier.clamp(0, 127) as i8));
                entry.insert("duration".to_owned(), Nbt::Int(effect.duration));
                entry.insert("ambient".to_owned(), Nbt::Byte(0));
                entry.insert("show_particles".to_owned(), Nbt::Byte(1));
                entry.insert("show_icon".to_owned(), Nbt::Byte(1));
                Nbt::Compound(entry)
            }).collect();
            data.insert("Effects".to_owned(), Nbt::List(effects));
        }
        self.world.summon("minecraft:area_effect_cloud", spec.at, &data);
    }

    fn break_block(&mut self, spec: &BreakSpec) -> bool {
        let Some(state) = self.state_at(spec.pos) else { return false };
        if state.block.to_string() == "minecraft:air" { return false; }
        // The fake tool's drops, approximated by the block's item form at the hit location; the
        // tier gate was already answered by correct_tier_for_drops where the effect asks it.
        if let Some(item) = self.block_item(&state.block) {
            self.drop(spec.location, &Stack::of(item, 1));
        }
        let replacement = spec.replacement.clone().unwrap_or_else(|| BlockState::new(Identifier::new("minecraft", "air").expect("valid id")));
        self.world.set_block_state(spec.pos, &replacement)
    }

    fn destroy_block(&mut self, pos: BlockPos) {
        if let Some(state) = self.state_at(pos) {
            if let Some(item) = self.block_item(&state.block) {
                self.drop([pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5], &Stack::of(item, 1));
            }
        }
        self.world.set_block_state(pos, &BlockState::new(Identifier::new("minecraft", "air").expect("valid id")));
    }

    fn move_row(&mut self, _pos: BlockPos, _facing: Direction, _direction: Direction, _moving: i32, _push: bool) {
        // Nothing is pushable through this context (is_pushable), so the effect never gets here in
        // practice; there is no piston-move surface to call if it did.
    }

    fn push_reaction(&self, _pos: BlockPos) -> PushReaction { PushReaction::Block }

    fn has_block_entity(&self, pos: BlockPos) -> bool { self.world.block_entity_data(pos).is_some() }

    fn is_pushable(&self, _pos: BlockPos, _direction: Direction, _allow_destroy: bool, _facing: Direction) -> bool { false }

    fn destroy_speed(&self, pos: BlockPos) -> f32 {
        match self.state_at(pos) {
            None => -1.0,
            Some(state) => match state.block.to_string().as_str() {
                "minecraft:air" | "minecraft:bedrock" | "minecraft:barrier" | "minecraft:end_portal_frame" => -1.0,
                _ => 1.0,
            },
        }
    }

    fn is_liquid_block(&self, pos: BlockPos) -> bool {
        self.state_at(pos).map_or(false, |state| {
            matches!(state.block.to_string().as_str(), "minecraft:water" | "minecraft:lava") || FluidWorld::is_world_fluid(&state.block)
        })
    }

    fn can_be_replaced(&self, pos: BlockPos) -> bool {
        self.state_at(pos).map_or(false, |state| {
            REPLACEABLE.contains(&state.block.to_string().as_str()) || FluidWorld::is_world_fluid(&state.block)
        })
    }

    fn within_world_border(&self, _pos: BlockPos) -> bool { true }

    fn place_block(&mut self, spec: &mut PlaceSpec) -> InteractionResult {
        let Some(block) = spec.block.clone() else { return InteractionResult::Pass };
        if !self.can_be_replaced(spec.pos) { return InteractionResult::Pass; }
        if !self.world.set_block_state(spec.pos, &block) { return InteractionResult::Pass; }
        if !spec.holder.creative && !spec.stack.is_empty() {
            spec.stack.count -= 1;
        }
        if let Some(sound) = &spec.sound {
            self.world.play_sound(sound, spec.location, 1.0, 1.0);
        }
        InteractionResult::Success
    }

    fn interact_block(&mut self, _spec: &mut InteractSpec) -> InteractionResult { InteractionResult::Pass }

    fn interact_entity(&mut self, _spec: &mut EntityInteractSpec) -> InteractionResult { InteractionResult::Pass }

    fn is_weathering_copper(&self, pos: BlockPos) -> bool {
        self.state_at(pos).map_or(false, |state| WEATHERING.iter().any(|(from, _)| *from == state.block.to_string()))
    }

    fn apply_weathering(&mut self, pos: BlockPos) {
        let Some(state) = self.state_at(pos) else { return };
        let Some((_, next)) = WEATHERING.iter().find(|(from, _)| *from == state.block.to_string()) else { return };
        let Ok(next) = Identifier::parse(next) else { return };
        let mut weathered = BlockState::new(next);
        weathered.properties = state.properties.clone();
        self.world.set_block_state(pos, &weathered);
    }

    fn melting_result(&self, item: &Identifier, temperature: i32) -> Option<FluidStack> {
        let data = common::smeltery_data()?;
        let data = data.read().ok()?;
        let items = self.items.as_deref();
        let in_tag = |id: &Identifier, tag: &Identifier| items.map_or(false, |items| items.get_tag(tag).map_or(false, |members| members.contains(id)));
        let melting = data.recipes.find_melting(item, &in_tag, false)?;
        if melting.temperature > temperature { return None; }
        let fluid = match &melting.output.source {
            Source::Id(fluid) => fluid.clone(),
            Source::Tag(tag) => self.fluids.as_deref()?.get_tag(tag)?.iter().next()?.clone(),
        };
        Some(FluidStack { fluid, amount: melting.output.amount.max(0) as u32, tag: melting.output.nbt.as_ref().and_then(value_to_nbt) })
    }

    fn fluid_block(&self, fluid: &Identifier) -> Option<BlockState> {
        match fluid.to_string().as_str() {
            "minecraft:water" => return Some(BlockState::new(fluid.clone())),
            "minecraft:lava" => return Some(BlockState::new(fluid.clone())),
            _ => {}
        }
        let definition = FluidRegistry::get(&fluid.to_string())?;
        if !definition.world_blocks { return None; }
        Some(FluidWorld::state(fluid, 0))
    }

    fn flowing_fluid_block(&self, fluid: &Identifier, level: i32) -> Option<BlockState> {
        let definition = FluidRegistry::get(&fluid.to_string())?;
        if !definition.world_blocks { return None; }
        Some(FluidWorld::state(fluid, level.clamp(0, 8) as u8))
    }

    fn block_item(&self, block: &Identifier) -> Option<Identifier> {
        self.items.as_deref().filter(|items| items.contains(block)).map(|_| block.clone())
    }

    fn item_provided_block(&self, _item: &Identifier) -> Option<BlockState> { None }

    fn correct_tier_for_drops(&self, _tier: &str, _pos: BlockPos) -> bool { false }

    fn named_block_predicate(&self, name: &str, pos: BlockPos, _state: &BlockState) -> Option<bool> {
        match name {
            "mantle:can_be_replaced" => Some(self.can_be_replaced(pos)),
            _ => None,
        }
    }

    fn adventure_place_allowed(&self, _stack: &Stack, _pos: BlockPos) -> bool { false }

    fn preferred_item(&self, output: &ItemOutput) -> Option<Stack> {
        let item = match &output.source {
            Source::Id(item) => item.clone(),
            Source::Tag(tag) => self.items.as_deref()?.get_tag(tag)?.iter().next()?.clone(),
        };
        Some(Stack { item, count: output.count, nbt: output.nbt.clone() })
    }

    fn potion_effects(&self, _tag: Option<&NbtCompound>) -> Vec<PotionEffect> {
        // No potion contents reach; see the module note.
        Vec::new()
    }

    fn living_in_block(&mut self, pos: BlockPos) -> Vec<&mut dyn EntityTarget> {
        let min = [pos[0] as f64, pos[1] as f64, pos[2] as f64];
        let max = [min[0] + 1.0, min[1] + 1.0, min[2] + 1.0];
        self.targets.clear();
        for entity in EntityReach::in_box(min, max) {
            if entity.living.is_some() {
                self.targets.push(LiveTarget::area(&entity, [min[0] + 0.5, min[1] + 0.5, min[2] + 0.5]));
            }
        }
        self.targets.iter_mut().map(|target| target as &mut dyn EntityTarget).collect()
    }

    fn random_float(&mut self) -> f32 { self.world.random_float().unwrap_or(0.0) }
}

/// Where a target's given effects go: the directly hit entity of a projectile impact queues them on
/// the impact itself, which PlutoLib does to the hit's parties afterwards; any other target has no
/// such queue, and only its hurts (through the entity reach) land.
enum Sink<'a> {
    Victim(&'a RefCell<Vec<EntityAction>>),
    Reach,
}

/// A live entity as an effect target: the facts the reaches answer, and conservative defaults for
/// the ones they cannot (see the module note).
pub(crate) struct LiveTarget<'a> {
    id: i32,
    kind: String,
    living: bool,
    player: bool,
    fire_immune: bool,
    position: [f64; 3],
    sink: Sink<'a>,
}

impl<'a> LiveTarget<'a> {
    /// The entity a projectile hit: its kind and the hit point come from the impact, its living facts
    /// from the entity reach around the hit point when the reach can see it (a kind the reach cannot
    /// see is alive unless it is one of the kinds that plainly is not), and its given effects queue on
    /// the impact's actions as the victim's.
    pub(crate) fn victim(hit: &ProjectileImpact, actions: &'a RefCell<Vec<EntityAction>>) -> Self {
        let kind = hit.entity_kind.clone();
        let (mut living, mut fire_immune) = (!matches!(kind.as_str(), "minecraft:item" | "minecraft:experience_orb" | "minecraft:falling_block"), false);
        if let Some(id) = hit.entity {
            let at = hit.at;
            for entity in EntityReach::in_box([at[0] - 0.5, at[1] - 0.5, at[2] - 0.5], [at[0] + 0.5, at[1] + 0.5, at[2] + 0.5]) {
                if entity.id == id as u64 {
                    living = entity.living.is_some();
                    fire_immune = entity.living.map_or(false, |facts| facts.fire_immune);
                    break;
                }
            }
        }
        Self {
            id: hit.entity.map_or(0, |id| id as i32),
            player: kind == "minecraft:player",
            kind,
            living,
            fire_immune,
            position: hit.at,
            sink: Sink::Victim(actions),
        }
    }

    /// A living entity an area effect found in a block's cell: facts from the reach, position the
    /// cell's centre (the reach does not hand out positions), hurts only.
    pub(crate) fn area(entity: &WorldEntity, position: [f64; 3]) -> Self {
        Self {
            id: entity.id as i32,
            kind: entity.kind.to_string(),
            living: entity.living.is_some(),
            player: entity.kind.to_string() == "minecraft:player",
            fire_immune: entity.living.map_or(false, |facts| facts.fire_immune),
            position,
            sink: Sink::Reach,
        }
    }

    fn queue(&mut self, action: EntityAction) -> bool {
        match &self.sink {
            Sink::Victim(actions) => {
                actions.borrow_mut().push(action);
                true
            }
            Sink::Reach => false,
        }
    }
}

impl EntityTarget for LiveTarget<'_> {
    fn id(&self) -> i32 { self.id }
    fn kind(&self) -> &str { &self.kind }
    fn is_living(&self) -> bool { self.living }
    fn is_player(&self) -> bool { self.player }
    fn fire_immune(&self) -> bool { self.fire_immune }
    fn named_predicate(&self, name: &str) -> Option<bool> {
        match name {
            "mantle:fire_immune" => Some(self.fire_immune),
            _ => None,
        }
    }
    fn in_type_tag(&self, tag: &Identifier) -> bool {
        common::entity_type_snapshot().map_or(false, |types| {
            types.get_tag(tag).map_or(false, |members| members.iter().any(|member| member.to_string() == self.kind))
        })
    }
    fn active_effect(&self, _effect: &Identifier) -> Option<ActiveEffect> { None }
    fn can_be_affected(&self, _effect: &NewEffect) -> bool { true }
    fn any_curative_effect(&self, _item: &Stack) -> bool { false }
    fn remaining_fire_ticks(&self) -> i32 { 0 }
    fn ticks_frozen(&self) -> i32 { 0 }
    fn ticks_required_to_freeze(&self) -> i32 { 140 }
    fn can_freeze(&self) -> bool { self.living }
    fn air_supply(&self) -> i32 { 300 }
    fn max_air_supply(&self) -> i32 { 300 }
    fn can_eat(&self, _always_eat: bool) -> bool { false }
    fn position(&self) -> [f64; 3] { self.position }
    fn look_angle(&self) -> [f64; 3] { [0.0, 0.0, 1.0] }
    fn block_position(&self) -> BlockPos {
        [self.position[0].floor() as i32, self.position[1].floor() as i32, self.position[2].floor() as i32]
    }
    fn strong_bones_helmet(&self) -> Option<(i32, Identifier)> { None }
    fn calcifiable_level(&self) -> i32 { 0 }
    fn set_remaining_fire_ticks(&mut self, ticks: i32) {
        // The actions vocabulary raises a burn by whole seconds and never lowers one; a request at or
        // below the (unreadable) current burn is left to that rule, as setSecondsOnFire itself is.
        if ticks > 0 {
            let seconds = (ticks + 19) / 20;
            self.queue(EntityAction::SetFire { target: ActionTarget::Victim, seconds });
        }
    }
    fn set_ticks_frozen(&mut self, ticks: i32) {
        if ticks > 0 {
            self.queue(EntityAction::Freeze { target: ActionTarget::Victim, ticks });
        }
    }
    fn add_effect(&mut self, effect: NewEffect) -> bool {
        self.queue(EntityAction::AddEffect {
            target: ActionTarget::Victim,
            effect: effect.effect.to_string(),
            amplifier: effect.amplifier,
            duration: effect.duration,
        })
    }
    fn remove_effect(&mut self, _effect: &Identifier) -> bool { false }
    fn cure_effects(&mut self, _item: &Stack) -> bool { false }
    fn hurt(&mut self, amount: f32, damage_type: &str) -> bool {
        if self.id <= 0 { return false; }
        EntityReach::hurt(self.id as u64, amount, damage_type)
    }
    fn eat(&mut self, _hunger: i32, _saturation: f32) {}
    fn food_eaten(&mut self, _representative: &Stack, _hunger: i32, _saturation: f32) {}
    fn award_stat(&mut self, _stat: &Identifier, _amount: i32) {}
    fn set_air_supply(&mut self, _air: i32) {}
    fn push(&mut self, _delta: [f64; 3]) {
        // No push-by-vector surface; see the module note.
    }
    fn random_teleport(&mut self, _diameter: i32, _chances: i32) {}
}

/// A block context over a live world, for the cannon's direct application and a projectile's block
/// hit: the holder names the fallback stack and, for a hit, the projectile carrying the fluid.
pub(crate) fn block_context<'a>(world: &'a mut LiveWorld, holder: Holder, pos: BlockPos, direction: Direction, location: [f64; 3]) -> BlockContext<'a> {
    BlockContext { env: crate::fluid_effects::Env { world, holder }, pos, direction, location }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{with_entities, MemoryEntityOps};

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }

    #[test]
    fn stacks_round_trip_between_slots_and_the_engine() {
        let mut slot = ItemSlot::new(id("minecraft:bucket"), 3);
        let mut data = NbtCompound::new();
        data.insert("Potion".to_owned(), Nbt::String("minecraft:poison".to_owned()));
        slot.custom_data = data;
        let stack = stack_of_slot(&slot);
        assert_eq!((stack.item.to_string().as_str(), stack.count), ("minecraft:bucket", 3));
        assert_eq!(stack.nbt.as_ref().and_then(|nbt| nbt.get("Potion")).and_then(|value| value.as_str()), Some("minecraft:poison"));
        // the same item keeps its slot (and its damage facts); a changed count and NBT land
        let mut changed = stack.clone();
        changed.count = 1;
        let mut slot = Some(slot);
        slot_write_back(&mut slot, &changed);
        assert_eq!(slot.as_ref().map(|slot| (slot.count, slot.custom_data.get_string("Potion").map(str::to_owned))), Some((1, Some("minecraft:poison".to_owned()))));
        // an emptied stack clears the slot, another item replaces it whole
        slot_write_back(&mut slot, &Stack::empty());
        assert!(slot.is_none());
        slot_write_back(&mut slot, &Stack::of(id("minecraft:stick"), 2));
        assert_eq!(slot.as_ref().map(|slot| (slot.item.to_string(), slot.count)), Some(("minecraft:stick".to_owned(), 2)));
    }

    #[test]
    fn a_victim_queues_its_given_effects_on_the_impact_and_hurts_through_the_reach() {
        let hit = ProjectileImpact {
            dimension: "minecraft:overworld".to_owned(),
            projectile: 7,
            kind: "tconstruct:fluid_spit".to_owned(),
            owner: -1,
            entity: Some(1),
            entity_kind: "minecraft:zombie".to_owned(),
            block: [0, 0, 0],
            face: [0, 1, 0],
            at: [0.5, 0.5, 0.5],
            actions: RefCell::new(Vec::new()),
        };
        let actions = RefCell::new(Vec::new());
        let mut ops = MemoryEntityOps {
            entities: vec![([0.5, 0.5, 0.5], WorldEntity {
                id: 1,
                kind: id("minecraft:zombie"),
                item: None,
                living: Some(pluto_lib::LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
            })],
            ..MemoryEntityOps::default()
        };
        with_entities(&mut ops, || {
            let mut target = LiveTarget::victim(&hit, &actions);
            assert!(target.is_living() && !target.is_player());
            assert!(target.add_effect(NewEffect { effect: id("minecraft:poison"), duration: 100, amplifier: 1, curative_items: None }));
            target.set_remaining_fire_ticks(41);
            assert!(target.hurt(2.0, "minecraft:magic"));
        });
        let queued = actions.borrow();
        assert_eq!(queued.len(), 2, "the effect and the burn queue; the hurt went through the reach instead");
        assert!(matches!(queued[0], EntityAction::AddEffect { amplifier: 1, duration: 100, .. }));
        assert!(matches!(queued[1], EntityAction::SetFire { seconds: 3, .. }), "41 ticks round up to 3 seconds");
        assert_eq!(ops.hurt.len(), 1);
    }
}
