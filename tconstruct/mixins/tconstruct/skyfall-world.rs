use super::{Game, tconstruct_skyfall as rules};

fn levels(game: &Game<'_>) -> Option<([i32; 4], [i32; 4])> {
    if game.remote.is_some() {
        return None;
    }
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !rules::known(super::tconstruct_modifier_snapshot::highest_definition(
        raw,
        "tconstruct:tinkering/modifiers/skyfall",
    )) {
        return None;
    }
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let worn = tags.get("tconstruct:modifiable/armor/worn")?;
    let modifiable = tags.get("tconstruct:modifiable")?;
    let (mut gravity, mut fall) = ([0; 4], [0; 4]);
    for (index, stack) in game
        .armor
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.as_ref().map(|s| (i, s)))
    {
        if super::tconstruct_dragonfall::armor_slot(&stack.item) != Some(index)
            || !worn.contains(stack.item.as_ref())
            || !modifiable.contains(stack.item.as_ref())
        {
            continue;
        }
        if stack.count != 1 || stack.wire_patch().is_some() {
            return None;
        }
        let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
        let level = super::tconstruct_source_material_traits::level_with_upgrade(
            stack,
            &traits,
            rules::ID,
        )?;
        if level < 0 {
            return None;
        }
        // Percent-style gravity uses equipment-change hooks even when broken.
        // Ordinary safe-fall item attributes are disabled on a broken tool.
        gravity[index] = level;
        if super::tconstruct_source_melee_shape::intact(stack).is_some() {
            fall[index] = level;
        }
    }
    Some((gravity, fall))
}

pub(super) fn safe_fall(game: &Game<'_>, dragonfall: f64) -> f64 {
    let fall = levels(game).map(|(_, fall)| fall).unwrap_or([0; 4]);
    rules::combined_safe_fall(dragonfall, &fall).unwrap_or(0.0)
}

pub(super) fn refresh(game: &mut Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let gravity = levels(game).map(|(gravity, _)| gravity).unwrap_or([0; 4]);
    let Some(player) = game.player.as_mut() else {
        return;
    };
    for (index, level) in gravity.into_iter().enumerate() {
        let id = format!("tconstruct:skyfall_gravity/{index}");
        let amount = rules::gravity(level).unwrap_or(0.0);
        let _ = player.set_modifier(
            "minecraft:gravity",
            &id,
            amount,
            "add_multiplied_total",
            amount != 0.0,
        );
    }
}
