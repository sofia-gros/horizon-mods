const ATTRIBUTE: &str = "minecraft:oxygen_bonus";
const MODIFIER: &str = "tconstruct:turtles_grace/respiration";

pub(super) fn refresh(game: &mut super::Game<'_>) {
    // Clients receive the authoritative attribute. Adding locally would double
    // the same value and is not needed for the existing breathing path.
    if game.remote.is_some() { return; }
    let wanted = super::tconstruct_turtles_grace::bonus(&game.world,&game.armor).map(f64::from);
    let Some(player) = game.player.as_mut() else { return; };
    let key = player.attributes.key(ATTRIBUTE);
    let Ok(present) = player.attributes.has_modifier(key.clone(),MODIFIER) else { return; };
    if !present && wanted.is_none() { return; }
    if present && wanted.is_some() && player.attributes.modifier_value(key,MODIFIER).ok() == wanted { return; }
    let _ = player.set_modifier(ATTRIBUTE,MODIFIER,wanted.unwrap_or(0.0),"add_value",wanted.is_some());
}
