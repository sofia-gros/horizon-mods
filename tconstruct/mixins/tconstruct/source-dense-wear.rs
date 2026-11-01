// Apply supported source durability modifiers to normal tool and armor wear.
fn tconstruct_dense_wear(loss: u32, game: &mut crate::game::Game<'_>, stack: &ItemStack) -> u32 {
    use horizon_core::random::RandomSource;
    use horizon_world::nbt::Tag;
    if loss == 0 || !stack.item.starts_with("tconstruct:") { return loss; }
    let traits = crate::game::tconstruct_source_material_traits::levels(&game.world, stack);
    let trait_level = |id| traits.as_ref().and_then(|v| v.levels.get(id).copied()).unwrap_or(0);
    let dense = trait_level("tconstruct:dense");
    // Armor's native caller supplies immutable snapshots; its scoped context
    // records the shield to be committed to the matched equipment slot.
    let actor = game.lan_acting;
    let cause = game.world.chunks.__horizon_mixins_tconstruct_modifier_wear_cause().get(&actor).cloned();
    let cause = cause.as_deref();
    let armor = game.world.chunks.__horizon_mixins_tconstruct_armor_wear_context().contains_key(&actor)
        && !crate::game::tconstruct_source_ranged_wear::is_broken(stack);
    let loss = if armor { crate::game::tconstruct_source_material_effects::tanned_wear(loss, trait_level("tconstruct:tanned"), cause) } else { loss };
    let (loss, used) = if armor {
        let shield = crate::game::tconstruct_source_overslime::amount(stack, "tic_overslime") as u32;
        let (remaining, residual) = crate::game::tconstruct_source_overslime::absorb(shield, loss, cause);
        (residual, shield - remaining)
    } else { (loss, 0) };
    if used > 0 {
        game.world.chunks.__horizon_mixins_tconstruct_overslime_pending_mut().entry(actor).or_default().push((stack.clone(), used));
    }
    let upgrades = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_upgrades"));
    let amount = |id| match upgrades.and_then(|u| u.get(id)) { Some(Tag::Int(n)) => *n, _ => 0 };
    crate::game::tconstruct_source_material_effects::reinforced_wear(loss, dense,
        amount("tconstruct:reinforced") + 4 * trait_level("tconstruct:reinforced"), amount("tconstruct:unbreakable") > 0 || trait_level("tconstruct:unbreakable") > 0,
        cause, || game.world.random.next_float())
}
