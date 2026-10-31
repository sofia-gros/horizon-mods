fn tconstruct_machine_remove(level: &mut crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos, old: horizon_world::registry::BlockStateId) {
    let name = &level.facts.get(old).name;
    if &**name == "tconstruct:smeltery_controller" {
        let controller = (pos.x, pos.y, pos.z);
        level.world.chunks.__horizon_mixins_tconstruct_smeltery_owners_mut().retain(|_, owner| *owner != controller);
    }
    if matches!(&**name, "tconstruct:seared_melter" | "tconstruct:seared_heater" | "tconstruct:seared_faucet" | "tconstruct:seared_table" | "tconstruct:seared_basin") {
        crate::game::tconstruct_source_melter_store::clear(level.world, (pos.x, pos.y, pos.z));
    }
    if &**name == "tconstruct:smeltery_controller" {
        crate::game::tconstruct_source_melter_store::clear(level.world, (pos.x, pos.y, pos.z));
    }
}
