fn tconstruct_armor_wear_end(value: (), game: &mut crate::game::Game<'_>) -> () {
    let actor = game.lan_acting;
    let Some(losses) = game.world.chunks.__horizon_mixins_tconstruct_armor_wear_context_mut().remove(&actor) else { return value; };
    let mut used = [false; 4];
    let mut broke = false;
    for (original, loss) in losses {
        if let Some((index, slot)) = game.armor.iter_mut().enumerate().find(|(index, slot)| !used[*index] && slot.as_ref() == Some(&original)) {
            used[index] = true;
            if let Some(stack) = slot.as_mut() {
                let before = crate::game::tconstruct_source_ranged_wear::is_broken(stack);
                crate::game::tconstruct_tx::fixed_damage(stack, loss);
                broke |= !before && crate::game::tconstruct_source_ranged_wear::is_broken(stack);
            }
        }
    }
    if broke { game.apply_equipment(); }
    value
}
