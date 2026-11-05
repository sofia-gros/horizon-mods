fn tconstruct_wither_bone_roll(
    mut drops: Vec<LootStack>,
    d: &Death,
    table: &str,
) -> Vec<LootStack> {
    use self::tconstruct_source_wither_bone as proof;
    if table == proof::TABLE && d.loot.has_table(proof::MARKER) {
        for stack in &mut drops {
            if stack.item == "minecraft:bone" {
                let count = stack.get_count();
                // Mantle replaces the stack, not its item id: old components
                // are discarded. No random draw; held equipment is not here.
                *stack = loot_stack("tconstruct:necrotic_bone", count);
            }
        }
    }
    drops
}
