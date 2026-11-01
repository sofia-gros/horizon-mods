fn tconstruct_armor_wear_end(value: (), game: &mut crate::game::Game<'_>) -> () {
    let actor = game.lan_acting;
    let Some(losses) = game.world.chunks.__horizon_mixins_tconstruct_armor_wear_context_mut().remove(&actor) else { return value; };
    let selected = crate::game::tconstruct_source_armor_wear::matched_losses(&game.armor, &losses);
    let mut broke = false;
    for (index, loss) in selected {
        if let Some(stack) = game.armor[index].as_mut() {
            let before = crate::game::tconstruct_source_ranged_wear::is_broken(stack);
            crate::game::tconstruct_tx::fixed_damage(stack, loss);
            broke |= !before && crate::game::tconstruct_source_ranged_wear::is_broken(stack);
        }
    }
    if broke { game.apply_equipment(); }
    value
}
