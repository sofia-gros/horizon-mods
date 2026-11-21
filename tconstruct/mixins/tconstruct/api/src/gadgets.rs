//! TinkerGadgets' punji sticks and slime cakes (SlimeKnights, MIT; see LICENSE): the gadgets whose behaviour
//! needs no host service this port lacks. The rest of TinkerGadgets stays unported, each for a named reason:
//! the fancy frames and armor stands are entities with screens, the throwables are projectiles, the
//! piggyback rides on a carry effect, and the books need a book UI; PlutoLib offers none of those services.
//!
//! Punji (PunjiBlock) sticks to any sturdy face, points away from the block holding it, and joins to
//! same-facing neighbours along its local north and east. Not ported, each named:
//! - the contact damage (1, plus fall distance + 1 after a fall, from the cactus source, only for a living
//!   entity whose box crosses the block's mid-plane on the facing axis): the entity-inside callback does
//!   fire since FND1 and a Hurt action from it lands, but the pinned host's inside seam
//!   (pluto-lib/mod/pluto-block-inside.rs over horizon's pluto_block_access transport) fires only for the
//!   player entity — the original damages any living entity — and the transport carries neither the
//!   entity's bounding box nor its fall distance, the two numbers the mid-plane test and the fall bonus
//!   are computed from. Wiring the player-only, box-less half would not be the original's behaviour.
//! - the 0.4 speed factor: the host's block properties have no speed-factor key.
//! - waterlogging: left out of the declaration, as with the pouring blocks (no fluid logging here).
//! - the per-facing collision box: the declaration holds one default box (the floor one, 3,0,3 to 13,6,13);
//!   the sticks' outline still follows the model per facing.
//! - the path node is the original's DAMAGE_OTHER, which the host's categories have no name for; its
//!   DAMAGING category is the damaging-block one and is what the behaviour answers.
//!
//! The cakes (FoodCakeBlock; InvertedCakeBlock for the ichor cake) ride the vanilla cake template, which
//! carries the bites state, the floor cakes' support rule and their per-bite shapes; eating is this
//! module's use-on-block callback, a slice at a time, exactly FoodCakeBlock.eatSlice. Not ported:
//! - the cake items stack like other block items, not to 1: the original's unstackable item has no
//!   per-stack stack size here (the same limit as a filled tank's maximum of 16).
//! - composting (every cake at 1.0): PlutoLib has no composter registration.
//! - the EAT_CAKE_SLICE statistic: no statistic is exposed to award.
//! - the tooltip's duration suffix ("Bouncy (0:30)"): a tooltip line here is one flat text or key, so the
//!   line is the effect's name in its category's colour, without the duration. The amplifier wrap never
//!   applies: every cake effect is amplifier 0.
//! - the ichor cake's per-bite collision narrowing: one default box (its unbitten shape), as with the punji.
//! The Tinkers effects the cakes give (bouncy, double jump, antigravity, returning) are other units' work;
//! given to a player they are skipped by the game until those land, the same as the slime bottles' effects.
use pluto_lib::{
    ActionResult, BlockBehavior, BlockBehaviors, BlockState, BlockView, Direction, EffectInstance, Identifier,
    ItemTooltipCallback, PathNodeType, PistonBehavior, Placement, PlacementContext, ShapeUpdateContext,
    TooltipLine, UseItemOnBlockCallback, UseItemOnBlockContext, World,
};

fn id(name: &str) -> Identifier {
    Identifier::new("tconstruct", name).expect("fixed block id")
}

fn air() -> BlockState {
    BlockState::new(Identifier::new("minecraft", "air").expect("fixed block id"))
}

fn bool_text(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

/// One cake's food (TinkerFood): every cake food can always be eaten and holds a single effect at
/// amplifier 0. `add` is FoodCakeBlock's ADD combination (the ender cake alone); the rest are BLOCK.
/// `color` is the tooltip colour of the effect category's formatting in the original (blue for
/// beneficial, red for harmful, the plain grey for neutral), as 0xRRGGBBAA.
struct Cake {
    nutrition: i32,
    saturation: f32,
    effect: &'static str,
    duration: i32,
    chance: f32,
    add: bool,
    color: u32,
}

const BENEFICIAL: u32 = 0x5555_FFFF;
const HARMFUL: u32 = 0xFF55_55FF;
const NEUTRAL: u32 = 0xAAAA_AAFF;

fn cake(name: &str) -> Option<Cake> {
    Some(match name {
        "earth_cake" => Cake { nutrition: 1, saturation: 0.3, effect: "tconstruct:bouncy", duration: 600, chance: 1.0, add: false, color: BENEFICIAL },
        "sky_cake" => Cake { nutrition: 1, saturation: 0.2, effect: "tconstruct:double_jump", duration: 600, chance: 1.0, add: false, color: BENEFICIAL },
        "ichor_cake" => Cake { nutrition: 1, saturation: 0.3, effect: "tconstruct:antigravity", duration: 600, chance: 1.0, add: false, color: HARMFUL },
        "ender_cake" => Cake { nutrition: 1, saturation: 0.4, effect: "tconstruct:returning", duration: 600, chance: 1.0, add: true, color: NEUTRAL },
        "blood_cake" => Cake { nutrition: 2, saturation: 0.2, effect: "minecraft:regeneration", duration: 150, chance: 1.0, add: false, color: BENEFICIAL },
        "magma_cake" => Cake { nutrition: 2, saturation: 0.2, effect: "minecraft:fire_resistance", duration: 600, chance: 1.0, add: false, color: BENEFICIAL },
        _ => return None,
    })
}

/// The six cakes in TinkerGadgets' order (the foliage enum's order, then the magma cake).
const CAKES: [&str; 6] = ["earth_cake", "sky_cake", "ichor_cake", "ender_cake", "blood_cake", "magma_cake"];

pub(crate) fn register() {
    static EVENTS: std::sync::Once = std::sync::Once::new();
    EVENTS.call_once(|| {
        UseItemOnBlockCallback::EVENT.register(eat_use);
        ItemTooltipCallback::EVENT.register(tooltip);
    });
    BlockBehaviors::register(
        BlockBehavior::new(id("punji"))
            .can_survive(|context| punji_survives(&context.state, context.pos, context.view))
            .state_for_placement(punji_placement)
            .update_shape(punji_update)
            .path_node_type(|_| Some(PathNodeType::Damaging))
            .piston_behavior(PistonBehavior::Destroy),
    );
    // The ichor cake hangs from the block above, against its template's floor-cake rules.
    BlockBehaviors::register(
        BlockBehavior::new(id("ichor_cake"))
            .can_survive(|context| ichor_survives(context.view, context.pos))
            .update_shape(ichor_update)
            .piston_behavior(PistonBehavior::Destroy),
    );
    // The floor cakes stand by their template; only the original's push reaction is theirs to state.
    for name in CAKES {
        if name != "ichor_cake" {
            BlockBehaviors::register(BlockBehavior::new(id(name)).piston_behavior(PistonBehavior::Destroy));
        }
    }
}

fn tooltip(tip: &pluto_lib::ItemTooltipContext) {
    let Some(name) = tip.item.strip_prefix("tconstruct:") else { return };
    if name == "punji" {
        // Mantle's tooltip block item: the one lang line under the name.
        tip.added.borrow_mut().push(TooltipLine::translatable("block.tconstruct.punji.tooltip"));
    } else if let Some(cake) = cake(name) {
        // ContainerFoodItem.addEffectTooltip: the effect's name in its category's colour.
        let key = format!("effect.{}", cake.effect.replace(':', "."));
        tip.added.borrow_mut().push(TooltipLine { text: key, color: cake.color, translate: true });
    }
}

// --- Punji (PunjiBlock) ---

fn facing_of(state: &BlockState) -> Direction {
    state.property("facing").and_then(Direction::parse).unwrap_or(Direction::Down)
}

/// PunjiBlock.getLocalNorth: the sticks' own north for a facing.
fn local_north(facing: Direction) -> Direction {
    match facing {
        Direction::Down => Direction::North,
        Direction::Up => Direction::South,
        _ => Direction::Up,
    }
}

/// PunjiBlock.getLocalEast: the sticks' own east for a facing.
fn local_east(facing: Direction) -> Direction {
    match facing {
        Direction::Down | Direction::Up => Direction::East,
        facing => facing.clockwise(),
    }
}

/// PunjiBlock.isConnected: the block there is the punji itself with the same facing.
fn connected(view: &dyn BlockView, facing: Direction, target: [i32; 3]) -> bool {
    view.block_state(target).is_some_and(|state| state.block == id("punji") && facing_of(&state) == facing)
}

/// PunjiBlock.canSurvive: the block the sticks point away from holds them with a sturdy face.
pub(crate) fn punji_survives(state: &BlockState, pos: [i32; 3], view: &dyn BlockView) -> bool {
    let facing = facing_of(state);
    view.is_face_sturdy(facing.relative(pos), facing.opposite())
}

/// The placed state of a punji facing `facing` at `pos`, connections read from the world.
fn punji_state(view: &dyn BlockView, pos: [i32; 3], facing: Direction) -> BlockState {
    let north = local_north(facing);
    let east = local_east(facing);
    let north_pos = north.relative(pos);
    BlockState::new(id("punji"))
        .with("facing", facing.name())
        .with("north", bool_text(connected(view, facing, north_pos)))
        .with("east", bool_text(connected(view, facing, east.relative(pos))))
        .with("northeast", bool_text(connected(view, facing, east.relative(north_pos))))
        .with("northwest", bool_text(connected(view, facing, east.opposite().relative(north_pos))))
}

/// PunjiBlock.getStateForPlacement: face the way the clicked face points away, else the first side in
/// Direction order that can hold the sticks, else refuse the placement.
pub(crate) fn punji_placement(context: &PlacementContext<'_>) -> Placement {
    let first = context.face.opposite();
    let order = [first].into_iter().chain(Direction::ALL.into_iter().filter(move |side| *side != first));
    for facing in order {
        let probe = BlockState::new(id("punji")).with("facing", facing.name());
        if punji_survives(&probe, context.pos, context.view) {
            return Placement::State(punji_state(context.view, context.pos, facing));
        }
    }
    Placement::Refuse
}

/// PunjiBlock.updateShape: the sticks fall when their support goes; the north and east joins follow the
/// neighbour that changed, and the two diagonal joins are always read again.
pub(crate) fn punji_update(context: &ShapeUpdateContext<'_>) -> Option<BlockState> {
    let facing = facing_of(&context.state);
    if context.direction == facing && !punji_survives(&context.state, context.pos, context.view) {
        return Some(air());
    }
    let north = local_north(facing);
    let east = local_east(facing);
    let mut state = context.state.clone();
    if context.direction == north {
        state = state.with("north", bool_text(connected(context.view, facing, context.neighbour_pos)));
    } else if context.direction == east {
        state = state.with("east", bool_text(connected(context.view, facing, context.neighbour_pos)));
    }
    let north_pos = north.relative(context.pos);
    state = state.with("northeast", bool_text(connected(context.view, facing, east.relative(north_pos))));
    state = state.with("northwest", bool_text(connected(context.view, facing, east.opposite().relative(north_pos))));
    Some(state)
}

// --- The hanging ichor cake (InvertedCakeBlock) ---

/// InvertedCakeBlock.canSurvive: the block above is solid. The port reads that as a sturdy underside,
/// the face the cake hangs from (the vanilla lantern's rule).
pub(crate) fn ichor_survives(view: &dyn BlockView, pos: [i32; 3]) -> bool {
    view.is_face_sturdy([pos[0], pos[1] + 1, pos[2]], Direction::Down)
}

/// InvertedCakeBlock.updateShape: only the support above matters.
pub(crate) fn ichor_update(context: &ShapeUpdateContext<'_>) -> Option<BlockState> {
    if context.direction == Direction::Up && !ichor_survives(context.view, context.pos) {
        return Some(air());
    }
    Some(context.state.clone())
}

// --- Eating the cakes (FoodCakeBlock.use / eatSlice) ---

/// A right click on a cake eats a slice, whatever the hand holds, as FoodCakeBlock.use does; any other
/// block passes. A refused slice (the BLOCK combination, below) consumes the click instead of passing:
/// passing would hand the click to the vanilla cake template, which would eat the slice its own way.
fn eat_use(used: &UseItemOnBlockContext) -> ActionResult {
    let world = World::new(&used.dimension);
    let Some(state) = world.get_block_state(used.pos) else { return ActionResult::Pass };
    let Some(name) = state.block.to_string().strip_prefix("tconstruct:").map(str::to_owned) else { return ActionResult::Pass };
    let Some(cake) = cake(&name) else { return ActionResult::Pass };
    eat_slice(&world, used, &state, &cake)
}

fn eat_slice(world: &World, used: &UseItemOnBlockContext, state: &BlockState, cake: &Cake) -> ActionResult {
    let player = &used.player;
    let held = player.effects().into_iter().find(|effect| effect.id == cake.effect);
    // Every cake food can always be eaten, so Java's canEat check never refuses. The BLOCK combination
    // refuses while the player already holds the food's effect with at least 100 ticks left.
    if !cake.add && held.as_ref().is_some_and(|effect| effect.duration >= 100) {
        return ActionResult::Consume;
    }
    // The EAT_CAKE_SLICE statistic is not awarded: no statistic is exposed (see the module note).
    player.add_food(cake.nutrition, cake.saturation);
    // The original rolls the level's random against the effect's chance, which is 1.0 for every cake, so
    // the effect lands on any roll; with no roll to take, a certain effect still lands and an uncertain
    // one is not invented.
    let applies = match world.random_float() {
        Some(roll) => roll < cake.chance,
        None => cake.chance >= 1.0,
    };
    if applies {
        let mut duration = cake.duration;
        // The ADD combination adds the current duration, on an exact amplifier match.
        if cake.add {
            if let Some(held) = &held {
                if held.amplifier == 0 {
                    duration += held.duration;
                }
            }
        }
        player.add_effect(&EffectInstance::new(cake.effect, duration));
    }
    let bites: i32 = state.property("bites").and_then(|text| text.parse().ok()).unwrap_or(0);
    if bites < 6 {
        world.set_block_state(used.pos, &state.clone().with("bites", &(bites + 1).to_string()));
    } else {
        world.set_block_state(used.pos, &air());
    }
    ActionResult::Success
}
