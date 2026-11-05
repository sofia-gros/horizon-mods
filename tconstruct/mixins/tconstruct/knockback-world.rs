const ATTRIBUTE: &str = "minecraft:attack_knockback";
const MODIFIER: &str = "tconstruct:knockback/mainhand";

fn mainhand_level(game: &super::Game<'_>) -> Option<i32> {
    if game.remote.is_some() { return None; }
    let stack = game.inventory.get(game.selected)?.as_ref()?;
    super::tconstruct_source_knockback::level(&game.world, Some(stack))
}

pub(super) fn level(game: &super::Game<'_>) -> Option<i32> {
    if game.player.as_ref()?.auto_spin_attack_dmg > 0.0 { return None; }
    if game.held() != game.inventory.get(game.selected)?.as_ref() { return None; }
    mainhand_level(game)
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    if game.remote.is_some() { return; }
    let wanted = mainhand_level(game).map(f64::from);
    let Some(player) = game.player.as_mut() else { return; };
    let key = player.attributes.key(ATTRIBUTE);
    let Ok(present) = player.attributes.has_modifier(key.clone(), MODIFIER) else { return; };
    if !present && wanted.is_none() { return; }
    if present && wanted.is_some() && player.attributes.modifier_value(key, MODIFIER).ok() == wanted { return; }
    let _ = player.set_modifier(ATTRIBUTE, MODIFIER, wanted.unwrap_or(0.0), "add_value", wanted.is_some());
}

fn with_bonus(value: (f32, bool), level: i32) -> (f32, bool) {
    if !value.0.is_finite() || value.0 < 0.0 || level <= 0 { return value; }
    (value.0 + level as f32 * 0.5, value.1)
}

pub(super) fn extra(game: &super::Game<'_>, value: (f32, bool)) -> (f32, bool) {
    // The pinned host's primary-mob calculation uses native enchantments and
    // sprint, but does not read attack_knockback. Supply only our contribution.
    level(game).map_or(value, |level| with_bonus(value, level))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_attribute_is_halved_without_replacing_native_sprint_or_enchant() {
        assert_eq!(with_bonus((0.0,false),1), (0.5,false));
        assert_eq!(with_bonus((0.5,true),3), (2.0,true));
        assert_eq!(with_bonus((1.5,false),2), (2.5,false));
        assert_eq!(with_bonus((0.5,true),0), (0.5,true));
    }
}
