fn tconstruct_slime_seed_use(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_items::items::ItemStack,
    pos: [i32; 3],
    claimed: &mut bool,
) {
    use crate::game::tconstruct_slime_seed_rules::{conversion, ResultKind};
    if *claimed
        || game.remote.is_some()
        || stack.count == 0
        || !game
            .held()
            .is_some_and(|held| held.count > 0 && held.same_item_same_components(stack))
        || (!game.may_build() && !game.can_place_on_in_adventure(pos))
    {
        return;
    }
    let old = game.world.block(pos[0], pos[1], pos[2]);
    let Some(old_state) = game.pack.assets.registry.state(old) else {
        return;
    };
    let Some(result) = conversion(stack.item.as_ref(), &old_state.name) else {
        return;
    };
    let id = match result {
        ResultKind::Grass(foliage, soil) => format!("tconstruct:{foliage}_{soil}_slime_grass"),
        ResultKind::Vine(foliage) => format!("tconstruct:{foliage}_slime_vine"),
    };
    let Some(default) = game.pack.assets.registry.default_state(&id) else {
        return;
    };
    let Some(mut next) = game.pack.assets.registry.state(default).cloned() else {
        return;
    };
    if matches!(result, ResultKind::Vine(_)) {
        for face in ["north", "south", "east", "west", "up"] {
            let Some(value) = old_state.properties.get(face) else {
                return;
            };
            let Some(property) = next.properties.get_mut(face) else {
                return;
            };
            *property = value.clone();
        }
    }
    let Ok(next) = game.pack.assets.registry.resolve(&next) else {
        return;
    };
    let dimension = game.dim.id().to_owned();
    if !game.native_set_block(&dimension, pos, next, 3)
        || game.world.block(pos[0], pos[1], pos[2]) != next
    {
        return;
    }
    if !game.creative {
        let slot = game.held_mut();
        if let Some(held) = slot
            .as_mut()
            .filter(|held| held.same_item_same_components(stack))
        {
            held.count = held.count.saturating_sub(1);
            if held.count == 0 {
                *slot = None;
            }
        }
    }
    game.trigger_block("minecraft:item_used_on_block", pos, stack);
    game.stats.add("used", &stack.item, 1);
    let sound = if matches!(result, ResultKind::Vine(_)) {
        "minecraft:block.grass.place"
    } else {
        "minecraft:block.slime_block.place"
    };
    let at = [
        f64::from(pos[0]) + 0.5,
        f64::from(pos[1]) + 0.5,
        f64::from(pos[2]) + 0.5,
    ];
    game.with_level(|_, level, _| {
        level.native_play_sound(sound, at, 4, 1.0, 1.0);
    });
    *claimed = true;
}
