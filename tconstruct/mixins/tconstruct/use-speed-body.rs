// Cancellable HEAD: the preserved native body cannot be transformed internally
// by the pinned Mixins version. Only a supported changed speed takes this path.
fn tconstruct_use_speed_head(game: &mut crate::game::Game<'_>) -> std::ops::ControlFlow<()> {
    let native = game.using.and_then(|(slot, _)| game.stack_in(slot))
        .and_then(|stack| game.items().get(&stack.item)).map(|item| item.use_speed);
    let desired = crate::game::tconstruct_use_speed::apply(native, game);
    let Some(speed) = desired.filter(|_| desired != native) else {
        return std::ops::ControlFlow::Continue(());
    };
    // BEGIN PINNED NATIVE BODY

        let glider = game.glider_worn();
        let sleeping = game.sleeping.is_some();
        let item_in_use = game
            .using
            .and_then(|(slot, _)| {
                if slot == shield::OFFHAND_SLOT {
                    game.offhand.as_ref()
                } else {
                    game.inventory.get(slot)?.as_ref()
                }
            })
            .and_then(|s| game.items().get(&s.item));
        let item_use_speed = Some(speed);
        let item_use_slows_sprint = item_in_use.is_some_and(|i| !i.can_sprint);
        let respiration = game.armor[0]
            .as_ref()
            .map_or(0, |s| s.enchantment_level("minecraft:respiration") as i32);
        let leather_boots = game.armor[3]
            .as_ref()
            .is_some_and(|s| &*s.item == "minecraft:leather_boots");
        let freeze_immune = game.wears_freeze_immune();
        let burn_immune = game.wears_burn_from_stepping_immunity();
        let nudge = game.closest_space_nudge();
        let blind = game.potions.effects.get("blindness").is_some();
        let Some(player) = game.player.as_mut() else {
            return std::ops::ControlFlow::Break(());
        };
        player.nudge = nudge;
        player.mobility_restricted = blind;
        player.yaw = game.yaw;
        player.pitch = game.pitch;
        player.auto_jump_enabled = game.settings.auto_jump;
        player.item_use_speed = item_use_speed;
        player.item_use_slows_sprint = item_use_slows_sprint;
        player.respiration = respiration;
        player.glider = glider;
        let _ = player.set_sleeping(sleeping);
        player.leather_boots = leather_boots;
        player.freeze_immune = freeze_immune;
        player.burn_from_stepping_immune = burn_immune;
        player.fluids.fast_lava = game.dim == Dim::Nether;
    // END PINNED NATIVE BODY
    std::ops::ControlFlow::Break(())
}
