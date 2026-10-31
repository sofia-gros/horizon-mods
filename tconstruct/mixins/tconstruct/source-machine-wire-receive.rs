fn tconstruct_machine_wire_receive(game: &mut crate::game::Game<'_>, pos: [i32; 3], kind: i32, tag: &horizon_world::nbt::Tag) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let name = game.remote.as_ref().and_then(|r| r.builtin.get("minecraft:block_entity_type")?.name(kind));
    let blocks: &[&str] = match name {
        Some("tconstruct:melter") => &["tconstruct:seared_melter"],
        Some("tconstruct:heater") => &["tconstruct:seared_heater"],
        Some("tconstruct:faucet") => &["tconstruct:seared_faucet"],
        Some("tconstruct:table") => &["tconstruct:seared_table"],
        Some("tconstruct:basin") => &["tconstruct:seared_basin"],
        Some("tconstruct:smeltery") => &["tconstruct:smeltery_controller"],
        Some("tconstruct:fuel_tank") => &["tconstruct:seared_fuel_tank", "tconstruct:seared_fuel_gauge"],
        Some("tconstruct:ingot_tank") => &["tconstruct:seared_ingot_tank", "tconstruct:seared_ingot_gauge"],
        _ => return Continue(()),
    };
    let block = &game.res.facts().get(game.world.block(pos[0], pos[1], pos[2])).name;
    if !blocks.contains(&&**block) { return Break(()); }
    let Some(value) = name.and_then(|name| tag.get(name)).cloned() else { return Break(()); };
    crate::game::tconstruct_source_melter_store::clear_different(&mut game.world, (pos[0], pos[1], pos[2]), name.unwrap());
    if crate::game::tconstruct_source_melter_store::write(&mut game.world, (pos[0], pos[1], pos[2]), name.unwrap(), value) {
        let sy = (pos[1] - horizon_client::chunk::MIN_Y) >> 4;
        if (0..horizon_client::chunk::SECTIONS as i32).contains(&sy) {
            game.world.dirty_sections.insert((pos[0] >> 4, sy, pos[2] >> 4));
        }
    }
    Break(())
}
