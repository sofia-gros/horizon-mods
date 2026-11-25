// The contact actions' hurt, generalized past the living: the game's own hurt call
// (Game::native_hurt_entity) resolves players and mobs only, where the original's
// Entity.hurt — what a block's entity-inside callback calls, knightmetal's bite the
// case at hand — lands on any entity that takes damage. Living targets keep the hurt
// call's full pipeline (armour, effects, invulnerability time, death and the damage
// hooks) and its refusals; the rest go through the game's damage hurt
// (Game::damage_target, the /damage command's), which covers dropped items,
// vehicles, end crystals, armor stands and the dragon the same way. A dropped stack
// a mod marks indestructible takes nothing, as the original's indestructible item
// entity is invulnerable to a non-bypassing source.

use super::commands::Target;

pub(crate) fn hurt_any(game: &mut super::Game<'_>, entity: u32, amount: f32, damage_type: &str) -> bool {
    if game.native_hurt_entity(game.dim.id(), entity, amount, damage_type, None) {
        return true;
    }
    let Ok(id) = i32::try_from(entity) else { return false };
    let Some(target) = game.level_entity(id) else { return false };
    match target {
        // A living target the hurt call refused (invulnerable, dead, creative,
        // another dimension) keeps the refusal.
        Target::Player | Target::Guest(_) | Target::Mob(_) => false,
        Target::Item(id) if dropped_indestructible(game, id) => false,
        _ => game.damage_target(target, amount, damage_type, None, None, None),
    }
}

// A dropped stack whose item behaviour answers indestructible, asked the way the
// environment tick's guard asks it (pluto-item-indestructible-head).
fn dropped_indestructible(game: &super::Game<'_>, id: u32) -> bool {
    let Some(item) = game.dropped.iter().find(|item| item.id == id) else { return false };
    let data = match item.stack.data().and_then(|d| d.other("minecraft:custom_data")) {
        Some(tag @ sunrise_world::nbt::Tag::Compound(_)) => horizon_client_core::scoreboard::tag_to_json(tag),
        _ => serde_json::Value::Object(serde_json::Map::new()),
    };
    sunrise_world::level::pluto_item_behavior_access::indestructible(&item.stack.item, &data) == Some(true)
}
