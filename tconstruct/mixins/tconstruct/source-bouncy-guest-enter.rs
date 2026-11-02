fn tconstruct_bouncy_guest_enter(value: Option<GuestSwap>, game: &mut Game<'_>, host: &mut LanHost, conn: u32) -> Option<GuestSwap> {
    use horizon_world::nbt::Tag;
    if value.is_none() { return value; }
    let sneaking = host.hub.guest(conn).is_some_and(|guest| guest.sneaking);
    let mut enabled = false;
    for stack in game.armor.iter().flatten() {
        if !stack.item.starts_with("tconstruct:") || crate::game::tconstruct_source_ranged_wear::is_broken(stack) { continue; }
        let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { continue; };
        let trait_level = crate::game::tconstruct_source_material_traits::levels(&game.world, stack)
            .and_then(|traits| traits.levels.get("tconstruct:bouncy").copied()).unwrap_or(0);
        let upgrade = match custom.get("tic_upgrades").and_then(|upgrades| upgrades.get("tconstruct:bouncy")) {
            Some(Tag::Int(level)) => *level, _ => 0,
        };
        if trait_level + upgrade > 0 || stack.item == "tconstruct:slime_boots" { enabled = true; break; }
    }
    if let Some(player) = game.player.as_mut() {
        player.shift_key_down = sneaking;
        let state = horizon_client::motion::pluto_landing_state::get_mut(player);
        state.guest = true;
        state.enabled = enabled;
    }
    value
}
