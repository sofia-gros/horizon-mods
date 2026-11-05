// The arrow owns the launch-time decision; changing hands/materials later must
// not grant or remove a trait from a projectile already in flight.
use super::Game;
use horizon_client::{
    items::ItemStack,
    mobs::Who,
    projectiles::{Hit, Projectile, ProjectileKind},
};
use horizon_world::nbt::Tag;

const MARKER: &str = "tic_enderference_arrow";
const EFFECT: &str = "tconstruct:enderference";

fn launcher(stack: &ItemStack) -> bool {
    stack.count == 1
        && stack.wire_patch().is_none()
        && matches!(
            stack.item.as_ref(),
            "tconstruct:longbow" | "tconstruct:crossbow"
        )
}

fn mark(stack: &mut ItemStack, enabled: bool) -> Option<()> {
    let mut custom = stack.data()?.other("minecraft:custom_data")?.clone();
    let Tag::Compound(ref mut fields) = custom else {
        return None;
    };
    // Never inherit a stale flag from an item edited/copied from a prior arrow.
    fields.remove(&MARKER.into());
    if enabled {
        fields.insert(MARKER.into(), Tag::Byte(1));
    }
    stack
        .data_mut()
        .set_other("minecraft:custom_data", Some(custom));
    Some(())
}

fn marked(stack: &ItemStack) -> bool {
    launcher(stack)
        && matches!(
            stack
                .data()
                .and_then(|data| data.other("minecraft:custom_data"))
                .and_then(|custom| custom.get(MARKER)),
            Some(Tag::Byte(1))
        )
}

fn eligible(game: &Game<'_>, stack: &ItemStack) -> Option<()> {
    if !launcher(stack)
        || super::tconstruct_source_melee_shape::intact(stack).is_none()
        || !super::tconstruct_source_enderference::supported_definition(&game.world)
        || horizon_client::brewing::effect(EFFECT).is_none()
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
        .contains(stack.item.as_ref())
    {
        return None;
    }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    // The normal source is Chorus material. An opaque purchased entry has no
    // demonstrated acquisition route and must not manufacture this trait.
    if custom.get("tic_upgrades").is_some_and(|upgrades| {
        !matches!(upgrades, Tag::Compound(_)) || upgrades.get(EFFECT).is_some()
    }) {
        return None;
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    (traits.levels.get(EFFECT).copied().unwrap_or(0) > 0).then_some(())
}

// Called last within the existing authoritative material-shot context. Both
// native bow paths snapshot the launcher before launch wear; use that copy,
// not the potentially worn/broken live held item. Stamping last also preserves
// other launch hooks that compare the captured weapon with the live stack.
// No new charge, ammo, durability, RNG or multishot loop is introduced.
pub(super) fn launch(game: &Game<'_>, projectile: &mut Projectile) {
    if game.remote.is_some()
        || projectile.owner != Some(Who::Player)
        || !matches!(projectile.kind, ProjectileKind::Arrow { .. })
    {
        return;
    }
    let Some(arrow) = projectile.arrow.as_mut() else {
        return;
    };
    let Some(weapon) = arrow.fired_from_weapon.as_mut() else {
        return;
    };
    let enabled = eligible(game, weapon).is_some();
    mark(weapon, enabled);
}

// Native impact cancellation is settled before Game::projectile_hit. Applying
// here (rather than arrow_post_hurt_effects) also covers invulnerable/refused
// targets, matching the source's unblocked pre-damage projectile hook.
pub(super) fn impact(game: &mut Game<'_>, projectile: &Projectile, hit: Hit) {
    if game.remote.is_some()
        || projectile.owner != Some(Who::Player)
        || projectile.guest.is_some() && projectile.guest != game.lan_acting
        || !matches!(projectile.kind, ProjectileKind::Arrow { .. })
        || !projectile
            .arrow
            .as_ref()
            .and_then(|arrow| arrow.fired_from_weapon.as_ref())
            .is_some_and(marked)
        || !super::tconstruct_source_enderference::supported_definition(&game.world)
        || horizon_client::brewing::effect(EFFECT).is_none()
    {
        return;
    }
    let Hit::Entity {
        who: Who::Mob(id), ..
    } = hit
    else {
        return;
    };
    let Some(mob) = game
        .mobs
        .iter_mut()
        .find(|mob| mob.id == id && mob.alive() && !mob.removed)
    else {
        return;
    };
    // This bounded mob path has no player/vehicle/armor-stand shield semantics.
    // Shulker is included explicitly because Enderference cancels its teleport.
    if !super::tconstruct_source_spilling::eligible_mob(mob) && mob.kind.id != "minecraft:shulker" {
        return;
    }
    let new = !mob.has_effect(EFFECT);
    mob.add_effect(EFFECT, 100, 0);
    game.mod_effect_added(EFFECT, Some(id as i32), 0, new);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn weapon() -> ItemStack {
        let mut stack = ItemStack::new("tconstruct:longbow", 1);
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([("tic_max_damage", Tag::Int(100))])),
        );
        stack
    }
    #[test]
    fn launch_marker_is_flat_and_does_not_mutate_original_launcher() {
        let original = weapon();
        let mut arrow_weapon = original.clone();
        assert!(!marked(&arrow_weapon));
        assert_eq!(mark(&mut arrow_weapon, true), Some(()));
        assert!(marked(&arrow_weapon));
        assert!(!marked(&original));
        // Later durability changes do not recalculate the in-flight decision.
        arrow_weapon.damage = 100;
        assert!(marked(&arrow_weapon));
        assert_eq!(mark(&mut arrow_weapon, false), Some(()));
        assert!(!marked(&arrow_weapon));
    }
    #[test]
    fn malformed_flags_and_nonlaunchers_do_not_gain_the_effect() {
        let mut stack = weapon();
        stack.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([(MARKER, Tag::Int(1))])),
        );
        assert!(!marked(&stack));
        mark(&mut stack, true);
        stack.count = 2;
        assert!(!marked(&stack));
        let mut vanilla = ItemStack::new("minecraft:bow", 1);
        vanilla.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([(MARKER, Tag::Byte(1))])),
        );
        assert!(!marked(&vanilla));
    }
}
