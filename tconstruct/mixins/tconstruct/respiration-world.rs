const ATTRIBUTE: &str = "minecraft:oxygen_bonus";
const MODIFIER: &str = "tconstruct:respiration/head";

pub(super) fn refresh(game: &mut super::Game<'_>) {
    // The host synchronizes the authoritative attribute to remote players.
    // Do not add a second client prediction modifier on top of that value.
    if game.remote.is_some() { return; }
    let wanted = super::tconstruct_source_respiration::level(&game.world, game.armor[0].as_ref())
        .map(f64::from);
    let Some(player) = game.player.as_mut() else { return; };
    let key = player.attributes.key(ATTRIBUTE);
    let Ok(present) = player.attributes.has_modifier(key.clone(), MODIFIER) else { return; };
    if !present && wanted.is_none() { return; }
    if present && wanted.is_some() && player.attributes.modifier_value(key, MODIFIER).ok() == wanted { return; }
    let _ = player.set_modifier(ATTRIBUTE, MODIFIER, wanted.unwrap_or(0.0), "add_value", wanted.is_some());
}
