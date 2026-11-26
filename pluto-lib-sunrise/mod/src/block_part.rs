//! PlutoLib's block behaviours as a Sunrise `BlockPart`: the native-API
//! replacement for the woven `pluto-block-*` helpers (L3b).
//!
//! One part instance is attached to every block that has a pure
//! [`BlockBehaviors`] registration; each part method converts the event's
//! native state to the pure text state, opens a [`BlockView`]/[`BlockEditor`]
//! over the level the game lends, and runs the same pure dispatch the woven
//! sink ran (`pluto_block_level`). The fluid rows ride the same part:
//! `world-fluid-tick` is the fluid engine's tick claimed at the head of
//! `tick_block`, and `world-fluid-neighbor` is its schedule request at the
//! tail of `update_shape`.
//!
//! Registration: [`register_parts`] attaches the part for every behaviour
//! block the pure registry already holds when PlutoLib's own `register`
//! runs. A consumer mod that registers behaviours later attaches the part
//! itself in its own `register` through [`attach`].

use std::sync::Arc;

use mod_api::{
    BlockPart, BlockState as NativeState, BlockUpdateFlags, Direction as NativeDirection, Flow,
    LevelHost, LevelRandomHost, LevelSide, LevelTicksHost, LevelWriteHost, Registrar,
};

use crate::pure::base::Identifier;
use crate::pure::block_api_v1::{
    self, BlockBehaviors, BlockEditor, BlockView, BonemealContext, Direction, PlaceOnContext,
    Placement, PlacementContext, ShapeUpdateContext, SurviveContext, TickContext,
};
use crate::pure::world_api_v1::BlockState;
use pluto_api::fluid::{self, Cell, Sample};

type Pos = [i32; 3];

/// The part. Stateless: the behaviour is looked up per call from the pure
/// registry, exactly as the woven sink's `has(block)` gate did.
pub struct PlutoBlockPart;

/// Attach the block part to `block` (a namespaced block id) on `registrar`.
/// Consumers call this from their own `register` for each block they gave a
/// pure behaviour, after registering the behaviour itself.
pub fn attach(registrar: &mut dyn Registrar, block: &str) {
    let key = registrar.block_key(block);
    registrar.attach_block_part(key, "pluto_lib:block", Arc::new(PlutoBlockPart));
}

/// Attach the part to every block the pure behaviour registry holds now.
/// Called from PlutoLib's `register`.
pub fn register_parts(registrar: &mut dyn Registrar) {
    for behavior in BlockBehaviors::registered() {
        attach(registrar, &behavior.block().to_string());
    }
}

fn native_direction(direction: Direction) -> NativeDirection {
    match direction {
        Direction::Down => NativeDirection::Down,
        Direction::Up => NativeDirection::Up,
        Direction::North => NativeDirection::North,
        Direction::South => NativeDirection::South,
        Direction::West => NativeDirection::West,
        Direction::East => NativeDirection::East,
    }
}

fn pure_direction(direction: NativeDirection) -> Direction {
    match direction {
        NativeDirection::Down => Direction::Down,
        NativeDirection::Up => Direction::Up,
        NativeDirection::North => Direction::North,
        NativeDirection::South => Direction::South,
        NativeDirection::West => Direction::West,
        NativeDirection::East => Direction::East,
    }
}

fn face(direction: Direction) -> mod_api::BlockFace {
    match direction {
        Direction::Down => mod_api::BlockFace::Down,
        Direction::Up => mod_api::BlockFace::Up,
        Direction::North => mod_api::BlockFace::North,
        Direction::South => mod_api::BlockFace::South,
        Direction::West => mod_api::BlockFace::West,
        Direction::East => mod_api::BlockFace::East,
    }
}

/// A native state as the pure text state, through the level's own tables.
fn pure_state(level: &dyn LevelHost, state: NativeState) -> Option<BlockState> {
    BlockState::parse(&level.state_string(state)?).ok()
}

/// A pure state resolved back to a native state in this level.
fn native_state(level: &dyn LevelHost, state: &BlockState) -> Option<NativeState> {
    level.block_state_id(&state.to_string())
}

fn is_air_state(state: &BlockState) -> bool {
    matches!(state.block.to_string().as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
}

/// The pure [`BlockView`] over a level the game lends to a part call.
struct LevelView<'a> {
    level: &'a dyn LevelHost,
}

impl LevelView<'_> {
    fn state_at(&self, pos: Pos) -> Option<BlockState> {
        pure_state(self.level, self.level.block_state(pos)?)
    }

    fn in_tag(&self, block: &str, tag: &str) -> bool {
        let Some(key) = mod_api::__private::block_key(block) else { return false };
        let Some(tag_id) = mod_api::Identifier::parse_namespaced(tag) else { return false };
        let Some(tag_key) = mod_api::__private::tag_key(&tag_id) else { return false };
        self.level.tags().block_in_tag(key, tag_key).unwrap_or(false)
    }
}

impl BlockView for LevelView<'_> {
    fn block_state(&self, pos: Pos) -> Option<BlockState> {
        self.state_at(pos)
    }

    fn is_air(&self, pos: Pos) -> bool {
        self.state_at(pos).is_some_and(|state| is_air_state(&state))
    }

    fn is_replaceable(&self, pos: Pos) -> bool {
        let Some(native) = self.level.block_state(pos) else { return false };
        self.level.block_states().replaceable(native).unwrap_or_else(|| self.is_air(pos))
    }

    fn has_neighbor_signal(&self, pos: Pos) -> Option<bool> {
        self.level.has_neighbor_signal(pos)
    }

    fn is_face_sturdy(&self, pos: Pos, face_dir: Direction) -> bool {
        self.face_sturdy(pos, face_dir).unwrap_or(false)
    }

    fn face_sturdy(&self, pos: Pos, face_dir: Direction) -> Option<bool> {
        self.level.block_face_sturdy(pos, face(face_dir), mod_api::SupportType::Full)
    }

    fn is_in_tag(&self, pos: Pos, tag: &str) -> bool {
        let Some(state) = self.state_at(pos) else { return false };
        self.in_tag(&state.block.to_string(), tag)
    }

    fn is_state_in_tag(&self, state: &BlockState, tag: &str) -> Option<bool> {
        let key = mod_api::__private::block_key(&state.block.to_string())?;
        let tag_id = mod_api::Identifier::parse_namespaced(tag)?;
        let tag_key = mod_api::__private::tag_key(&tag_id)?;
        self.level.tags().block_in_tag(key, tag_key)
    }

    // The class-chain facts the pure survival walk reads are a host-table
    // artifact of the woven bridge: in the part model the game itself owns
    // vanilla survival, and this part is only asked about blocks with a
    // registered behaviour, which `survives` answers first. None is honest.
    fn placement_facts(&self, _state: &BlockState) -> Option<crate::pure::world_api_v1::PlacementFacts> {
        None
    }

    fn brightness(&self, pos: Pos) -> u8 {
        let darken = self.level.sky_darken().unwrap_or(0);
        u8::try_from(self.level.raw_brightness(pos, darken).clamp(0, 15)).unwrap_or(0)
    }

    fn raw_brightness(&self, pos: Pos) -> Option<u8> {
        Some(u8::try_from(self.level.raw_brightness(pos, 0).clamp(0, 15)).unwrap_or(0))
    }

    fn light_passes_up(&self, pos: Pos) -> bool {
        let Some(native) = self.level.block_state(pos) else { return false };
        self.level.block_states().propagates_skylight_down(native).unwrap_or(false)
    }

    fn is_water(&self, pos: Pos) -> bool {
        let Some(state) = self.state_at(pos) else { return false };
        state.block.to_string() == "minecraft:water" || state.property("waterlogged") == Some("true")
    }

    fn is_fluid_full(&self, pos: Pos) -> bool {
        self.level.fluid_state(pos).is_some_and(|fluid| fluid.amount >= 8)
    }

    fn is_area_loaded(&self, pos: Pos, radius: i32) -> bool {
        let r = radius.max(0);
        [[0, 0], [r, r], [r, -r], [-r, r], [-r, -r]]
            .iter()
            .all(|[dx, dz]| self.level.is_loaded([pos[0] + dx, pos[1], pos[2] + dz]))
    }

    fn is_full_block(&self, pos: Pos) -> bool {
        let Some(shape) = self.level.collision_shape(pos) else { return false };
        let boxes: Vec<[f64; 6]> = shape.boxes().collect();
        boxes.len() == 1 && boxes[0] == [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
    }

    fn build_height(&self) -> [i32; 2] {
        [self.level.min_y(), self.level.min_y() + self.level.height()]
    }
}

/// The pure [`BlockEditor`] over a writable level.
struct LevelEdit<'a> {
    level: &'a mut dyn LevelWriteHost,
}

impl LevelEdit<'_> {
    fn view(&self) -> LevelView<'_> {
        LevelView { level: &*self.level }
    }
}

impl BlockView for LevelEdit<'_> {
    fn block_state(&self, pos: Pos) -> Option<BlockState> {
        self.view().block_state(pos)
    }
    fn is_air(&self, pos: Pos) -> bool {
        self.view().is_air(pos)
    }
    fn is_replaceable(&self, pos: Pos) -> bool {
        self.view().is_replaceable(pos)
    }
    fn has_neighbor_signal(&self, pos: Pos) -> Option<bool> {
        self.view().has_neighbor_signal(pos)
    }
    fn is_face_sturdy(&self, pos: Pos, face_dir: Direction) -> bool {
        self.view().is_face_sturdy(pos, face_dir)
    }
    fn face_sturdy(&self, pos: Pos, face_dir: Direction) -> Option<bool> {
        self.view().face_sturdy(pos, face_dir)
    }
    fn is_in_tag(&self, pos: Pos, tag: &str) -> bool {
        self.view().is_in_tag(pos, tag)
    }
    fn is_state_in_tag(&self, state: &BlockState, tag: &str) -> Option<bool> {
        self.view().is_state_in_tag(state, tag)
    }
    fn placement_facts(&self, state: &BlockState) -> Option<crate::pure::world_api_v1::PlacementFacts> {
        self.view().placement_facts(state)
    }
    fn brightness(&self, pos: Pos) -> u8 {
        self.view().brightness(pos)
    }
    fn raw_brightness(&self, pos: Pos) -> Option<u8> {
        self.view().raw_brightness(pos)
    }
    fn light_passes_up(&self, pos: Pos) -> bool {
        self.view().light_passes_up(pos)
    }
    fn is_water(&self, pos: Pos) -> bool {
        self.view().is_water(pos)
    }
    fn is_fluid_full(&self, pos: Pos) -> bool {
        self.view().is_fluid_full(pos)
    }
    fn is_area_loaded(&self, pos: Pos, radius: i32) -> bool {
        self.view().is_area_loaded(pos, radius)
    }
    fn is_full_block(&self, pos: Pos) -> bool {
        self.view().is_full_block(pos)
    }
    fn build_height(&self) -> [i32; 2] {
        self.view().build_height()
    }
}

fn update_flags(flags: i32) -> BlockUpdateFlags {
    let mut out = BlockUpdateFlags::NONE;
    if flags & 1 != 0 {
        out = out | BlockUpdateFlags::NEIGHBORS;
    }
    if flags & 2 != 0 {
        out = out | BlockUpdateFlags::CLIENTS;
    }
    if flags & 16 != 0 {
        out = out | BlockUpdateFlags::KNOWN_SHAPE;
    }
    out
}

impl BlockEditor for LevelEdit<'_> {
    fn set_block_state(&mut self, pos: Pos, state: &BlockState) -> bool {
        self.set_block_state_with_flags(pos, state, 3)
    }

    fn set_block_state_with_flags(&mut self, pos: Pos, state: &BlockState, flags: i32) -> bool {
        let Some(native) = native_state(&*self.level, state) else { return false };
        self.level.set_block(pos, native, update_flags(flags))
    }

    fn remove_block(&mut self, pos: Pos) -> bool {
        self.level.remove_block(pos, false)
    }

    // No destroy-with-drops call exists on the writable level host; the
    // trait's None (unsupported) is the honest answer, as in the woven
    // editor, whose break went through a separate game seam.
    fn break_block(&mut self, _pos: Pos, _drop: bool) -> Option<bool> {
        None
    }

    fn random_int(&mut self, bound: i32) -> i32 {
        self.level.with_random(|rng| rng.next_int_bound(bound)).unwrap_or(0)
    }

    fn random_float(&mut self) -> f32 {
        self.level.with_random(|rng| rng.next_float()).unwrap_or(0.0)
    }

    fn schedule_tick(&mut self, pos: Pos, block: &str, delay: i32) -> bool {
        let Some(key) = mod_api::__private::block_key(block) else { return false };
        self.level.schedule_block_tick(pos, key, delay)
    }

    // Feature placement has no call on the writable level host.
    fn place_feature(&mut self, _feature: &str, _pos: Pos) -> bool {
        false
    }
}

/// A read-only editor for the bonemeal-success roll, whose part call lends
/// only a random host: reads and draws work, writes honestly fail.
struct RandomEdit<'a> {
    level: &'a mut dyn LevelRandomHost,
}

impl BlockView for RandomEdit<'_> {
    fn block_state(&self, pos: Pos) -> Option<BlockState> {
        LevelView { level: &*self.level }.block_state(pos)
    }
    fn is_air(&self, pos: Pos) -> bool {
        LevelView { level: &*self.level }.is_air(pos)
    }
    fn is_replaceable(&self, pos: Pos) -> bool {
        LevelView { level: &*self.level }.is_replaceable(pos)
    }
    fn has_neighbor_signal(&self, pos: Pos) -> Option<bool> {
        LevelView { level: &*self.level }.has_neighbor_signal(pos)
    }
    fn is_face_sturdy(&self, pos: Pos, face_dir: Direction) -> bool {
        LevelView { level: &*self.level }.is_face_sturdy(pos, face_dir)
    }
    fn face_sturdy(&self, pos: Pos, face_dir: Direction) -> Option<bool> {
        LevelView { level: &*self.level }.face_sturdy(pos, face_dir)
    }
    fn is_in_tag(&self, pos: Pos, tag: &str) -> bool {
        LevelView { level: &*self.level }.is_in_tag(pos, tag)
    }
    fn is_state_in_tag(&self, state: &BlockState, tag: &str) -> Option<bool> {
        LevelView { level: &*self.level }.is_state_in_tag(state, tag)
    }
    fn placement_facts(&self, state: &BlockState) -> Option<crate::pure::world_api_v1::PlacementFacts> {
        LevelView { level: &*self.level }.placement_facts(state)
    }
    fn brightness(&self, pos: Pos) -> u8 {
        LevelView { level: &*self.level }.brightness(pos)
    }
    fn raw_brightness(&self, pos: Pos) -> Option<u8> {
        LevelView { level: &*self.level }.raw_brightness(pos)
    }
    fn light_passes_up(&self, pos: Pos) -> bool {
        LevelView { level: &*self.level }.light_passes_up(pos)
    }
    fn is_water(&self, pos: Pos) -> bool {
        LevelView { level: &*self.level }.is_water(pos)
    }
    fn is_fluid_full(&self, pos: Pos) -> bool {
        LevelView { level: &*self.level }.is_fluid_full(pos)
    }
    fn is_area_loaded(&self, pos: Pos, radius: i32) -> bool {
        LevelView { level: &*self.level }.is_area_loaded(pos, radius)
    }
    fn is_full_block(&self, pos: Pos) -> bool {
        LevelView { level: &*self.level }.is_full_block(pos)
    }
    fn build_height(&self) -> [i32; 2] {
        LevelView { level: &*self.level }.build_height()
    }
}

impl BlockEditor for RandomEdit<'_> {
    fn set_block_state(&mut self, _pos: Pos, _state: &BlockState) -> bool {
        false
    }
    fn random_int(&mut self, bound: i32) -> i32 {
        self.level.with_random(|rng| rng.next_int_bound(bound)).unwrap_or(0)
    }
    fn random_float(&mut self) -> f32 {
        self.level.with_random(|rng| rng.next_float()).unwrap_or(0.0)
    }
    fn schedule_tick(&mut self, _pos: Pos, _block: &str, _delay: i32) -> bool {
        false
    }
    fn place_feature(&mut self, _feature: &str, _pos: Pos) -> bool {
        false
    }
}

/// The fluid engine's cell at a position, mirroring the game-side adapter's
/// `cell` (world-fluid-query): the block names a registered fluid and its
/// `level` property (0..=8) is the cell level.
fn fluid_cell(level: &dyn LevelHost, pos: Pos) -> Option<(std::sync::Arc<fluid::Definition>, Cell)> {
    let state = pure_state(level, level.block_state(pos)?)?;
    let definition = fluid::for_block(&state.block.to_string())?;
    let level_value = state.property("level")?.parse::<u8>().ok().filter(|n| *n <= 8)?;
    let fluid_id = definition.id.clone();
    Some((definition, Cell { fluid: fluid_id, level: level_value }))
}

/// The fluid engine's world over a writable level (the `world-fluid-tick`
/// replacement). The game-side adapter also gated spread through the
/// protection surface (`allow_spread`); that gate belongs to the protection
/// rows, not to this engine port, and is noted in SUNRISE.md.
struct FluidWorld<'a> {
    level: &'a mut dyn LevelWriteHost,
    spreading_from: Option<Pos>,
}

impl FluidWorld<'_> {
    fn sample_at(&self, pos: Pos) -> Sample {
        if !self.level.is_loaded(pos) {
            return Sample::Unloaded;
        }
        let Some(state) = pure_state(&*self.level, match self.level.block_state(pos) {
            Some(state) => state,
            None => return Sample::Unloaded,
        }) else {
            return Sample::Solid;
        };
        if is_air_state(&state) {
            return Sample::Air;
        }
        match fluid_cell(&*self.level, pos) {
            Some((_, cell)) => Sample::Fluid(cell),
            None => Sample::Solid,
        }
    }
}

impl fluid::World for FluidWorld<'_> {
    fn sample(&self, pos: Pos) -> Sample {
        self.sample_at(pos)
    }

    fn replace(&mut self, pos: Pos, expected: &Sample, next: Option<&Cell>) -> bool {
        if &self.sample_at(pos) != expected {
            return false;
        }
        let state = match next {
            None => BlockState::new(Identifier::parse("minecraft:air").unwrap()),
            Some(cell) if cell.level <= 8 => {
                let Some(definition) = fluid::get(&cell.fluid) else { return false };
                BlockState::new(Identifier::parse(&definition.block).unwrap())
                    .with("level", &cell.level.to_string())
            }
            Some(_) => return false,
        };
        let Some(native) = native_state(&*self.level, &state) else { return false };
        self.level.set_block(pos, native, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS)
    }

    fn schedule(&mut self, pos: Pos, delay: u16) {
        if !self.level.is_loaded(pos) {
            return;
        }
        let Some((definition, _)) = fluid_cell(&*self.level, pos) else { return };
        let Some(key) = mod_api::__private::block_key(&definition.block) else { return };
        self.level.schedule_block_tick(pos, key, i32::from(delay));
    }
}

fn pos(p: mod_api::BlockPos) -> Pos {
    [p.x, p.y, p.z]
}

fn level_view<'a>(side: &LevelSide<'a>) -> LevelView<'a> {
    match side {
        LevelSide::Server(level) => LevelView { level: *level },
        LevelSide::Client(level) => LevelView { level: *level },
    }
}

#[mod_api::sunrise_part]
impl BlockPart for PlutoBlockPart {
    fn can_survive(&self, level: LevelSide<'_>, event: &mut mod_api::CanSurviveEvent) -> Option<bool> {
        let view = level_view(&level);
        let state = pure_state(view.level, event.state)?;
        block_api_v1::can_survive(&state, pos(event.pos), &view)
    }

    fn may_place_on(&self, level: LevelSide<'_>, event: &mut mod_api::MayPlaceOnEvent) -> Option<bool> {
        let view = level_view(&level);
        let state = pure_state(view.level, event.state)?;
        let block = state.block.to_string();
        let ground = pure_state(view.level, event.ground)?;
        let context = PlaceOnContext { state, ground, ground_pos: pos(event.ground_pos), view: &view };
        block_api_v1::may_place_on(&block, &context)
    }

    fn state_for_placement(
        &self,
        level: LevelSide<'_>,
        event: &mut mod_api::StateForPlacementEvent<'_>,
    ) -> Option<Option<NativeState>> {
        let view = level_view(&level);
        let state = pure_state(view.level, event.state)?;
        let (yaw, pitch, sneaking) = match event.context.source {
            mod_api::PlacementSource::Player { yaw, pitch, sneaking } => (yaw, pitch, sneaking),
            _ => (0.0, 0.0, false),
        };
        let item = event
            .context
            .item
            .map(|key| mod_api::__private::item_key_id(key).unwrap_or("minecraft:air").to_owned())
            .unwrap_or_else(|| "minecraft:air".to_owned());
        let context = PlacementContext {
            state: state.clone(),
            pos: pos(event.context.pos),
            face: pure_direction(event.context.face),
            yaw,
            pitch,
            sneaking,
            in_water: is_water_key(&event.fluid),
            item,
            replace_clicked: event.context.replace_clicked,
            view: &view,
        };
        match block_api_v1::placement(&state.block.to_string(), &context) {
            Placement::Default => None,
            Placement::Refuse => Some(None),
            Placement::State(wanted) => native_state(view.level, &wanted).map(Some),
        }
    }

    fn update_shape(
        &self,
        level: &mut dyn LevelTicksHost,
        event: &mut mod_api::UpdateShapeEvent,
    ) -> Option<NativeState> {
        // world-fluid-neighbor: a fluid cell whose neighbour changed asks
        // for its own tick after the definition's delay.
        if let Some((definition, _)) = fluid_cell(&*level, pos(event.pos)) {
            if let Some(key) = mod_api::__private::block_key(&definition.block) {
                level.schedule_block_tick(pos(event.pos), key, i32::from(definition.tick_delay));
            }
        }
        let (answer, delay, block) = {
            let view = LevelView { level: &*level };
            let state = pure_state(view.level, event.state)?;
            let block = state.block.to_string();
            let neighbour = pure_state(view.level, event.neighbour);
            let context = ShapeUpdateContext::new(
                state,
                pos(event.pos),
                pure_direction(event.direction),
                pos(event.neighbour_pos),
                neighbour,
                &view,
            );
            let answer = block_api_v1::shape_update(&block, &context);
            (answer, context.scheduled_tick_delay(), block)
        };
        if let Some(delay) = delay {
            if let Some(key) = mod_api::__private::block_key(&block) {
                level.schedule_block_tick(pos(event.pos), key, delay);
            }
        }
        answer.and_then(|wanted| native_state(&*level, &wanted))
    }

    fn random_tick(&self, level: &mut dyn LevelWriteHost, event: &mut mod_api::RandomTickEvent) -> Flow {
        let state = match pure_state(&*level, event.state) {
            Some(state) => state,
            None => return Flow::Continue,
        };
        let mut edit = LevelEdit { level };
        let mut context = TickContext { state: state.clone(), pos: pos(event.pos), world: &mut edit };
        if block_api_v1::random_tick(&state.block.to_string(), &mut context) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn tick_block(&self, level: &mut dyn LevelWriteHost, event: &mut mod_api::TickBlockEvent) -> Flow {
        // world-fluid-tick: the fluid engine claims its cells before the
        // block dispatch, as the woven head claimed `handled`.
        if let Some((definition, _)) = fluid_cell(&*level, pos(event.pos)) {
            let mut world = FluidWorld { level, spreading_from: Some(pos(event.pos)) };
            fluid::tick(&mut world, pos(event.pos), &definition);
            return Flow::Cancel;
        }
        let state = match pure_state(&*level, event.state) {
            Some(state) => state,
            None => return Flow::Continue,
        };
        let mut edit = LevelEdit { level };
        let mut context = TickContext { state: state.clone(), pos: pos(event.pos), world: &mut edit };
        if block_api_v1::scheduled_tick(&state.block.to_string(), &mut context) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn is_valid_bonemeal_target(
        &self,
        level: LevelSide<'_>,
        event: &mut mod_api::IsValidBonemealTargetEvent,
    ) -> Option<bool> {
        let view = level_view(&level);
        let state = pure_state(view.level, event.state)?;
        let context = BonemealContext { state: state.clone(), pos: pos(event.pos), view: &view };
        block_api_v1::can_bonemeal(&state.block.to_string(), &context)
    }

    fn is_bonemeal_success(&self, level: &mut dyn LevelRandomHost, event: &mut mod_api::IsBonemealSuccessEvent) {
        let state = match pure_state(&*level, event.state) {
            Some(state) => state,
            None => return,
        };
        let mut edit = RandomEdit { level };
        let mut context = TickContext { state: state.clone(), pos: pos(event.pos), world: &mut edit };
        if let Some(success) = block_api_v1::bonemeal_success(&state.block.to_string(), &mut context) {
            event.success = success;
        }
    }

    fn perform_bonemeal(&self, level: &mut dyn LevelWriteHost, event: &mut mod_api::PerformBonemealEvent) {
        let state = match pure_state(&*level, event.state) {
            Some(state) => state,
            None => return,
        };
        let mut edit = LevelEdit { level };
        let mut context = TickContext { state, pos: pos(event.pos), world: &mut edit };
        let block = context.state.block.to_string();
        block_api_v1::on_bonemeal(&block, &mut context);
    }

    fn fire_odds(&self, level: LevelSide<'_>, event: &mut mod_api::FireOddsEvent) -> Option<(i32, i32)> {
        let view = level_view(&level);
        let block = view.level.state_block(event.state)?;
        block_api_v1::fire_odds(block)
    }

    /// The block-settings row's runtime half: a behaviour's static piston
    /// reaction, answered from the same registry the declaration vocabulary
    /// (`BlockProperties.push_reaction`) mirrors at content load.
    fn piston_push_reaction(&self, event: &mut mod_api::PistonPushReactionEvent) -> Option<mod_api::PushReaction> {
        let block = mod_api::__private::block_key_id(event.state.block())?;
        let behavior = BlockBehaviors::get(block)?.piston_behavior_override()?;
        Some(match behavior {
            block_api_v1::PistonBehavior::Normal => mod_api::PushReaction::PushPull,
            block_api_v1::PistonBehavior::Destroy => mod_api::PushReaction::Popped,
            block_api_v1::PistonBehavior::Block => mod_api::PushReaction::Immoveable,
            block_api_v1::PistonBehavior::Ignore => mod_api::PushReaction::IgnoreEntity,
            block_api_v1::PistonBehavior::PushOnly => mod_api::PushReaction::Push,
        })
    }
}

/// Whether the placement event's fluid is water.
fn is_water_key(fluid: &mod_api::FluidState) -> bool {
    fluid.amount > 0 && mod_api::__private::fluid_key("minecraft:water").is_some_and(|water| fluid.fluid == water)
}
