// Apply supported source durability modifiers to normal tool and armor wear.
fn tconstruct_dense_wear(loss: u32, game: &mut crate::game::Game<'_>, stack: &ItemStack) -> u32 {
    use horizon_core::random::RandomSource;
    use horizon_world::nbt::Tag;
    if loss == 0 || !stack.item.starts_with("tconstruct:") { return loss; }
    let dense = crate::game::tconstruct_source_material_traits::levels(&game.world, stack)
        .and_then(|traits| traits.levels.get("tconstruct:dense").copied()).unwrap_or(0);
    let upgrades = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tic_upgrades"));
    let amount = |id| match upgrades.and_then(|u| u.get(id)) { Some(Tag::Int(n)) => *n, _ => 0 };
    // durability_loss is used by the currently supported ordinary wear paths.
    // Future modifier-driven wear must pass its source cause and bypass tag.
    crate::game::tconstruct_source_material_effects::reinforced_wear(loss, dense,
        amount("tconstruct:reinforced"), amount("tconstruct:unbreakable") > 0,
        false, || game.world.random.next_float())
}
