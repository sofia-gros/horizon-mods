use super::{Bolt, Game};
use crate::game::entity::{Entity as _, EntityHandle, ThunderHit};
use horizon_core::geometry::{Aabb, Vec3};

fn enabled(game: &Game<'_>) -> bool {
    let registry = &game.motion.health.damage;
    let bypasses = registry
        .id("minecraft:lightning_bolt")
        .map(|id| registry.in_tag(id, "minecraft:bypasses_invulnerability"));
    game.remote.is_none() && crate::game::tconstruct_worldbound::blocks_lightning(bypasses)
}

pub(super) fn skip_item(game: &Game<'_>, handle: EntityHandle, stage: ThunderHit) -> bool {
    if !matches!(stage, ThunderHit::Item) || !enabled(game) {
        return false;
    }
    let EntityHandle::Item(index) = handle else {
        return false;
    };
    game.dropped.get(index).is_some_and(|item| {
        crate::game::tconstruct_worldbound_world::protects(game, &item.stack) == Some(true)
    })
}

pub(super) fn record_protected_hits(game: &Game<'_>, bolt: &mut Bolt) {
    if !enabled(game) {
        return;
    }
    let at = bolt.at;
    let strike = Aabb::new(
        Vec3::new(at[0] - 3.0, at[1] - 3.0, at[2] - 3.0),
        Vec3::new(at[0] + 3.0, at[1] + 9.0, at[2] + 3.0),
    );
    // Native thunder_struck skips protected drops only at the item stage.
    // They still belong to hitEntities, not the advancement bystander list.
    // Match the native entity bounds and include drops spawned earlier during
    // this strike. Like native thunder_hit, record a hit on every active flash.
    for item in game.dropped.iter().rev() {
        if strike.intersects(item.thunder_bounds())
            && crate::game::tconstruct_worldbound_world::protects(game, &item.stack) == Some(true)
        {
            bolt.hit_items.push(item.id);
        }
    }
}
