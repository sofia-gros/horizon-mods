//! Behaviour tests for the G1 gadgets: PunjiBlock's placement fallback, survival and connection updates,
//! the hanging ichor cake's support rule, and FoodCakeBlock's slice eating through the registered
//! use-on-block callback, dispatched the way the host dispatches them.

use pluto_lib::{
    ActionResult, BlockBehaviors, BlockState, Direction, EffectInstance, FoodStatus, MemoryBlockView,
    MemoryPlayerOps, PathNodeType, PathNodeTypeContext, Placement, PlacementContext, ServerPlayer,
    ShapeUpdateContext, UseItemOnBlockCallback, UseItemOnBlockContext, WorldOps,
};
use std::collections::{BTreeMap, VecDeque};

fn state(text: &str) -> BlockState {
    BlockState::parse(text).unwrap()
}

fn punji(facing: &str) -> BlockState {
    state(&format!("tconstruct:punji[facing={facing},north=false,east=false,northeast=false,northwest=false]"))
}

fn view_of(blocks: &[([i32; 3], &str)]) -> MemoryBlockView {
    let mut view = MemoryBlockView::default();
    for (pos, text) in blocks {
        view.blocks.insert(*pos, state(text));
    }
    view
}

fn registered() {
    crate::gadgets::register();
}

#[test]
fn all_seven_gadget_blocks_register_and_the_punji_paths_as_damaging() {
    registered();
    for name in ["punji", "earth_cake", "sky_cake", "ichor_cake", "ender_cake", "blood_cake", "magma_cake"] {
        assert!(BlockBehaviors::get(&format!("tconstruct:{name}")).is_some(), "{name} has a behaviour");
    }
    let view = MemoryBlockView::default();
    let context = PathNodeTypeContext { state: punji("down"), pos: [0, 64, 0], view: &view, entity: None, kind: None };
    assert_eq!(pluto_lib::fixtures::path_node(&context), Some(PathNodeType::Damaging), "the punji's DAMAGE_OTHER is the host's DAMAGING");
}

fn placement(view: &MemoryBlockView, pos: [i32; 3], face: Direction) -> Placement {
    let context = PlacementContext {
        state: state("tconstruct:punji"), pos, face, yaw: 0.0, pitch: 0.0, sneaking: false,
        in_water: false, item: "tconstruct:punji".to_owned(), replace_clicked: false, view,
    };
    crate::gadgets::punji_placement(&context)
}

#[test]
fn punji_placement_faces_away_from_the_clicked_face() {
    registered();
    // Clicking the top of a floor block: the sticks stand on it, facing down into it.
    let view = view_of(&[([0, 63, 0], "minecraft:stone")]);
    let Placement::State(placed) = placement(&view, [0, 64, 0], Direction::Up) else { panic!("placed") };
    assert_eq!(placed, punji("down"));
    // Clicking the underside of a ceiling: facing up, held by the block above.
    let view = view_of(&[([0, 65, 0], "minecraft:stone")]);
    let Placement::State(placed) = placement(&view, [0, 64, 0], Direction::Down) else { panic!("placed") };
    assert_eq!(placed, punji("up"));
    // Clicking a wall's north face: facing south, held by the wall behind.
    let view = view_of(&[([0, 64, 1], "minecraft:stone")]);
    let Placement::State(placed) = placement(&view, [0, 64, 0], Direction::North) else { panic!("placed") };
    assert_eq!(placed, punji("south"));
}

#[test]
fn punji_placement_falls_back_in_direction_order_and_refuses_without_support() {
    registered();
    // The clicked face wants facing down, but only the block to the north can hold the sticks:
    // DOWN and UP fail first, NORTH is next in Direction order.
    let view = view_of(&[([0, 64, -1], "minecraft:stone")]);
    let Placement::State(placed) = placement(&view, [0, 64, 0], Direction::Up) else { panic!("placed") };
    assert_eq!(placed, punji("north"));
    // Nothing can hold them anywhere: the placement is refused, as a null state in the original.
    let view = view_of(&[]);
    assert_eq!(placement(&view, [0, 64, 0], Direction::Up), Placement::Refuse);
    // A weak face does not hold the sticks either.
    let mut view = view_of(&[([0, 63, 0], "minecraft:stone")]);
    view.weak.insert([0, 63, 0]);
    assert_eq!(placement(&view, [0, 64, 0], Direction::Up), Placement::Refuse);
}

#[test]
fn punji_placement_reads_its_connections() {
    registered();
    // Facing down, the local north is north and the local east is east; only a punji with the same
    // facing joins. The north-east diagonal reads the block past the north one.
    let view = view_of(&[
        ([0, 63, 0], "minecraft:stone"),
        ([0, 64, -1], "tconstruct:punji[facing=down]"),
        ([1, 64, 0], "tconstruct:punji[facing=up]"),
        ([1, 64, -1], "tconstruct:punji[facing=down]"),
    ]);
    let Placement::State(placed) = placement(&view, [0, 64, 0], Direction::Up) else { panic!("placed") };
    assert_eq!(placed.property("north"), Some("true"), "the same-facing punji to the north joins");
    assert_eq!(placed.property("east"), Some("false"), "the up-facing punji to the east does not join");
    assert_eq!(placed.property("northeast"), Some("true"), "the diagonal past the north joins");
    assert_eq!(placed.property("northwest"), Some("false"));
}

fn update(view: &MemoryBlockView, state: BlockState, pos: [i32; 3], direction: Direction) -> Option<BlockState> {
    let neighbour_pos = direction.relative(pos);
    let neighbour = view.blocks.get(&neighbour_pos).cloned();
    let context = ShapeUpdateContext::new(state, pos, direction, neighbour_pos, neighbour, view);
    crate::gadgets::punji_update(&context)
}

#[test]
fn punji_falls_when_its_support_goes_and_only_then() {
    registered();
    // The support below is gone and the change came from that side: the sticks fall.
    let view = view_of(&[]);
    assert_eq!(update(&view, punji("down"), [0, 64, 0], Direction::Down), Some(state("minecraft:air")));
    // The same missing support, but the change came from the north: no fall (Java only checks the
    // facing side), the state comes back with its joins reread.
    let updated = update(&view, punji("down"), [0, 64, 0], Direction::North).unwrap();
    assert_eq!(updated.block.to_string(), "tconstruct:punji");
    // The support is still there: nothing changes but the joins.
    let view = view_of(&[([0, 63, 0], "minecraft:stone")]);
    let updated = update(&view, punji("down"), [0, 64, 0], Direction::Down).unwrap();
    assert_eq!(updated, punji("down"));
}

#[test]
fn punji_updates_follow_the_changed_side_and_the_diagonals_always() {
    registered();
    // A same-facing punji appears to the north: the north join follows.
    let view = view_of(&[([0, 63, 0], "minecraft:stone"), ([0, 64, -1], "tconstruct:punji[facing=down]")]);
    let updated = update(&view, punji("down"), [0, 64, 0], Direction::North).unwrap();
    assert_eq!(updated.property("north"), Some("true"));
    // It goes again: the join opens.
    let view = view_of(&[([0, 63, 0], "minecraft:stone")]);
    let before = state("tconstruct:punji[facing=down,north=true,east=false,northeast=false,northwest=false]");
    let updated = update(&view, before, [0, 64, 0], Direction::North).unwrap();
    assert_eq!(updated.property("north"), Some("false"));
    // A change from above still rereads the diagonals.
    let view = view_of(&[([0, 63, 0], "minecraft:stone"), ([-1, 64, -1], "tconstruct:punji[facing=down]")]);
    let updated = update(&view, punji("down"), [0, 64, 0], Direction::Up).unwrap();
    assert_eq!(updated.property("northwest"), Some("true"), "the diagonal joins are always reread");
    assert_eq!(updated.property("north"), Some("false"), "an unrelated change leaves the side joins alone");
}

#[test]
fn ichor_cake_hangs_from_a_solid_block_above() {
    registered();
    let view = view_of(&[([0, 65, 0], "minecraft:stone")]);
    assert!(crate::gadgets::ichor_survives(&view, [0, 64, 0]));
    let view = view_of(&[]);
    assert!(!crate::gadgets::ichor_survives(&view, [0, 64, 0]));
    // Only a change from above drops it.
    let cake = state("tconstruct:ichor_cake[bites=2]");
    let context = ShapeUpdateContext::new(cake.clone(), [0, 64, 0], Direction::Up, [0, 65, 0], None, &view);
    assert_eq!(crate::gadgets::ichor_update(&context), Some(state("minecraft:air")));
    let context = ShapeUpdateContext::new(cake.clone(), [0, 64, 0], Direction::Down, [0, 63, 0], None, &view);
    assert_eq!(crate::gadgets::ichor_update(&context), Some(cake.clone()), "a change below leaves the hanging cake");
    let view = view_of(&[([0, 65, 0], "minecraft:stone")]);
    let context = ShapeUpdateContext::new(cake.clone(), [0, 64, 0], Direction::Up, [0, 65, 0], None, &view);
    assert_eq!(crate::gadgets::ichor_update(&context), Some(cake), "the support above still holds it");
}

/// A world for the eating tests: the blocks set, and the random draws the level hands out, in order.
#[derive(Default)]
struct TestWorld {
    blocks: BTreeMap<(String, [i32; 3]), String>,
    draws: VecDeque<f32>,
}

impl WorldOps for TestWorld {
    fn block_state(&mut self, dimension: &str, pos: [i32; 3]) -> Option<String> {
        Some(self.blocks.get(&(dimension.to_owned(), pos)).cloned().unwrap_or_else(|| "minecraft:air".to_owned()))
    }
    fn set_block_state(&mut self, dimension: &str, pos: [i32; 3], state: &str) -> bool {
        self.blocks.insert((dimension.to_owned(), pos), state.to_owned());
        true
    }
    fn play_sound(&mut self, _dimension: &str, _sound: &str, _at: [f64; 3], _volume: f32, _pitch: f32) -> bool {
        true
    }
    fn random_float(&mut self, _dimension: &str) -> Option<f32> {
        self.draws.pop_front()
    }
}

const DIM: &str = "minecraft:overworld";

fn world_with(block: &str, pos: [i32; 3]) -> TestWorld {
    let mut world = TestWorld::default();
    world.blocks.insert((DIM.to_owned(), pos), block.to_owned());
    world
}

fn player(food: Option<FoodStatus>, effects: &[(&str, i32)]) -> MemoryPlayerOps {
    let mut ops = MemoryPlayerOps { food, ..Default::default() };
    for (effect, duration) in effects {
        ops.effects.insert((7, effect.to_string()), EffectInstance::new(effect, *duration));
    }
    ops
}

fn eat(world: &mut TestWorld, ops: &mut MemoryPlayerOps, pos: [i32; 3]) -> ActionResult {
    let context = UseItemOnBlockContext {
        item: String::new(), count: 0, pos, face: [0, 1, 0], dimension: DIM.to_owned(),
        player: ServerPlayer { uuid: 7, name: "Alex".to_owned() }, creative: false, held: None,
    };
    pluto_lib::with_ops(ops, || pluto_lib::with_world(world, || UseItemOnBlockCallback::EVENT.invoke(&context)))
}

fn block_at(world: &TestWorld, pos: [i32; 3]) -> String {
    world.blocks.get(&(DIM.to_owned(), pos)).cloned().unwrap_or_else(|| "minecraft:air".to_owned())
}

fn effect(ops: &MemoryPlayerOps, id: &str) -> Option<i32> {
    ops.effects.get(&(7, id.to_owned())).map(|effect| effect.duration)
}

#[test]
fn a_cake_slice_feeds_applies_its_effect_and_takes_a_bite() {
    registered();
    let pos = [0, 64, 0];
    let mut world = world_with("tconstruct:earth_cake[bites=0]", pos);
    let mut ops = player(Some(FoodStatus { level: 10, saturation: 0.0, exhaustion: 0.0 }), &[]);
    assert_eq!(eat(&mut world, &mut ops, pos), ActionResult::Success);
    assert_eq!(block_at(&world, pos), "tconstruct:earth_cake[bites=1]");
    let food = ops.food.unwrap();
    assert_eq!(food.level, 11, "the earth cake feeds 1");
    assert!((food.saturation - 0.6).abs() < 1e-6, "saturation is nutrition * modifier * 2: {}", food.saturation);
    assert_eq!(effect(&ops, "tconstruct:bouncy"), Some(600), "thirty seconds of bouncy");
}

#[test]
fn the_last_slice_removes_the_cake() {
    registered();
    let pos = [0, 64, 0];
    let mut world = world_with("tconstruct:magma_cake[bites=6]", pos);
    let mut ops = player(Some(FoodStatus { level: 10, saturation: 0.0, exhaustion: 0.0 }), &[]);
    assert_eq!(eat(&mut world, &mut ops, pos), ActionResult::Success);
    assert_eq!(block_at(&world, pos), "minecraft:air");
    assert_eq!(ops.food.unwrap().level, 12, "the magma cake feeds 2");
    assert_eq!(effect(&ops, "minecraft:fire_resistance"), Some(600));
}

#[test]
fn a_block_combination_cake_refuses_while_its_effect_has_time_left() {
    registered();
    let pos = [0, 64, 0];
    // 100 ticks left is the boundary: at or above it the slice is refused and nothing is eaten.
    for (duration, refused) in [(150, true), (100, true), (99, false)] {
        let mut world = world_with("tconstruct:earth_cake[bites=0]", pos);
        let mut ops = player(Some(FoodStatus { level: 10, saturation: 0.0, exhaustion: 0.0 }), &[("tconstruct:bouncy", duration)]);
        let result = eat(&mut world, &mut ops, pos);
        if refused {
            assert_eq!(result, ActionResult::Consume, "duration {duration}: the click is consumed, the slice refused");
            assert_eq!(block_at(&world, pos), "tconstruct:earth_cake[bites=0]", "duration {duration}: no bite taken");
            assert_eq!(ops.food.unwrap().level, 10, "duration {duration}: nothing eaten");
            assert_eq!(effect(&ops, "tconstruct:bouncy"), Some(duration), "duration {duration}: the effect is untouched");
        } else {
            assert_eq!(result, ActionResult::Success, "duration {duration}: the slice is eaten");
            assert_eq!(effect(&ops, "tconstruct:bouncy"), Some(600), "duration {duration}: the effect refreshes to the food's");
        }
    }
}

#[test]
fn the_ender_cake_adds_to_the_current_effect_and_never_refuses() {
    registered();
    let pos = [0, 64, 0];
    let mut world = world_with("tconstruct:ender_cake[bites=3]", pos);
    let mut ops = player(Some(FoodStatus { level: 10, saturation: 0.0, exhaustion: 0.0 }), &[("tconstruct:returning", 250)]);
    assert_eq!(eat(&mut world, &mut ops, pos), ActionResult::Success);
    assert_eq!(effect(&ops, "tconstruct:returning"), Some(850), "ADD: 600 on top of the 250 already held");
    assert_eq!(block_at(&world, pos), "tconstruct:ender_cake[bites=4]");
}

#[test]
fn the_blood_cakes_regeneration_is_short_and_other_blocks_pass() {
    registered();
    let pos = [0, 64, 0];
    let mut world = world_with("tconstruct:blood_cake[bites=0]", pos);
    let mut ops = player(Some(FoodStatus { level: 10, saturation: 0.0, exhaustion: 0.0 }), &[]);
    assert_eq!(eat(&mut world, &mut ops, pos), ActionResult::Success);
    assert_eq!(effect(&ops, "minecraft:regeneration"), Some(150), "three times fifty ticks");
    // A click on anything else is not the cakes' business.
    let mut world = world_with("minecraft:stone", pos);
    let mut ops = player(None, &[]);
    assert_eq!(eat(&mut world, &mut ops, pos), ActionResult::Pass);
    assert_eq!(block_at(&world, pos), "minecraft:stone");
}

fn inside_context(state: BlockState, kind: &str, bounds: [f64; 6], fall_distance: f64) -> pluto_lib::EntityInsideContext {
    pluto_lib::EntityInsideContext {
        dimension: "minecraft:overworld".to_owned(), state, pos: [0, 64, 0], entity: 3,
        kind: kind.to_owned(), bounds, fall_distance, actions: std::cell::RefCell::new(Vec::new()),
    }
}

#[test]
fn punji_damage_follows_the_facing_half_and_the_fall() {
    // Facing down (negative on Y): the entity's min must cross below the mid-plane (y = 64.5).
    assert_eq!(crate::gadgets::punji_damage(Direction::Down, [0, 64, 0], [0.2, 64.6, 0.2, 0.8, 65.8, 0.8], 0.0), None);
    assert_eq!(crate::gadgets::punji_damage(Direction::Down, [0, 64, 0], [0.2, 64.5, 0.2, 0.8, 65.8, 0.8], 0.0), None);
    assert_eq!(crate::gadgets::punji_damage(Direction::Down, [0, 64, 0], [0.2, 64.2, 0.2, 0.8, 65.8, 0.8], 0.0), Some(1.0));
    // Facing up (positive on Y): the entity's max must cross above the mid-plane.
    assert_eq!(crate::gadgets::punji_damage(Direction::Up, [0, 64, 0], [0.2, 63.2, 0.2, 0.8, 64.5, 0.8], 0.0), None);
    assert_eq!(crate::gadgets::punji_damage(Direction::Up, [0, 64, 0], [0.2, 63.2, 0.2, 0.8, 64.6, 0.8], 0.0), Some(1.0));
    // Facing north (negative on Z, mid-plane z = 0.5) and east (positive on X, mid-plane x = 0.5).
    assert_eq!(crate::gadgets::punji_damage(Direction::North, [0, 64, 0], [0.2, 64.0, 0.5, 0.8, 65.8, 1.8], 0.0), None);
    assert_eq!(crate::gadgets::punji_damage(Direction::North, [0, 64, 0], [0.2, 64.0, 0.4, 0.8, 65.8, 1.8], 0.0), Some(1.0));
    assert_eq!(crate::gadgets::punji_damage(Direction::East, [0, 64, 0], [-0.8, 64.0, 0.2, 0.5, 65.8, 0.8], 0.0), None);
    assert_eq!(crate::gadgets::punji_damage(Direction::East, [0, 64, 0], [-0.8, 64.0, 0.2, 0.6, 65.8, 0.8], 0.0), Some(1.0));
    // The fall bonus: once the entity has fallen at all, the damage is 1 + (fall distance + 1).
    assert_eq!(crate::gadgets::punji_damage(Direction::Down, [0, 64, 0], [0.2, 64.2, 0.2, 0.8, 65.8, 0.8], 3.0), Some(5.0));
}

#[test]
fn punji_inside_queues_cactus_damage_for_a_living_entity_only() {
    let crossing = inside_context(punji("down"), "minecraft:zombie", [0.2, 64.2, 0.2, 0.8, 65.8, 0.8], 0.0);
    crate::gadgets::punji_inside(&crossing);
    assert_eq!(crossing.actions.borrow().as_slice(), [pluto_lib::EntityAction::Hurt { target: pluto_lib::ActionTarget::Victim, amount: 1.0, damage_type: "minecraft:cactus".to_owned() }]);
    // An item entity is not living: the original's instanceof check leaves it alone.
    let item = inside_context(punji("down"), "minecraft:item", [0.2, 64.2, 0.2, 0.8, 65.8, 0.8], 0.0);
    crate::gadgets::punji_inside(&item);
    assert!(item.actions.borrow().is_empty());
    // A living entity in the other half of the block is left alone.
    let clear = inside_context(punji("down"), "minecraft:zombie", [0.2, 64.6, 0.2, 0.8, 65.8, 0.8], 0.0);
    crate::gadgets::punji_inside(&clear);
    assert!(clear.actions.borrow().is_empty());
}
