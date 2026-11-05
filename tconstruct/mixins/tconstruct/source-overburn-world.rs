use horizon_core::random::RandomSource;
use super::{tconstruct_source_overburn as rule, Game};

pub(super) fn tick(game: &mut Game<'_>) {
    if game.remote.is_some()
        || !game
            .player
            .as_ref()
            .is_some_and(|p| !p.health.dead_or_dying())
    {
        return;
    }
    let actor = game.native_acting_id();
    let phase = rule::advance(
        game.world
            .chunks
            .__horizon_mixins_tconstruct_overburn_phase_mut()
            .entry(actor)
            .or_default(),
    );
    // All supported levels use a multiple of five ticks.
    if phase % 5 != 0 {
        return;
    }
    let Some(raw) = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()
        .and_then(|v| v.as_ref().ok())
    else {
        return;
    };
    if !rule::known(super::tconstruct_modifier_snapshot::highest_definition(
        raw,
        "tconstruct:tinkering/modifiers/overburn",
    )) {
        return;
    }
    let using = game.using.map(|(slot, _)| slot);
    for slot in (0..game.inventory.len() as i32)
        .chain((100..=103).rev())
        .chain(std::iter::once(-106))
    {
        if super::tconstruct_source_overgrowth::active_slot(slot, using) {
            continue;
        }
        let Some(stack) = game.native_carried_stack(slot).cloned() else {
            continue;
        };
        let Some(tags) = game
            .world
            .chunks
            .__horizon_mixins_tconstruct_resolved_item_tags()
            .as_ref()
            .and_then(|v| v.as_ref().ok())
        else {
            continue;
        };
        if !tags
            .get("tconstruct:modifiable")
            .is_some_and(|t| t.contains(stack.item.as_ref()))
        {
            continue;
        }
        let Some(traits) = super::tconstruct_source_material_traits::levels(&game.world, &stack)
        else {
            continue;
        };
        let Some(level) =
            super::tconstruct_source_material_traits::level_with_upgrade(&stack, &traits, rule::ID)
        else {
            continue;
        };
        let Some(interval) = rule::interval(level) else {
            continue;
        };
        if u32::from(phase) % interval != 0 {
            continue;
        }
        let Some(overworked) = super::tconstruct_source_material_traits::level_with_upgrade(
            &stack,
            &traits,
            "tconstruct:overworked",
        ) else {
            continue;
        };
        // Lookup by fluid identity, even for less than one complete fuel recipe.
        // The planner scales the partial duration exactly as OverburnModule does.
        let mut random = game.world.random.clone();
        let output = super::tconstruct_source_overburn_stack::plan(
            &stack,
            level,
            overworked,
            game.world.game_time,
            |id| {
                super::tconstruct_source_melter_world::fluid_fuel(&game.world, id, u32::MAX).map(
                    |f| rule::Fuel {
                        amount: f.fluid.amount,
                        duration: f.duration,
                        rate: f.rate,
                    },
                )
            },
            || random.next_int_bound(10).ok().map(|v| v as u32),
        );
        if let Some(output) = output {
            // Neither planning nor this write invokes an external callback.
            if let Some(live) = game.native_carried_stack(slot) {
                *live = output;
                game.world.random = random;
            }
        }
    }
}
