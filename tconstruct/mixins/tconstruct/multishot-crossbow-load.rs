fn tconstruct_multishot_load(
    game: &mut crate::game::Game<'_>,
    slot: usize,
    ticks: u32,
    stack: &ItemStack,
) -> std::ops::ControlFlow<()> {
    use crate::game::tconstruct_multishot as source;
    use std::ops::ControlFlow::{Break, Continue};
    if stack.item.as_ref() != "tconstruct:crossbow" || game.remote.is_some() {
        return Continue(());
    }
    let saved = source::saved_level(stack);
    let traits = crate::game::tconstruct_source_material_traits::levels(&game.world, stack);
    let Some(traits) = traits else {
        return if saved == Some(0) {
            Continue(())
        } else {
            Break(())
        };
    };
    let Some(level) = crate::game::tconstruct_source_material_traits::level_with_upgrade(
        stack,
        &traits,
        source::ID,
    ) else {
        return Break(());
    };
    if level == 0 {
        return Continue(());
    }
    let Some(wanted) = source::desired(level) else {
        return Break(());
    };
    if crate::game::tconstruct_source_ranged_wear::is_broken(stack) {
        return Break(());
    }
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()
        .and_then(|v| v.as_ref().ok());
    let definition = raw.and_then(|raw| {
        crate::game::tconstruct_modifier_snapshot::highest_definition(
            raw,
            "tconstruct:tinkering/modifiers/multishot",
        )
    });
    if !source::known_definition(definition) {
        return Break(());
    }
    let held = game.ticks_held(stack, ticks);
    let now = (held + 1) as f32 / Game::charge_ticks(stack).max(1) as f32;
    if now < 1.0 || stack.data().is_some_and(|d| !d.charged.is_empty()) {
        return Continue(());
    }
    // Preserve the existing host's charge clock and sound sequence at the
    // final tick; take over only its ammunition transaction.
    game.using = Some((slot, ticks + 1));
    let sounds = Game::charging_sounds(stack);
    let at = game
        .player
        .as_ref()
        .map(|p| [p.position.x, p.position.y, p.position.z]);
    let (played, start, middle) = charge_sounds(now, game.crossbow_sounds, sounds);
    game.crossbow_sounds = played;
    for sound in [start, middle].into_iter().flatten() {
        game.audio.play(sound, at, 0.5, 1.0);
    }
    let Some(ammo) = game.get_projectile(stack) else {
        return Break(());
    };
    // An external ammo provider owns consumption; do not counterfeit its
    // callbacks. Rockets and virtual survival ammo are outside this port.
    if !game.creative && ammo.slot.is_none() {
        return Break(());
    }
    let slots = std::iter::once(OFFHAND_SLOT)
        .chain(0..game.inventory.len())
        .filter_map(|i| game.stack_in(i).cloned().map(|s| (i, s)))
        .collect::<Vec<_>>();
    let Some(plan) = source::plan(&ammo.stack, ammo.slot, &slots, wanted, game.creative) else {
        return Break(());
    };
    let same = |a: &ItemStack, b: &ItemStack| {
        a.count == b.count && a.damage == b.damage && a.same_item_same_components(b)
    };
    if !game.stack_in(slot).is_some_and(|bow| {
        same(bow, stack) && bow.count == 1 && bow.data().is_none_or(|d| d.charged.is_empty())
    }) || plan.consumed.iter().any(|(i, old, n)| {
        *i == slot
            || !game
                .stack_in(*i)
                .is_some_and(|now| same(now, old) && now.count >= *n)
    }) {
        return Break(());
    }
    // All slots revalidated before the first mutation; no external call can
    // interleave consumption and charged-projectile publication.
    for (i, _, count) in plan.consumed {
        let cell = game.stack_in_mut(i);
        let remaining = cell.as_ref().unwrap().count - count;
        if remaining == 0 {
            *cell = None;
        } else {
            cell.as_mut().unwrap().count = remaining;
        }
    }
    game.stack_in_mut(slot).as_mut().unwrap().data_mut().charged = plan.charged;
    let pitch = 1.0 / (game.world.random.next_float() * 0.5 + 1.0) + 0.2;
    game.audio.play(sounds[2], at, 1.0, pitch);
    Break(())
}
