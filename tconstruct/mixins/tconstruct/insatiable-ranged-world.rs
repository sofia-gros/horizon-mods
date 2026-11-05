use super::{Game, tconstruct_insatiable as rules};
use horizon_client::{mobs::Who, projectiles::Projectile};
use horizon_world::nbt::Tag;
const MARKER: &str = "tic_insatiable_ranged_level";

fn known(game: &Game<'_>) -> bool {
    game.world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()
        .and_then(|raw| raw.as_ref().ok())
        .is_some_and(|raw| {
            !raw.0
                .contains_key("tconstruct:tinkering/modifiers/insatiable")
        })
}

// The existing launch context has already selected an ordinary TiC arrow.
// Mark its native retained firing-stack copy, never the actual launcher/ammo.
pub(super) fn launch(game: &Game<'_>, p: &mut Projectile, damage: f64) -> f64 {
    let prepared = (|| {
        if game.remote.is_some() || !known(game) {
            return None;
        }
        let mut weapon = p.arrow.as_ref()?.fired_from_weapon.clone()?;
        if !matches!(
            weapon.item.as_ref(),
            "tconstruct:longbow" | "tconstruct:crossbow"
        ) || weapon.count != 1
            || weapon.wire_patch().is_some()
            || super::tconstruct_source_melee_shape::intact(&weapon).is_none()
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
        if !tags
            .get("tconstruct:modifiable/ranged")?
            .contains(&*weapon.item)
        {
            return None;
        }
        let traits = super::tconstruct_source_material_traits::levels(&game.world, &weapon)?;
        let level = super::tconstruct_source_material_traits::level_with_upgrade(
            &weapon,
            &traits,
            "tconstruct:insatiable",
        )?;
        if level <= 0 {
            return None;
        }
        let mut custom = weapon.data()?.other("minecraft:custom_data")?.clone();
        let multiplier = match custom.get("tic_raging_projectile_multiplier") {
            Some(Tag::Float(bits)) => f32::from_bits(*bits),
            _ => f32::NAN,
        };
        let bonus = rules::ranged_bonus(
            game.potions.effects.amplifier(rules::RANGED_EFFECT),
            level,
            multiplier,
        );
        let Tag::Compound(ref mut fields) = custom else {
            return None;
        };
        fields.insert(MARKER.into(), Tag::Int(level));
        weapon
            .data_mut()
            .set_other("minecraft:custom_data", Some(custom));
        Some((weapon, bonus))
    })();
    let Some((weapon, bonus)) = prepared else {
        return damage;
    };
    if let Some(arrow) = p.arrow.as_mut() {
        arrow.fired_from_weapon = Some(weapon);
    }
    bonus
        .and_then(|bonus| rules::ranged_damage_delta(damage, bonus))
        .unwrap_or(damage)
}

// Host routes a guest projectile through its owning player's swapped context.
// Do not fall back to host ownership for a departed guest or mismatched UUID.
pub(super) fn hit(game: &mut Game<'_>, p: &Projectile, who: Who) {
    if game.remote.is_some()
        || !known(game)
        || p.guest.is_some() && p.guest != game.lan_acting
        || game
            .player
            .as_ref()
            .is_none_or(|player| player.health.dead_or_dying())
    {
        return;
    }
    let Who::Mob(target) = who else {
        return;
    };
    if !game.mobs.iter().any(|mob| mob.id == target) {
        return;
    }
    let actor = game.acting_player_uuid();
    if p.owner_uuid.is_some_and(|id| id != actor)
        || !(p.owner == Some(Who::Player) || p.owner.is_none() && p.owner_uuid == Some(actor))
    {
        return;
    }
    let Some(weapon) = p
        .arrow
        .as_ref()
        .and_then(|arrow| arrow.fired_from_weapon.as_ref())
    else {
        return;
    };
    if !matches!(
        weapon.item.as_ref(),
        "tconstruct:longbow" | "tconstruct:crossbow"
    ) || !matches!(weapon.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|custom| custom.get(MARKER)), Some(Tag::Int(level)) if *level > 0)
    {
        return;
    }
    let amplifier = rules::next_amplifier(game.potions.effects.amplifier(rules::RANGED_EFFECT));
    let mut effect = horizon_client::effects::EffectInstance::new(
        rules::RANGED_EFFECT,
        rules::RANGED_DURATION,
        amplifier,
    );
    effect.visible = false;
    game.add_player_effect(effect);
}
