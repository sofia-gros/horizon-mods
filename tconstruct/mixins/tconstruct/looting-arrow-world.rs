use super::{Game, tconstruct_source_looting as rules};
use horizon_client::{
    mobs::{DamageKind, Mob, Who},
    projectiles::Projectile,
};
use horizon_world::nbt::Tag;
const MARKER: &str = "tic_looting_arrow_level";

fn known(game: &Game<'_>) -> bool {
    game.world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()
        .and_then(|raw| raw.as_ref().ok())
        .is_some_and(|raw| {
            rules::known(
                "tconstruct:looting",
                super::tconstruct_modifier_snapshot::highest_definition(
                    raw,
                    "tconstruct:tinkering/modifiers/looting",
                ),
            )
        })
}

// The existing TiC launch context selected an ordinary arrow. Mark only its
// retained pre-wear firing-stack copy, never the actual launcher or ammunition.
pub(super) fn launch(game: &Game<'_>, p: &mut Projectile) {
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
            .contains(weapon.item.as_ref())
        {
            return None;
        }
        let traits = super::tconstruct_source_material_traits::levels(&game.world, &weapon)?;
        let level = super::tconstruct_source_material_traits::level_with_upgrade(
            &weapon,
            &traits,
            "tconstruct:looting",
        )?;
        let luck = super::tconstruct_source_material_traits::level_with_upgrade(
            &weapon,
            &traits,
            "tconstruct:luck",
        )?;
        let level = rules::arrow_level(level, luck)?;
        let mut custom = weapon.data()?.other("minecraft:custom_data")?.clone();
        let Tag::Compound(ref mut fields) = custom else {
            return None;
        };
        fields.insert(MARKER.into(), Tag::Int(level));
        weapon
            .data_mut()
            .set_other("minecraft:custom_data", Some(custom));
        Some(weapon)
    })();
    if let Some(weapon) = prepared
        && let Some(arrow) = p.arrow.as_mut()
    {
        arrow.fired_from_weapon = Some(weapon);
    }
}

// Native arrow hurt has completed; native death loot follows later. Keep only
// lethal hits and bind the bonus to the exact victim/owner/damage record.
pub(super) fn hit(game: &mut Game<'_>, p: &Projectile, who: Who) {
    if game.remote.is_some()
        || !known(game)
        || p.owner != Some(Who::Player)
        || p.guest.is_some() && p.guest != game.lan_acting
        || game.player.is_none()
    {
        return;
    }
    let Who::Mob(id) = who else {
        return;
    };
    let actor = game.acting_player_uuid();
    if p.owner_uuid.is_some_and(|uuid| uuid != actor) || game.died.contains(&id) {
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
    ) {
        return;
    }
    let Some(Tag::Int(level)) = weapon
        .data()
        .and_then(|data| data.other("minecraft:custom_data"))
        .and_then(|data| data.get(MARKER))
    else {
        return;
    };
    let Some(level) = rules::arrow_level(*level, 0) else {
        return;
    };
    let Some(mob) = game
        .mobs
        .iter()
        .find(|mob| mob.id == id && !mob.alive() && !mob.removed)
    else {
        return;
    };
    let Some((damage, age)) = mob.last_damage else {
        return;
    };
    let from = [p.previous.x, p.previous.y, p.previous.z];
    if damage.kind != DamageKind::Arrow
        || damage.attacker != Some(Who::Player)
        || damage.from != Some(from)
    {
        return;
    }
    let uuid = game.mob_uuid(id);
    let live: std::collections::HashMap<_, _> = game
        .mobs
        .iter()
        .filter(|mob| !mob.alive() && !mob.removed)
        .filter_map(|mob| Some((mob.id, mob.last_damage?.1)))
        .collect();
    let pending = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_looting_arrow_deaths_mut();
    pending.retain(|id, (_, _, age, _, _)| live.get(id).is_some_and(|actual| *actual == *age));
    pending.insert(id, (uuid, actor, age, from, level));
}

// Consume once even if another source or owner replaced the record. A numeric
// entity id alone is never sufficient after unload/reload or entity replacement.
pub(super) fn take(game: &mut Game<'_>, victim: &Mob, player: u128) -> Option<i32> {
    let (uuid, actor, age, from, level) = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_looting_arrow_deaths_mut()
        .remove(&victim.id)?;
    let (damage, actual_age) = victim.last_damage?;
    if !known(game)
        || victim.alive()
        || uuid != game.mob_uuid(victim.id)
        || horizon_client::lan::playerdata::ints_uuid(actor) != player
        || age != actual_age
        || damage.kind != DamageKind::Arrow
        || damage.attacker != Some(Who::Player)
        || damage.from != Some(from)
    {
        return None;
    }
    rules::arrow_level(level, 0)
}
