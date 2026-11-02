fn tconstruct_remote_bow_start(
    game: &mut crate::game::Game<'_>,
    held: &horizon_client::items::ItemStack,
) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    use crate::game::tconstruct_source_bow_start::{decide, Decision};
    let weapon = match &*held.item {
        "tconstruct:longbow" => "minecraft:bow",
        "tconstruct:crossbow" => "minecraft:crossbow",
        _ => return Continue(()),
    };
    let charged = held.data().is_some_and(|d| !d.charged.is_empty());
    let broken = crate::game::tconstruct_source_ranged_wear::is_broken(held);
    let using = game.using.is_some();
    let ammo = if !charged && !broken && !using && !game.creative {
        game.projectile_slot(weapon).is_some()
    } else { false };
    match decide(&held.item, charged, broken, using, game.creative, ammo) {
        Decision::Pass => Continue(()),
        Decision::Reject => Break(false),
        Decision::Handled => Break(true),
        Decision::Start => Break(game.start_using_item()),
    }
}
