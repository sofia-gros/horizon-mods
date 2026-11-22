//! The glow block, `tconstruct:glow` (SlimeKnights, MIT; see LICENSE): the block a glow ball
//! leaves behind, from `shared/block/GlowBlock.java` and its registration in TinkerCommons.
//! A 1/16 slab of light (level 14) stuck to the face it hangs from: no collision, replaceable,
//! destroyed by pistons, wool sounds, drops nothing (its loot is `BuiltInLootTables.EMPTY`,
//! so no loot table ships). The content declaration (`data/tconstruct/horizon/block/glow.json`)
//! carries those numbers on the `minecraft:torch` template — the vanilla block with the same
//! no-collision, replaceable, piston-destroyed body — with the per-facing outline shapes from
//! `GlowBlock.getShape` written out and the collision shape empty.
//!
//! Behaviour, from the Java class:
//! - `canSurvive`: the block the glow hangs from (`pos.relative(facing)`) must have a full
//!   sturdy face toward the glow (`isFaceSturdy(support, facing.getOpposite())`), and the
//!   glow's own cell must not hold a liquid. Registered below; the game refuses a placement
//!   that fails it and breaks the block when a neighbour change makes it fail, which is also
//!   what Java's `neighborChanged` does by hand (`removeBlock(pos, false)` — and with an
//!   empty loot table the outcomes agree: nothing drops either way).
//! - `getStateForPlacement`: the first of the placer's nearest looking directions whose
//!   state survives, which is exactly `PlacementContext::nearest_looking_directions`.
//! - The one place the behaviour cannot see is the glow ball's impact (crate::throwables):
//!   an impact callback carries a `World`, not a `BlockView`, so the ball places its
//!   preferred facing and schedules a tick; the scheduled tick below re-checks survival
//!   with the real view and removes the block where Java would have placed nothing (or a
//!   differently-facing glow — the ball's documented reduction).
use pluto_lib::{BlockBehavior, BlockBehaviors, BlockState, BlockView, Direction, Identifier, PistonBehavior, Placement};

fn glow_id() -> Identifier {
    Identifier::new("tconstruct", "glow").expect("valid block id")
}

/// GlowBlock.canSurvive, over any view of the blocks around: sturdy support face, no liquid
/// in the glow's own cell (water through the view; lava by its block, the two vanilla
/// LiquidBlocks). Generic over the view so both the behaviours' `BlockView` and a tick's
/// `BlockEditor` can answer.
pub(crate) fn glow_can_survive<V: BlockView + ?Sized>(view: &V, state: &BlockState, pos: [i32; 3]) -> bool {
    if view.is_water(pos) {
        return false;
    }
    if let Some(here) = view.block_state(pos) {
        if here.block == Identifier::new("minecraft", "lava").expect("valid id") {
            return false;
        }
    }
    let facing = state.property("facing").and_then(Direction::parse).unwrap_or(Direction::Down);
    view.is_face_sturdy(facing.relative(pos), facing.opposite())
}

pub(crate) fn register() {
    BlockBehaviors::register(
        BlockBehavior::new(glow_id())
            .can_survive(|c| glow_can_survive(c.view, &c.state, c.pos))
            .state_for_placement(|c| {
                for direction in c.nearest_looking_directions() {
                    let state = c.state.clone().with("facing", direction.name());
                    if glow_can_survive(c.view, &state, c.pos) {
                        return Placement::State(state);
                    }
                }
                Placement::Refuse
            })
            // The glow ball's self-check (crate::throwables::place_glow): a glow that cannot
            // survive where the ball put it removes itself a tick later.
            .scheduled_tick(|c| {
                if !glow_can_survive(c.world, &c.state, c.pos) {
                    c.world.remove_block(c.pos);
                }
            })
            .piston_behavior(PistonBehavior::Destroy),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{MemoryBlockView, PlacementContext};

    fn glow(facing: &str) -> BlockState {
        BlockState::new(glow_id()).with("facing", facing)
    }

    #[test]
    fn a_glow_survives_on_a_sturdy_face_and_not_on_a_weak_one_or_in_water() {
        let mut view = MemoryBlockView::default();
        let stone = BlockState::new(Identifier::parse("minecraft:stone").unwrap());
        // Facing down: the support is the block below.
        view.blocks.insert([4, 63, 4], stone.clone());
        assert!(glow_can_survive(&view, &glow("down"), [4, 64, 4]));
        view.weak.insert([4, 63, 4]);
        assert!(!glow_can_survive(&view, &glow("down"), [4, 64, 4]), "a slab's top is not a full face");
        view.weak.clear();
        view.water.insert([4, 64, 4]);
        assert!(!glow_can_survive(&view, &glow("down"), [4, 64, 4]), "never in a liquid");
        // Facing east: the support is the block to the east.
        view.water.clear();
        view.blocks.insert([5, 64, 4], stone);
        assert!(glow_can_survive(&view, &glow("east"), [4, 64, 4]));
        assert!(!glow_can_survive(&view, &glow("west"), [4, 64, 4]), "nothing to the west");
    }

    #[test]
    fn placement_takes_the_first_looking_direction_that_survives() {
        let mut view = MemoryBlockView::default();
        view.blocks.insert([4, 63, 4], BlockState::new(Identifier::parse("minecraft:stone").unwrap()));
        // Clicking the floor's top face (face up), looking down: Java tries the clicked face's
        // opposite (down) first, and the floor holds it.
        let context = PlacementContext {
            state: glow("down"),
            pos: [4, 64, 4],
            face: Direction::Up,
            yaw: 0.0,
            pitch: 90.0,
            sneaking: false,
            in_water: false,
            item: "tconstruct:glow".into(),
            replace_clicked: false,
            view: &view,
        };
        // The closure logic, exercised directly: down survives, so it wins over every other direction.
        let mut chosen = None;
        for direction in context.nearest_looking_directions() {
            let state = context.state.clone().with("facing", direction.name());
            if glow_can_survive(context.view, &state, context.pos) {
                chosen = Some(direction);
                break;
            }
        }
        assert_eq!(chosen, Some(Direction::Down));
    }
}
