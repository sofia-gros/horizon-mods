use crate::game::{Game, tconstruct_source_looting as rules};
use horizon_client::mobs::{DamageKind, Mob, Who};
use horizon_items::items::ItemStack;
use horizon_world::loot::LootEntity;

fn levels(game: &Game<'_>, stack: &ItemStack, armor: bool) -> Option<[i32; 3]> {
    if stack.count != 1 || stack.wire_patch().is_some() {
        return None;
    }
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    let traits = crate::game::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let mut result = [0; 3];
    for (index, id) in [
        "tconstruct:looting",
        "tconstruct:looter",
        "tconstruct:fortunate",
    ]
    .into_iter()
    .enumerate()
    {
        if !armor && index != 0 {
            continue;
        }
        let level =
            crate::game::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, id)?;
        if level > 0 {
            let path = format!(
                "tconstruct:tinkering/modifiers/{}",
                id.strip_prefix("tconstruct:")?
            );
            if !rules::known(
                id,
                crate::game::tconstruct_modifier_snapshot::highest_definition(raw, &path),
            ) {
                return None;
            }
        }
        result[index] = level;
    }
    Some(result)
}

fn projected_level(game: &Game<'_>, killer: &LootEntity) -> Option<i32> {
    let held = game.inventory.get(game.selected)?.as_ref()?;
    // The native snapshot must still describe the actual selected mainhand.
    // Refuse a departed/replaced actor or changes made by an earlier hook.
    let recorded = killer.item("mainhand")?;
    if held.count == 0 || held.wire_patch().is_some() || *recorded != super::to_loot(held) {
        return None;
    }
    let mut seen = std::collections::HashSet::new();
    if recorded
        .enchantments
        .iter()
        .any(|(id, _)| !seen.insert(id.as_str()))
    {
        return None;
    }
    let base = recorded
        .enchantments
        .iter()
        .find(|(id, _)| id == "minecraft:looting")
        .map_or(0, |(_, level)| *level);
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    let modifiable = tags.get("tconstruct:modifiable")?;
    let melee = tags.get("tconstruct:modifiable/melee")?;
    let worn = tags.get("tconstruct:modifiable/armor/worn")?;
    let mut bonuses = [0; 5];
    if held.item.starts_with("tconstruct:")
        && modifiable.contains(held.item.as_ref())
        && melee.contains(held.item.as_ref())
        && crate::game::tconstruct_source_melee_shape::intact(held).is_some()
    {
        bonuses[0] = levels(game, held, false)?[0];
    }
    for (index, stack) in game.armor.iter().enumerate() {
        let Some(stack) = stack else {
            continue;
        };
        if rules::armor_slot(&stack.item) != Some(index)
            || !worn.contains(stack.item.as_ref())
            || !modifiable.contains(stack.item.as_ref())
        {
            continue;
        }
        if crate::game::tconstruct_source_melee_shape::intact(stack).is_none() {
            continue;
        }
        let [looting, looter, fortunate] = levels(game, stack, true)?;
        bonuses[index + 1] = rules::armor_bonus(looting, looter, fortunate)?;
    }
    if bonuses == [0; 5] {
        return None;
    }
    rules::total(base, &bonuses)
}

pub(super) fn apply(
    game: &mut Game<'_>,
    victim: &Mob,
    player_uuid: Option<u128>,
    by_player: bool,
    killer: &mut LootEntity,
) {
    if game.remote.is_some() || !by_player || killer.kind != "minecraft:player" || !killer.living {
        return;
    }
    let Some((damage, _)) = victim.last_damage else {
        return;
    };
    // Projectile Looting comes from launch-time modifier capabilities, not
    // whatever melee tool happens to be held at impact. Do not approximate it.
    if damage.kind != DamageKind::PlayerAttack || damage.attacker != Some(Who::Player) {
        return;
    }
    let Some(uuid) = player_uuid else {
        return;
    };
    let Some(actor) = game.player_with_uuid(horizon_client::lan::playerdata::uuid_ints(uuid))
    else {
        return;
    };
    let level = if actor == game.lan_acting {
        projected_level(game, killer)
    } else if let Some(conn) = actor.filter(|_| game.lan_acting.is_none()) {
        let Some(mut host) = game.lan.take() else {
            return;
        };
        let level = game
            .as_guest(&mut host, conn, |guest| projected_level(guest, killer))
            .flatten();
        game.lan = Some(host);
        level
    } else {
        // A host cannot be reconstructed from inside another actor's turn.
        None
    };
    let Some(level) = level else {
        return;
    };
    let Some((_, hand)) = killer
        .equipment
        .iter_mut()
        .find(|(slot, stack)| slot == "mainhand" && !stack.is_empty())
    else {
        return;
    };
    match hand
        .enchantments
        .iter_mut()
        .find(|(id, _)| id == "minecraft:looting")
    {
        Some((_, old)) => *old = level,
        None => hand.enchantments.push(("minecraft:looting".into(), level)),
    }
}
