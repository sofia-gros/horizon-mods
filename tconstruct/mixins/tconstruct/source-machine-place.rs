fn tconstruct_machine_place(game: &mut crate::game::Game<'_>, pos: [i32; 3], state: horizon_world::registry::BlockStateId, stack: &horizon_client::items::ItemStack) {
    if game.remote.is_some() { return; }
    let name = game.res.facts().get(state).name.clone();
    if &*stack.item != &*name { return; }
    let key = (pos[0], pos[1], pos[2]);
    let chest_slots = match &*name { "tconstruct:crafting_station" => 9, "tconstruct:part_chest" => 256, "tconstruct:tinkers_chest" => 64, _ => 0 };
    if chest_slots != 0 {
        game.world.block_entities.get_or_insert_with(key, || horizon_client::containers::BlockEntity::Chest(vec![None; chest_slots]));
        if &*name == "tconstruct:tinkers_chest" {
            use horizon_world::nbt::{Compound,Tag};
            let mut data=Compound::new();
            if let Some(color)=stack.data().and_then(|data|data.dyed_color) {data.insert("color".into(),Tag::Int(color));}
            crate::game::tconstruct_source_melter_store::clear_different(&mut game.world,key,"tconstruct:modifier_chest");
            let _=crate::game::tconstruct_source_melter_store::write(&mut game.world,key,"tconstruct:modifier_chest",Tag::Compound(data));
        }
        return;
    }
    if matches!(&*name, "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" | "tconstruct:modifier_worktable") {
        game.world.block_entities.get_or_insert_with(key, || horizon_client::containers::BlockEntity::Chest(vec![None; 27]));
        crate::game::tconstruct_source_melter_store::clear_different(&mut game.world, key, "tconstruct:anvil");
        if let Some(value) = stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get("tconstruct:anvil")) {
            let _ = crate::game::tconstruct_source_melter_store::write(&mut game.world, key, "tconstruct:anvil", value.clone());
        }
        return;
    }
    if !crate::game::tconstruct_source_machine_runtime::ensure(game, key, &name) { return; }
    let Some(capacity) = crate::game::tconstruct_source_smeltery_fuel::item_capacity(&name) else { return; };
    let Some(saved) = stack.data().and_then(|data| data.other("minecraft:custom_data")).and_then(|data| data.get("tconstruct:tank")) else { return; };
    let Some(tank) = crate::game::tconstruct_source_melter_state::Tank::from_nbt(saved, capacity) else { return; };
    let Some(kind) = crate::game::tconstruct_source_machine_runtime::kind(&name) else { return; };
    if kind == "tconstruct:casting_tank" {
        let mut state = crate::game::tconstruct_source_casting_tank::CastingTank::new();
        state.tank = tank;
        let _ = crate::game::tconstruct_source_melter_store::write(&mut game.world, key, kind, state.to_nbt());
        return;
    }
    let _ = crate::game::tconstruct_source_melter_store::write(&mut game.world, key, kind, tank.to_nbt());
}
