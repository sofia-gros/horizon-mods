// Apply supported source durability modifiers to normal tool and armor wear.
fn tconstruct_dense_wear(loss: u32, game: &mut crate::game::Game<'_>, stack: &ItemStack) -> u32 {
    use horizon_core::random::RandomSource;
    use horizon_world::nbt::Tag;
    if loss == 0 || !stack.item.starts_with("tconstruct:") { return loss; }
    let traits = crate::game::tconstruct_source_material_traits::levels(&game.world, stack);
    let trait_level = |id| traits.as_ref().and_then(|v| v.levels.get(id).copied()).unwrap_or(0);
    let dense = trait_level("tconstruct:dense");
    let upgrades = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_upgrades"));
    let amount = |id| match upgrades.and_then(|u| u.get(id)) { Some(Tag::Int(n)) => *n, _ => 0 };
    // All currently connected call sites represent ordinary wear. A future
    // modifier-originated call must supply its ID to reinforced_cause_bypass.
    crate::game::tconstruct_source_material_effects::reinforced_wear(loss, dense,
        amount("tconstruct:reinforced") + 4 * trait_level("tconstruct:reinforced"), amount("tconstruct:unbreakable") > 0 || trait_level("tconstruct:unbreakable") > 0,
        None, || game.world.random.next_float())
}
