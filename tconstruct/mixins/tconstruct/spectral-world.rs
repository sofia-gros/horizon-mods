use super::{Game, tconstruct_spectral as rules};
use horizon_client::{items::ItemStack, mobs::Who, projectiles::Projectile};
use horizon_world::nbt::Tag;
const MARKER: &str = "tic_spectral_ticks";

fn known(game: &Game<'_>) -> bool {
    game.world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()
        .and_then(|raw| raw.as_ref().ok())
        .is_some_and(|raw| {
            rules::known(super::tconstruct_modifier_snapshot::highest_definition(
                raw,
                "tconstruct:tinkering/modifiers/spectral",
            ))
        })
}

fn duration(game: &Game<'_>, stack: &ItemStack, tag: &str) -> Option<i32> {
    if !known(game)
        || stack.count != 1
        || stack.wire_patch().is_some()
        || super::tconstruct_source_melee_shape::intact(stack).is_none()
    {
        return None;
    }
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    if !tags.get(tag)?.contains(&*stack.item) {
        return None;
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    rules::duration(
        super::tconstruct_source_material_traits::level_with_upgrade(
            stack,
            &traits,
            "tconstruct:spectral",
        )?,
    )
}

fn glow(game: &mut Game<'_>, target: u32, ticks: i32) {
    let tags = &game.res.stations().enchantments.entity_tags;
    let Some(mob) = game
        .mobs
        .iter_mut()
        .find(|mob| mob.id == target && mob.alive())
    else {
        return;
    };
    if super::potions::mob_can_be_affected(tags, &mob.kind.id, "glowing") {
        mob.add_effect("minecraft:glowing", ticks, 0);
    }
}

pub(super) fn begin(game: &mut Game<'_>, index: usize) {
    if game.remote.is_some() {
        return;
    }
    let actor = game.lan_acting;
    let entry = (|| {
        if !rules::charged(game.attack_strength_scale())
            || game
                .player
                .as_ref()
                .is_none_or(|player| player.auto_spin_attack_dmg > 0.0)
        {
            return None;
        }
        let stack = game.inventory.get(game.selected)?.as_ref()?;
        Some((
            game.mobs.get(index)?.id,
            duration(game, stack, "tconstruct:modifiable/melee")?,
        ))
    })();
    let contexts = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_spectral_context_mut();
    for (owner, pending) in contexts.iter_mut() {
        if *owner == actor {
            *pending = None;
        }
    }
    contexts.push((actor, entry));
}

pub(super) fn end(game: &mut Game<'_>) {
    if game.remote.is_some() {
        return;
    }
    let actor = game.lan_acting;
    let contexts = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_spectral_context_mut();
    if contexts.last().is_some_and(|(owner, _)| *owner == actor) {
        contexts.pop();
    }
}

pub(super) fn hit(game: &mut Game<'_>, index: usize, landed: bool) {
    if game.remote.is_some() {
        return;
    }
    let actor = game.lan_acting;
    let contexts = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_spectral_context_mut();
    let Some((owner, pending)) = contexts.last_mut() else {
        return;
    };
    if *owner != actor {
        return;
    }
    let Some((target, ticks)) = pending.take() else {
        return;
    };
    if landed && game.mobs.get(index).is_some_and(|mob| mob.id == target) && known(game) {
        glow(game, target, ticks);
    }
}

// Projectile launch may follow another modifier's wear. Permit only that wear
// difference, not material/upgrades/slot replacement, when committing the cost.
fn same_launcher(before: &ItemStack, current: &ItemStack) -> bool {
    fn normalized(stack: &ItemStack) -> Option<ItemStack> {
        if stack.wire_patch().is_some() || stack.count != 1 {
            return None;
        }
        let mut stack = stack.clone();
        stack.damage = 0;
        let mut custom = stack.data()?.other("minecraft:custom_data")?.clone();
        let Tag::Compound(ref mut fields) = custom else {
            return None;
        };
        for key in ["tic_overslime", "tic_broken", MARKER] {
            fields.remove(&key.into());
        }
        stack
            .data_mut()
            .set_other("minecraft:custom_data", Some(custom));
        Some(stack)
    }
    normalized(before)
        .zip(normalized(current))
        .is_some_and(|(a, b)| a.same_item_same_components(&b))
}

// Called only from the existing authoritative TiC arrow launch context, after
// Glowing (priority 100). Spectral (priority 10) costs five wear on primary only.
pub(super) fn launch(game: &mut Game<'_>, p: &mut Projectile, primary: bool) {
    if game.remote.is_some() {
        return;
    }
    let Some(mut weapon) = p
        .arrow
        .as_ref()
        .and_then(|arrow| arrow.fired_from_weapon.clone())
    else {
        return;
    };
    if !matches!(
        weapon.item.as_ref(),
        "tconstruct:longbow" | "tconstruct:crossbow"
    ) {
        return;
    }
    let Some(ticks) = duration(game, &weapon, "tconstruct:modifiable/ranged") else {
        return;
    };
    let Some(current) = game
        .held()
        .cloned()
        .filter(|current| same_launcher(&weapon, current))
    else {
        return;
    };
    if primary && !game.creative {
        let (loss, shield) = super::tconstruct_source_overslime_host::damage_with_cause(
            game,
            &current,
            5,
            Some("tconstruct:spectral"),
        );
        let Some(held) = game.held_mut().as_mut().filter(|held| {
            held.damage == current.damage && held.same_item_same_components(&current)
        }) else {
            return;
        };
        super::tconstruct_source_overslime::spend(held, shield);
        super::tconstruct_tx::fixed_damage(held, loss);
    }
    let Some(mut custom) = weapon
        .data()
        .and_then(|data| data.other("minecraft:custom_data"))
        .cloned()
    else {
        return;
    };
    let Tag::Compound(ref mut fields) = custom else {
        return;
    };
    fields.insert(MARKER.into(), Tag::Int(ticks));
    weapon
        .data_mut()
        .set_other("minecraft:custom_data", Some(custom));
    if let Some(arrow) = p.arrow.as_mut() {
        arrow.fired_from_weapon = Some(weapon);
    }
}

pub(super) fn projectile_duration(game: &Game<'_>, p: &Projectile) -> Option<i32> {
    if game.remote.is_some() || !known(game) || p.guest.is_some() && p.guest != game.lan_acting {
        return None;
    }
    let weapon = p.arrow.as_ref()?.fired_from_weapon.as_ref()?;
    if !matches!(
        weapon.item.as_ref(),
        "tconstruct:longbow" | "tconstruct:crossbow"
    ) {
        return None;
    }
    match weapon.data()?.other("minecraft:custom_data")?.get(MARKER)? {
        Tag::Int(ticks) if *ticks > 0 => Some(*ticks),
        _ => None,
    }
}

// Successful ordinary arrow damage is a conservative subset of source's
// unblocked projectile-impact callback; invulnerable/dodged hits are deferred.
pub(super) fn projectile_hit(game: &mut Game<'_>, p: &Projectile, who: Who) {
    let Who::Mob(target) = who else {
        return;
    };
    if let Some(ticks) = projectile_duration(game, p) {
        glow(game, target, ticks);
    }
}
