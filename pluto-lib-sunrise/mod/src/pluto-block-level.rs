// PlutoLib's block behaviours inside the game: the one place that answers `canSurvive`, `getStateForPlacement` and `updateShape` for
// blocks that have a behaviour registered through the API crate (block_api_v1). Other blocks are left to the game and to other hooks.
use super::*;
use sunrise_core::geometry::Direction;
use sunrise_core::random::RandomSource;
use crate::pluto_block::{self, LevelEdit, LevelView, Placement, PlacementAnswer, ShapeUpdate, StateInfo};

struct View<'a, 'g> {
    level: &'a GameLevel<'g>,
}

fn at(pos: [i32; 3]) -> BlockPos {
    BlockPos::new(pos[0], pos[1], pos[2])
}

fn info(level: &GameLevel<'_>, id: BlockStateId) -> Option<StateInfo> {
    let state = level.registry.state(id)?;
    Some(StateInfo {
        block: state.name.to_string(),
        properties: state.properties.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    })
}

fn resolve(level: &GameLevel<'_>, wanted: &StateInfo) -> Option<BlockStateId> {
    let base = level.registry.default_state(&wanted.block)?;
    let mut state = level.registry.state(base)?.clone();
    for (key, value) in &wanted.properties {
        state.properties.insert(key.as_str().into(), value.as_str().into());
    }
    level.registry.resolve(&state).ok()
}

impl LevelView for View<'_, '_> {
    fn state(&self, pos: [i32; 3]) -> Option<StateInfo> {
        info(self.level, self.level.block_state(at(pos)))
    }

    fn is_air(&self, pos: [i32; 3]) -> bool {
        self.level.behaviours.is_air(self.level.block_state(at(pos)))
    }

    fn replaceable(&self, pos: [i32; 3]) -> bool {
        self.level.behaviours.is_replaceable(self.level.block_state(at(pos)))
    }

    fn face_sturdy(&self, pos: [i32; 3], face: u8) -> bool {
        face < 6 && self.level.behaviours.shape_info(self.level.block_state(at(pos))).full_faces & (1 << face) != 0
    }

    fn in_tag(&self, pos: [i32; 3], tag: &str) -> bool {
        let behaviours = self.level.behaviours;
        let block = behaviours.block(self.level.block_state(at(pos)));
        behaviours.tag(tag).is_some_and(|set| set.contains(block))
    }

    fn state_in_tag(&self, state: &StateInfo, tag: &str) -> Option<bool> {
        let state = resolve(self.level, state)?;
        let behaviours = self.level.behaviours;
        Some(behaviours.tag(tag).is_some_and(|set| set.contains(behaviours.block(state))))
    }

    fn brightness(&self, pos: [i32; 3]) -> u8 {
        sunrise_world::level::raw_brightness(self.level, at(pos), self.level.sky_darken()).clamp(0, 15) as u8
    }

    fn light_passes_up(&self, pos: [i32; 3]) -> bool {
        use sunrise_world::level::data::state_flags::{CAN_OCCLUDE, SHAPE_OCCLUSION};
        let behaviours = self.level.behaviours;
        let state = self.level.block_state(at(pos));
        let above = self.level.block_state(at([pos[0], pos[1] + 1, pos[2]]));
        let occludes = |s| behaviours.facts(s).flags & (CAN_OCCLUDE | SHAPE_OCCLUSION) == (CAN_OCCLUDE | SHAPE_OCCLUSION);
        let from_full = occludes(state) && behaviours.shape_info(state).full_faces & (1 << Direction::Up as u8) != 0;
        let to_full = occludes(above) && behaviours.shape_info(above).full_faces & (1 << Direction::Down as u8) != 0;
        let attenuation = if from_full || to_full { 16 } else { i32::from(behaviours.facts(above).dampening) };
        attenuation < 15
    }

    fn is_water(&self, pos: [i32; 3]) -> bool {
        self.state(pos).is_some_and(|s| s.block == "minecraft:water" || s.properties.get("waterlogged").is_some_and(|v| v == "true"))
    }

    fn fluid_full(&self, pos: [i32; 3]) -> bool {
        self.state(pos).is_some_and(|s| {
            s.properties.get("waterlogged").is_some_and(|v| v == "true")
                || (matches!(s.block.as_str(), "minecraft:water" | "minecraft:lava") && s.properties.get("level").is_some_and(|v| v == "0"))
        })
    }

    fn area_loaded(&self, pos: [i32; 3], radius: i32) -> bool {
        [-radius, radius].iter().all(|dx| [-radius, radius].iter().all(|dz| self.level.world.loaded(pos[0] + dx, pos[2] + dz)))
    }

    fn full_block(&self, pos: [i32; 3]) -> bool {
        self.level.behaviours.shape_info(self.level.block_state(at(pos))).full_faces & 0x3f == 0x3f
    }

    fn build_height(&self) -> [i32; 2] {
        [self.level.world.bottom_y(), self.level.world.top_y()]
    }
}

/// A borrowed client callback view. Native signal getters require mutable
/// LevelAccess/Ticks even though the query API is read-only; scoped interior
/// borrows let the real native implementation answer without copying circuits.
/// The client ticker itself runs without a held level/tick borrow. Individual
/// native queries retain their own native callback semantics.
struct ClientView<'a, 'g> {
    level: std::cell::RefCell<&'a mut GameLevel<'g>>,
    ticks: std::cell::RefCell<&'a mut Ticks>,
}
impl ClientView<'_, '_> {
    fn read<R>(&self, f: impl FnOnce(&dyn LevelView) -> R) -> R {
        let level = self.level.borrow();
        f(&View { level: &**level })
    }
}
impl LevelView for ClientView<'_, '_> {
    fn state(&self, pos: [i32; 3]) -> Option<StateInfo> {
        self.read(|view| if view.area_loaded(pos, 0) { view.state(pos) } else { None })
    }
    fn is_air(&self, pos: [i32; 3]) -> bool { self.read(|v| v.is_air(pos)) }
    fn replaceable(&self, pos: [i32; 3]) -> bool { self.read(|v| v.replaceable(pos)) }
    fn has_neighbor_signal(&self, pos: [i32; 3]) -> Option<bool> {
        crate::pluto_block_access::query_neighbor_signal(pos, self.build_height(),
            |x, z| self.level.borrow().world.loaded(x, z), || {
                let mut level = self.level.borrow_mut();
                let mut ticks = self.ticks.borrow_mut();
                let behaviours = level.behaviours;
                behaviours.has_neighbor_signal(&mut **level, &mut **ticks, at(pos))
            })
    }
    fn face_sturdy(&self, pos: [i32; 3], face: u8) -> bool { self.read(|v| v.face_sturdy(pos, face)) }
    fn in_tag(&self, pos: [i32; 3], tag: &str) -> bool { self.read(|v| v.in_tag(pos, tag)) }
    fn state_in_tag(&self, state: &StateInfo, tag: &str) -> Option<bool> { self.read(|v| v.state_in_tag(state, tag)) }
    fn brightness(&self, pos: [i32; 3]) -> u8 { self.read(|v| v.brightness(pos)) }
    fn light_passes_up(&self, pos: [i32; 3]) -> bool {
        pos[1].checked_add(1).is_some() && self.read(|v| v.light_passes_up(pos))
    }
    fn is_water(&self, pos: [i32; 3]) -> bool { self.read(|v| v.is_water(pos)) }
    fn fluid_full(&self, pos: [i32; 3]) -> bool { self.read(|v| v.fluid_full(pos)) }
    fn area_loaded(&self, pos: [i32; 3], radius: i32) -> bool {
        if radius < 0 { return false; }
        let Some((x0, x1)) = pos[0].checked_sub(radius).zip(pos[0].checked_add(radius)) else { return false };
        let Some((z0, z1)) = pos[2].checked_sub(radius).zip(pos[2].checked_add(radius)) else { return false };
        let level = self.level.borrow();
        // Check every covered chunk, not just four corners of a large radius.
        (x0.div_euclid(16)..=x1.div_euclid(16)).all(|x|
            (z0.div_euclid(16)..=z1.div_euclid(16)).all(|z|
                level.world.loaded(x * 16, z * 16)))
    }
    fn full_block(&self, pos: [i32; 3]) -> bool { self.read(|v| v.full_block(pos)) }
    fn build_height(&self) -> [i32; 2] { self.read(|v| v.build_height()) }
}

/// Source-wired native client view entry. The client animation-pass driver calls
/// it after obtaining an owned transient-state update lease; native-unvalidated.
pub(super) fn with_client_view<R>(level: &mut GameLevel<'_>, ticks: &mut Ticks,
    body: impl FnOnce(&dyn LevelView) -> R) -> R {
    body(&ClientView { level: std::cell::RefCell::new(level), ticks: std::cell::RefCell::new(ticks) })
}

pub(super) fn survives(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, survives: &mut Option<bool>) {
    let Some(sink) = pluto_block::sink() else { return };
    let Some(wanted) = info(level, state) else { return };
    if !(sink.has)(&wanted.block) {
        return;
    }
    if let Some(answer) = (sink.survives)(&View { level }, &wanted, [pos.x, pos.y, pos.z]) {
        *survives = Some(answer);
    }
}

pub(super) fn placement(level: &GameLevel<'_>, state: BlockStateId, context: &PlacementContext, answer: &mut Option<Option<BlockStateId>>) {
    let Some(sink) = pluto_block::sink() else { return };
    let Some(default) = info(level, state) else { return };
    if !(sink.has)(&default.block) {
        return;
    }
    let here = info(level, level.block_state(context.pos));
    let in_water = here.is_some_and(|s| s.block == "minecraft:water" || s.properties.get("waterlogged").is_some_and(|v| v == "true"));
    let request = Placement {
        state: default,
        pos: [context.pos.x, context.pos.y, context.pos.z],
        face: context.face as u8,
        yaw: context.yaw,
        pitch: context.pitch,
        sneaking: context.sneaking,
        in_water,
        item: context.item.clone(),
        replace_clicked: context.replace_clicked,
    };
    match (sink.placement)(&View { level }, &request) {
        PlacementAnswer::Default => {}
        PlacementAnswer::Refuse => *answer = Some(None),
        PlacementAnswer::State(chosen) => {
            if let Some(id) = resolve(level, &chosen) {
                *answer = Some(Some(id));
            }
        }
    }
}

pub(super) fn update_shape(level: &GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, direction: Direction, neighbour_pos: BlockPos, neighbour: BlockStateId, new: &mut Option<BlockStateId>) {
    let Some(sink) = pluto_block::sink() else { return };
    let Some(current) = info(level, state) else { return };
    if !(sink.has)(&current.block) {
        return;
    }
    let update = ShapeUpdate {
        state: current,
        pos: [pos.x, pos.y, pos.z],
        direction: direction as u8,
        neighbour_pos: [neighbour_pos.x, neighbour_pos.y, neighbour_pos.z],
        neighbour: info(level, neighbour),
        tick_delay: std::cell::Cell::new(None),
    };
    let changed = (sink.update_shape)(&View { level }, &update);
    if let Some(delay) = update.tick_delay.get().filter(|delay| *delay > 0) {
        level.native_schedule_block_tick(ticks, update.pos, &update.state.block, delay);
    }
    if let Some(changed) = changed {
        // the same state counts as an answer too: the game's own handling of this update is then skipped
        if let Some(id) = resolve(level, &changed) {
            *new = Some(id);
        }
    }
}

/// A view that can write: the level with the hook's tick queue (a hook without one cannot write blocks).
struct Edit<'a, 'g> {
    level: &'a mut GameLevel<'g>,
    ticks: Option<&'a mut Ticks>,
    feature_depth: u8,
}

impl LevelView for Edit<'_, '_> {
    fn state(&self, pos: [i32; 3]) -> Option<StateInfo> {
        View { level: &*self.level }.state(pos)
    }

    fn is_air(&self, pos: [i32; 3]) -> bool {
        View { level: &*self.level }.is_air(pos)
    }

    fn replaceable(&self, pos: [i32; 3]) -> bool {
        View { level: &*self.level }.replaceable(pos)
    }

    fn face_sturdy(&self, pos: [i32; 3], face: u8) -> bool {
        View { level: &*self.level }.face_sturdy(pos, face)
    }

    fn in_tag(&self, pos: [i32; 3], tag: &str) -> bool {
        View { level: &*self.level }.in_tag(pos, tag)
    }
    fn state_in_tag(&self, state: &StateInfo, tag: &str) -> Option<bool> {
        View { level: &*self.level }.state_in_tag(state, tag)
    }

    fn brightness(&self, pos: [i32; 3]) -> u8 {
        View { level: &*self.level }.brightness(pos)
    }

    fn light_passes_up(&self, pos: [i32; 3]) -> bool {
        View { level: &*self.level }.light_passes_up(pos)
    }

    fn is_water(&self, pos: [i32; 3]) -> bool {
        View { level: &*self.level }.is_water(pos)
    }

    fn fluid_full(&self, pos: [i32; 3]) -> bool {
        View { level: &*self.level }.fluid_full(pos)
    }

    fn area_loaded(&self, pos: [i32; 3], radius: i32) -> bool {
        View { level: &*self.level }.area_loaded(pos, radius)
    }

    fn full_block(&self, pos: [i32; 3]) -> bool {
        View { level: &*self.level }.full_block(pos)
    }

    fn build_height(&self) -> [i32; 2] {
        View { level: &*self.level }.build_height()
    }
}

impl LevelEdit for Edit<'_, '_> {
    fn set_state(&mut self, pos: [i32; 3], wanted: &StateInfo, flags: i32) -> bool {
        let Some(id) = resolve(&*self.level, wanted) else { return false };
        let Some(ticks) = self.ticks.as_deref_mut() else { return false };
        let behaviours = self.level.behaviours;
        behaviours.set_block(&mut *self.level, ticks, at(pos), id, flags)
    }

    fn break_block(&mut self, pos: [i32; 3], drop: bool) -> Option<bool> {
        let ticks = self.ticks.as_deref_mut()?;
        let behaviours = self.level.behaviours;
        Some(behaviours.destroy_block(&mut *self.level, ticks, at(pos), drop))
    }

    fn use_block(&mut self, pos: [i32; 3], face: u8, hit: [f64; 3], item: Option<&str>) -> Option<bool> {
        let ticks = self.ticks.as_deref_mut()?;
        let face = match face {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::North,
            3 => Direction::South,
            4 => Direction::West,
            _ => Direction::East,
        };
        // The use is re-dispatched as a survival player who may build: the identity of the
        // holder is not the level's to know, and the harvest uses it drives (a berry bush, a
        // cave vine) ask nothing more of the player.
        let player = sunrise_world::level::PlayerContext { yaw: 0.0, pitch: 0.0, may_build: true, can_eat: false, game_master: false };
        let behaviours = self.level.behaviours;
        let result = behaviours.use_item_on(&mut *self.level, ticks, at(pos), face, sunrise_core::geometry::Vec3::new(hit[0], hit[1], hit[2]), player, item);
        Some(result.consumes_action())
    }

    fn random_int(&mut self, bound: i32) -> i32 {
        if bound <= 0 {
            return 0;
        }
        self.level.world.random.next_int_bound(bound).unwrap_or(0)
    }

    fn place_feature(&mut self, feature: &str, pos: [i32; 3]) -> bool {
        if self.ticks.is_none() || self.feature_depth >= pluto_block::MAX_FEATURE_DEPTH { return false; }
        let behaviours = self.level.behaviours;
        if let Some(definition) = crate::pluto_worldgen_level::configured_json(behaviours, feature) {
            if let Some(kind) = definition.get("type").and_then(serde_json::Value::as_str) {
                let handled = pluto_block::with_feature_depth(self, |edit| &mut edit.feature_depth, |edit| {
                    pluto_block::custom_feature(edit, kind, definition.get("config").unwrap_or(&definition), pos)
                });
                if let Some(placed) = handled { return placed; }
            }
        }
        let Some(ticks) = self.ticks.as_deref_mut() else { return false };
        crate::pluto_worldgen_level::place_native(
            behaviours, &mut *self.level, ticks, feature, at(pos),
        ).unwrap_or(false)
    }

    fn place_tree_with_roots(&mut self, configuration: &serde_json::Value, pos: [i32; 3], bindings: &crate::pluto_block_access::RootBindings,
        root: &mut crate::pluto_block_access::RootCallback<'_>, decorate: &mut crate::pluto_block_access::TreeCallback<'_>) -> bool {
        pluto_block::with_feature_depth(self, |edit| &mut edit.feature_depth, |edit| {
            let Some(ticks) = edit.ticks.as_deref_mut() else { return Some(false) };
            let behaviours = edit.level.behaviours;
            Some(false)
        }).unwrap_or(false)
    }

    fn place_tree_with_plan(&mut self, configuration: &serde_json::Value, pos: [i32; 3], roots: Option<(&crate::pluto_block_access::RootBindings, &mut crate::pluto_block_access::RootCallback<'_>)>, plan: &[crate::pluto_block_access::TreeStep], decorate: &mut crate::pluto_block_access::PlannedTreeCallback<'_>) -> bool {
        pluto_block::with_feature_depth(self, |edit| &mut edit.feature_depth, |edit| {
            let Some(ticks) = edit.ticks.as_deref_mut() else { return Some(false) };
            Some(false)
        }).unwrap_or(false)
    }

    fn place_tree(&mut self, configuration: &serde_json::Value, pos: [i32; 3], decorate: &mut crate::pluto_block_access::TreeCallback<'_>) -> bool {
        pluto_block::with_feature_depth(self, |edit| &mut edit.feature_depth, |edit| {
            let Some(ticks) = edit.ticks.as_deref_mut() else { return Some(false) };
            let behaviours = edit.level.behaviours;
            Some(false)
        }).unwrap_or(false)
    }

    fn random_float(&mut self) -> f32 {
        self.level.world.random.next_float()
    }

    fn schedule_tick(&mut self, pos: [i32; 3], block: &str, delay: i32) -> bool {
        let Some(ticks) = self.ticks.as_deref_mut() else { return false };
        self.level.native_schedule_block_tick(ticks, pos, block, delay)
    }
}

pub(super) fn random_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    tick_with(level, ticks, pos, state, handled, |sink| sink.random_tick);
}

pub(super) fn scheduled_tick(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId, handled: &mut bool) {
    tick_with(level, ticks, pos, state, handled, |sink| sink.scheduled_tick);
}

fn tick_with(
    level: &mut GameLevel<'_>,
    ticks: &mut Ticks,
    pos: BlockPos,
    state: BlockStateId,
    handled: &mut bool,
    pick: fn(pluto_block::Sink) -> fn(&mut dyn LevelEdit, &StateInfo, [i32; 3]) -> bool,
) {
    // Apply the block ticks host-side callbacks queued for this level first: the mod-facing
    // host cannot schedule a tick, so a bucket-placed fluid's first tick (and every other
    // World::schedule_tick request) waits in the queue for a seam that holds the tick queue.
    {
        let dimension = level.dimension.key().id();
        let mut edit = Edit { level: &mut *level, ticks: Some(&mut *ticks), feature_depth: 0 };
        pluto_block::drain_schedules(dimension, &mut edit);
    }
    let Some(sink) = pluto_block::sink() else { return };
    let Some(current) = info(level, state) else { return };
    if !(sink.has)(&current.block) {
        return;
    }
    let mut edit = Edit { level, ticks: Some(ticks), feature_depth: 0 };
    if pick(sink)(&mut edit, &current, [pos.x, pos.y, pos.z]) {
        *handled = true;
    }
}

pub(super) fn bonemeal_valid(level: &GameLevel<'_>, pos: BlockPos, state: BlockStateId, valid: &mut Option<bool>) {
    let Some(sink) = pluto_block::sink() else { return };
    let Some(current) = info(level, state) else { return };
    if !(sink.has)(&current.block) {
        return;
    }
    if let Some(answer) = (sink.bonemeal_valid)(&View { level }, &current, [pos.x, pos.y, pos.z]) {
        *valid = Some(answer);
    }
}

pub(super) fn fire_odds(level: &GameLevel<'_>, state: BlockStateId, odds: &mut Option<(i32, i32)>) {
    let Some(sink) = pluto_block::sink() else { return };
    let Some(current) = info(level, state) else { return };
    if !(sink.has)(&current.block) {
        return;
    }
    if let Some(answer) = (sink.fire_odds)(&current) {
        *odds = Some(answer);
    }
}

pub(super) fn bonemeal_success(level: &mut GameLevel<'_>, pos: BlockPos, state: BlockStateId, success: &mut bool) {
    let Some(sink) = pluto_block::sink() else { return };
    let Some(current) = info(level, state) else { return };
    if !(sink.has)(&current.block) {
        return;
    }
    let mut edit = Edit { level, ticks: None, feature_depth: 0 };
    if let Some(answer) = (sink.bonemeal_success)(&mut edit, &current, [pos.x, pos.y, pos.z]) {
        *success = answer;
    }
}

pub(super) fn bonemeal_perform(level: &mut GameLevel<'_>, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId) {
    let Some(sink) = pluto_block::sink() else { return };
    let Some(current) = info(level, state) else { return };
    if !(sink.has)(&current.block) {
        return;
    }
    let mut edit = Edit { level, ticks: Some(ticks), feature_depth: 0 };
    (sink.bonemeal_perform)(&mut edit, &current, [pos.x, pos.y, pos.z]);
}

pub(super) fn may_place_on(level: &GameLevel<'_>, state: BlockStateId, ground: BlockStateId, ground_pos: BlockPos, may: &mut Option<bool>) {
    let Some(sink) = pluto_block::sink() else { return };
    let (Some(plant), Some(below)) = (info(level, state), info(level, ground)) else { return };
    if !(sink.has)(&plant.block) {
        return;
    }
    if let Some(answer) = (sink.may_place_on)(&View { level }, &plant, &below, [ground_pos.x, ground_pos.y, ground_pos.z]) {
        *may = Some(answer);
    }
}
