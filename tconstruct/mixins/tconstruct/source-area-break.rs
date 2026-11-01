// Use the existing authoritative player break path for each source AOE block.
fn tconstruct_area_break(game: &mut crate::game::Game<'_>, pos: [i32;3]) -> std::ops::ControlFlow<Result<()>> {
    use horizon_world::nbt::Tag;
    use std::ops::ControlFlow::{Break, Continue};
    if game.remote.is_some() { return Continue(()); }
    let actor = game.lan_acting;
    if game.world.chunks.__horizon_mixins_tconstruct_area_active().contains(&actor) { return Continue(()); }
    let effective = |game: &crate::game::Game<'_>, block: &str| -> bool {
        let Some(stack) = game.held() else { return false; };
        if !matches!(stack.item.as_ref(), "tconstruct:sledge_hammer" | "tconstruct:excavator" | "tconstruct:vein_hammer" | "tconstruct:broad_axe" | "tconstruct:scythe") { return false; }
        let Some(custom) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return false; };
        let Some(Tag::Int(max)) = custom.get("tic_max_damage") else { return false; };
        if *max <= 0 || stack.damage >= *max as u32 { return false; }
        let Some(tool) = custom.get("tic_mining_tool") else { return false; };
        crate::game::tconstruct_source_mining_rules::harvestable(tool, block, |tag, block| game.res.items().block_in_tag(tag, block)) == Some(true)
    };
    let Some(tool_name) = game.held().map(|stack| stack.item.to_string()) else { return Continue(()); };
    let state = game.world.block(pos[0], pos[1], pos[2]);
    let facts = game.res.facts().get(state);
    let hardness = facts.hardness;
    let center_name = facts.name.to_string();
    let is_tree = game.res.items().block_in_tag("tconstruct:source_effective/tconstruct/tree_log", &center_name);
    if hardness < 0.0 || !effective(game, &facts.name) { return Continue(()); }
    let (face, protected) = match actor {
        Some(conn) => match game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits().get(&conn) {
            Some((target, face, protected)) if *target == pos => (Some(*face), protected.clone()),
            _ => return Continue(()),
        },
        None => (game.pick().filter(|hit| hit.pos == pos).map(|hit| hit.face), Default::default()),
    };
    enum Area { Box(std::vec::IntoIter<[i32;3]>), Vein(crate::game::tconstruct_source_chain_geometry::Vein), Tree(crate::game::tconstruct_source_chain_geometry::Tree) }
    let area = match tool_name.as_str() {
        "tconstruct:scythe" => { let Some(positions) = face.and_then(|face| crate::game::tconstruct_source_transform_geometry::box_area(pos, face, game.yaw, 2)) else { return Continue(()); }; Area::Box(positions.into_iter()) },
        "tconstruct:vein_hammer" => Area::Vein(crate::game::tconstruct_source_chain_geometry::Vein::new(pos, 2)),
        "tconstruct:broad_axe" if is_tree => Area::Tree(crate::game::tconstruct_source_chain_geometry::Tree::new(pos)),
        "tconstruct:broad_axe" => Area::Box((1..=5).filter_map(|dy| pos[1].checked_add(dy).map(|y| [pos[0],y,pos[2]])).collect::<Vec<_>>().into_iter()),
        _ => { let Some(positions) = face.and_then(|face| crate::game::tconstruct_source_area_shape::plane(pos, face, game.yaw)) else { return Continue(()); }; Area::Box(positions.into_iter()) }
    };
    game.world.chunks.__horizon_mixins_tconstruct_area_active_mut().insert(actor);
    let result = (|| -> Result<()> {
        game.break_block_as_player(pos)?;
        if game.world.block(pos[0], pos[1], pos[2]) == state { return Ok(()); }
        let mut area = area;
        loop {
            // Stop before removing another block if the original tool has broken
            // or the player's held item has changed during the operation.
            let intact = game.held().is_some_and(|stack| {
                stack.item.as_ref() == tool_name.as_str() && stack.data()
                    .and_then(|data| data.other("minecraft:custom_data"))
                    .and_then(|custom| match custom.get("tic_max_damage") { Some(Tag::Int(max)) if *max > 0 => Some(*max as u32), _ => None })
                    .is_some_and(|max| stack.damage < max)
            });
            if !intact { break; }
            let is_box = matches!(&area, Area::Box(_));
            let same = |p: [i32;3]| game.res.facts().get(game.world.block(p[0],p[1],p[2])).name.as_ref() == center_name.as_str();
            let occludes = |p: [i32;3]| game.res.facts().get(game.world.block(p[0],p[1],p[2])).can_occlude;
            let next = match &mut area { Area::Box(iter) => iter.next(), Area::Vein(iter) => iter.next(same), Area::Tree(iter) => iter.next(same, occludes) };
            let Some(extra) = next else { break; };
            if protected.contains(&(extra[0],extra[2])) { continue; }
            let state = game.world.block(extra[0], extra[1], extra[2]);
            if game.res.facts().is_air(state) { continue; }
            let facts = game.res.facts().get(state);
            if is_box && (!crate::game::tconstruct_source_area_shape::compatible_hardness(hardness, facts.hardness) || !effective(game, &facts.name)) { continue; }
            game.break_block_as_player(extra)?;
        }
        Ok(())
    })();
    game.world.chunks.__horizon_mixins_tconstruct_area_active_mut().remove(&actor);
    if let Some(conn) = actor { game.world.chunks.__horizon_mixins_tconstruct_area_guest_hits_mut().remove(&conn); }
    Break(result)
}
