fn tconstruct_worldbound_environment(
    entity: &mut crate::entities::ItemEntity,
    world: &crate::world::World,
) -> std::ops::ControlFlow<crate::entities::ItemEnvironmentTick> {
    if !world
        .chunks
        .__horizon_mixins_tconstruct_worldbound_environment_items()
        .contains(&entity.id)
    {
        return std::ops::ControlFlow::Continue(());
    }
    // Skip fire, lava, burning and cactus damage before it happens. Preserve
    // current health: protection must not revive an already killed entity.
    // Void removal is in the ordinary item tick and remains unaffected.
    entity.fire_ticks = 0;
    std::ops::ControlFlow::Break(crate::entities::ItemEnvironmentTick {
        alive: entity.health > 0,
        lava_hurt: false,
    })
}
