// Read the current world's block tags and controller orientation for the
// shared source cuboid detector. Claims prevent two controllers from using the
// same wall or floor; unloaded chunks and unresolved tags fail closed.
use super::{Game, tconstruct_source_smeltery_structure::{self as structure, Part}};
type Pos = (i32, i32, i32);

pub(super) fn detect(game: &Game<'_>, controller: Pos) -> Option<([i32; 3], [i32; 3], u32)> {
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_block_tags().as_ref()?.as_ref().ok()?;
    let block = game.res.facts().get(game.world.block(controller.0, controller.1, controller.2)).name.clone();
    let foundry = &*block == "tconstruct:foundry_controller";
    let (floor_tag, wall_tag, tank_tag) = if foundry {
        ("tconstruct:foundry/floor", "tconstruct:foundry/wall", "tconstruct:foundry/tanks")
    } else {
        ("tconstruct:smeltery/floor", "tconstruct:smeltery/wall", "tconstruct:smeltery/tanks")
    };
    let floors = tags.get(floor_tag)?;
    let walls = tags.get(wall_tag)?;
    let tanks = tags.get(tank_tag)?;
    let extra_air = tags.get("tconstruct:structure_air");
    let state = game.world.block(controller.0, controller.1, controller.2);
    let facing = match game.pack.assets.registry.state(state)?.properties.get("facing")?.as_str() {
        "north" => [0,-1], "south" => [0,1], "west" => [-1,0], "east" => [1,0], _ => return None,
    };
    let owners = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners();
    let accepts = |p: [i32; 3], part| {
        let pos = (p[0], p[1], p[2]);
        if !game.world.chunks.contains_key(&(p[0] >> 4, p[2] >> 4)) { return false; }
        if part == Part::Wall && pos == controller { return true; }
        let name = game.res.facts().get(game.world.block(p[0], p[1], p[2])).name.clone();
        match part {
            Part::Interior => matches!(&*name, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air") || extra_air.is_some_and(|set| set.contains(&*name)),
            Part::Floor | Part::Wall => {
                if owners.get(&pos).is_some_and(|other| *other != controller) { return false; }
                if part == Part::Floor { floors.contains(&*name) } else { walls.contains(&*name) || tanks.contains(&*name) }
            }
        }
    };
    let (min, max, capacity) = structure::discover([controller.0, controller.1, controller.2], facing, &accepts)?;
    let capacity = if foundry { structure::validate_foundry(min, max, &accepts)? } else { capacity };
    Some((min, max, capacity))
}

pub(super) fn claim(game: &mut Game<'_>, controller: Pos, bounds: Option<([i32; 3], [i32; 3], u32)>) {
    let foundry = &*game.res.facts().get(game.world.block(controller.0, controller.1, controller.2)).name == "tconstruct:foundry_controller";
    let owners = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners_mut();
    owners.retain(|_, owner| *owner != controller);
    let Some((min, max, _)) = bounds else { return; };
    for x in min[0]+1..max[0] { for z in min[2]+1..max[2] { owners.insert((x,min[1],z), controller); } }
    for y in min[1]+1..=max[1] {
        for x in min[0]+1..max[0] { owners.insert((x,y,min[2]), controller); owners.insert((x,y,max[2]), controller); }
        for z in min[2]+1..max[2] { owners.insert((min[0],y,z), controller); owners.insert((max[0],y,z), controller); }
    }
    if foundry {
        for x in min[0]..=max[0] { for z in min[2]..=max[2] {
            if x == min[0] || x == max[0] || z == min[2] || z == max[2] { owners.insert((x,min[1],z), controller); }
        }}
        for y in min[1]+1..=max[1] { for x in [min[0], max[0]] { for z in [min[2], max[2]] {
            owners.insert((x,y,z), controller);
        }}}
    }
}
