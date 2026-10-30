fn tconstruct_remote_bow_start(
    game: &mut crate::game::Game<'_>,
    held: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let weapon = match &*held.item {
        "tconstruct:longbow" => "minecraft:bow",
        "tconstruct:crossbow" if held.data().is_none_or(|d| d.charged.is_empty()) => {
            "minecraft:crossbow"
        }
        "tconstruct:crossbow" => return Break(()),
        _ => return Continue(()),
    };
    if crate::game::tconstruct_source_ranged_wear::is_broken(held) { return Break(()); }
    if game.using.is_none() && (game.creative || game.projectile_slot(weapon).is_some()) {
        game.using = Some((game.held_slot(), 0));
    }
    Break(())
}
