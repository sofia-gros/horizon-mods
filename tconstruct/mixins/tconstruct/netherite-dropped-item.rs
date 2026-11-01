// Source Netherite uses a fire-immune, damage-resistant dropped entity.
// Horizon's item environment tick handles fire, lava and cactus damage; void
// removal occurs outside this tick and remains unaffected.
fn tconstruct_netherite_dropped_item(
    mut result: crate::entities::ItemEnvironmentTick,
    entity: &mut crate::entities::ItemEntity,
) -> crate::entities::ItemEnvironmentTick {
    use horizon_world::nbt::Tag;
    let netherite = matches!(entity.stack.data().and_then(|d| d.other("minecraft:custom_data"))
        .and_then(|d| d.get("tconstruct:netherite")), Some(Tag::Byte(v)) if *v != 0);
    if netherite {
        entity.health = 5;
        entity.fire_ticks = 0;
        result.alive = true;
        result.lava_hurt = false;
    }
    result
}
