// Private official-Horizon adapter. Consumers use pluto_api::fluid instead.
use super::{Game, GameLevel, GuestUse};
use super::pluto_world_fluid_query::{cell, View};
use horizon_client::{first_person::UseResult, items::ItemStack};
use horizon_core::position::BlockPos;
use horizon_world::{
    level::{flags, ticks::Ticks, LevelAccess, Spread},
    registry::BlockStateId,
};
use pluto_api::fluid::{self, contact, Cell, Position, Sample, World};
use std::ops::ControlFlow;

fn pos(p: Position) -> BlockPos {
    BlockPos::new(p[0], p[1], p[2])
}
/// Read custom-fluid geometry without creating a mutable level or flushing
/// pending host changes. Callers supply their own point/bounds and effect policy.
#[allow(dead_code)]
pub(super) fn query_point(game: &Game<'_>, point: [f64; 3]) -> Result<contact::PointContact, contact::QueryError> {
    View::new(&game.world, game.pack.assets.registry, game.res.behaviours()).query_point(point)
}
#[allow(dead_code)]
pub(super) fn query_box(game: &Game<'_>, bounds: contact::Bounds, limits: contact::Limits) -> Result<contact::Contacts, contact::QueryError> {
    View::new(&game.world, game.pack.assets.registry, game.res.behaviours()).query_box(bounds, limits)
}

struct Adapter<'a, 'w> {
    level: &'a mut GameLevel<'w>,
    ticks: &'a mut Ticks,
    spreading_from: Option<BlockPos>,
}
impl World for Adapter<'_, '_> {
    fn sample(&self, p: Position) -> Sample {
        View::new(&*self.level.world, self.level.registry, self.level.behaviours).sample(p)
    }
    fn replace(&mut self, p: Position, expected: &Sample, next: Option<&Cell>) -> bool {
        if !self.level.native_authoritative || &self.sample(p) != expected {
            return false;
        }
        let at = pos(p);
        if let Some(from) = self.spreading_from {
            if from != at && !self.level.allow_spread(Spread::Fluid, from, at) {
                return false;
            }
        }
        let state = match next {
            None => self.level.registry.default_state("minecraft:air"),
            Some(c) if c.level <= 8 => fluid::get(&c.fluid).and_then(|d| {
                let default = self.level.registry.default_state(&d.block)?;
                let mut state = self.level.registry.state(default)?.clone();
                state.properties.insert("level".into(), c.level.to_string());
                self.level.registry.id(&state)
            }),
            Some(_) => None,
        };
        let Some(state) = state else {
            return false;
        };
        let b = self.level.behaviours;
        b.set_block(self.level, self.ticks, at, state, flags::UPDATE_ALL)
    }
    fn schedule(&mut self, p: Position, delay: u16) {
        let at = pos(p);
        if !self.level.is_loaded(at) {
            return;
        }
        let Some((d, _)) = cell(self.level.registry, self.level.block_state(at)) else {
            return;
        };
        self.level
            .native_schedule_block_tick(self.ticks, p, &d.block, i32::from(delay));
    }
}
/// Claim only registered cells before the host dispatch moves its level borrow.
/// The original native observers still run and see handled=true for our cells.
pub(super) fn tick(
    level: &mut GameLevel<'_>,
    ticks: &mut Ticks,
    p: BlockPos,
    state: BlockStateId,
    handled: &mut bool,
) {
    if *handled || !level.native_authoritative {
        return;
    }
    let Some((d, _)) = cell(level.registry, state) else {
        return;
    };
    *handled = true;
    // Definition and optional policy are immutable within the same registry.
    // Poison/missing identity must not silently fall back from explicit routing.
    let Ok(distance) = fluid::registered_slope_distance(&d.id) else { return; };
    let mut world = Adapter { level, ticks, spreading_from: Some(p) };
    match distance {
        Some(distance) => fluid::tick_with_slope(&mut world, [p.x, p.y, p.z], &d, distance),
        None => fluid::tick(&mut world, [p.x, p.y, p.z], &d),
    }
}
pub(super) fn neighbor(level: &GameLevel<'_>, ticks: &mut Ticks, p: BlockPos, state: BlockStateId) {
    if !level.native_authoritative {
        return;
    }
    if let Some((d, _)) = cell(level.registry, state) {
        level.native_schedule_block_tick(ticks, [p.x, p.y, p.z], &d.block, i32::from(d.tick_delay));
    }
}
/// Fluid buckets have no useOn. The authoritative UseItem owns the transaction;
/// the existing host routes the selected hand and LAN actor into held()/with_level.
pub(super) fn use_bucket(
    game: &mut Game<'_>,
    stack: &ItemStack,
) -> ControlFlow<super::Result<UseResult>> {
    use ControlFlow::{Break, Continue};
    let filled = fluid::for_bucket(&stack.item);
    if filled.is_none() && &*stack.item != "minecraft:bucket" {
        return Continue(());
    }
    if game.serving.is_none() && game.options.dedicated.is_none() {
        return Continue(());
    }
    if matches!(game.guest_use, Some(GuestUse::OnBlock(_))) {
        return Break(Ok(UseResult::Pass));
    }
    let hit = if filled.is_some() {
        game.pick()
    } else {
        game.bucket_hit()
    };
    let Some(hit) = hit else {
        return Continue(());
    };
    let (definition, target, pickup) = if let Some(d) = filled {
        (d, hit.beside(), false)
    } else {
        let found = game.with_level(|_, level, _| cell(level.registry, level.block_state(pos(hit.pos))));
        let Some((d, c)) = found else {
            return Continue(());
        };
        if !c.is_source() {
            return Break(Ok(UseResult::Fail));
        }
        (d, hit.pos, true)
    };
    if !game.may_build()
        || stack.count == 0
        || (!pickup && stack.count != 1)
        || !game
            .held()
            .is_some_and(|h| h.count == stack.count && h.same_item_same_components(stack))
        || game
            .items()
            .get(if pickup {
                &definition.bucket
            } else {
                "minecraft:bucket"
            })
            .is_none()
    {
        return Break(Ok(UseResult::Fail));
    }
    let mut cancel = false;
    game.native_fill_bucket(stack, target, &mut cancel);
    if !pickup {
        if let (Some(uuid), Some(name)) = (game.lan_acting_uuid, game.lan_acting_name.clone()) {
            // The vanilla helper guesses water for custom buckets, so pass the
            // actual library identity to this official permission event.
            game.native_bucket_empty(uuid, &name, target, &definition.id, &mut cancel);
        }
    }
    if cancel
        || !game
            .held()
            .is_some_and(|h| h.count == stack.count && h.same_item_same_components(stack))
    {
        return Break(Ok(UseResult::Fail));
    }
    let changed = game.with_level(|_, level, ticks| {
        let mut world = Adapter {
            level,
            ticks,
            spreading_from: None,
        };
        if pickup {
            fluid::pickup_source(&mut world, target, &definition)
        } else {
            fluid::place_source(&mut world, target, &definition)
        }
    });
    if !changed {
        return Break(Ok(UseResult::Fail));
    }
    // Commit inventory before processing fallible events: a successful world
    // write must never leave the original bucket available for a second use.
    let result = ItemStack::new(
        if pickup {
            definition.bucket.as_str()
        } else {
            "minecraft:bucket"
        },
        1,
    );
    if pickup {
        game.filled_result(result.clone());
    } else if !game.creative {
        *game.held_mut() = Some(result);
    }
    game.award_stat("used", &stack.item, 1);
    game.post_block_game_event(
        if pickup {
            "minecraft:fluid_pickup"
        } else {
            "minecraft:fluid_place"
        },
        target,
        Some(horizon_client::mobs::Who::Player),
        None,
    );
    if pickup {
        game.trigger_item(
            "minecraft:filled_bucket",
            &ItemStack::new(&definition.bucket, 1),
        );
    }
    if let Err(error) = game.process_level_events() {
        return Break(Err(error));
    }
    Break(Ok(UseResult::Success { swing: true }))
}
