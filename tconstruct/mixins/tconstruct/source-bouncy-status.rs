// TinkerEffects Bouncy contributes +1 to the boolean bounce attribute.
// Source: TConstruct f4e77d43 (MIT); existing third-party notice retained.
fn tconstruct_bouncy_status(value: (), game: &mut crate::game::Game<'_>) {
    // Recompute after every normal effect-status update, including removal,
    // expiration, playerdata restore, remote packets and authoritative guests.
    let enabled = game.potions.effects.get("tconstruct:bouncy").is_some();
    if let Some(player) = game.player.as_mut() {
        player.status.bounce_on_fall = enabled;
        player.status.bounce_sound = enabled.then_some("tconstruct:slimy_bounce");
    }
    value
}
