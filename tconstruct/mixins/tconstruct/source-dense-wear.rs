// Apply the source Dense modifier to the already computed durability loss.
fn tconstruct_dense_wear(loss: u32, game: &mut crate::game::Game<'_>, stack: &ItemStack) -> u32 {
    use horizon_core::random::RandomSource;
    if loss == 0 || !stack.item.starts_with("tconstruct:") { return loss; }
    let Some(traits) = crate::game::tconstruct_source_material_traits::levels(&game.world, stack) else { return loss; };
    let level = traits.levels.get("tconstruct:dense").copied().unwrap_or(0);
    if level <= 0 { return loss; }
    let probability = crate::game::tconstruct_source_material_effects::dense_reduction_probability(level);
    (0..loss).filter(|_| game.world.random.next_float() >= probability).count() as u32
}
