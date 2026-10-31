// Read the current world's block tags and controller orientation for the
// shared source cuboid detector. Claims prevent two controllers from using the
// same wall or floor; unloaded chunks and unresolved tags fail closed.
use super::{Game, tconstruct_source_smeltery_structure::{self as structure, Part}};
type Pos = (i32, i32, i32);

pub(super) fn detect(game: &Game<'_>, controller: Pos) -> Option<([i32; 3], [i32; 3], u32)> {
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_block_tags().as_ref()?.as_ref().ok()?;
    let floors = tags.get("tconstruct:smeltery/floor")?;
    let walls = tags.get("tconstruct:smeltery/wall")?;
    let tanks = tags.get("tconstruct:smeltery/tanks")?;
    let extra_air = tags.get("tconstruct:structure_air");
    let state = game.world.block(controller.0, controller.1, controller.2);
    let facing = match game.pack.assets.registry.state(state)?.properties.get("facing")?.as_str() {
        "north" => [0,-1], "south" => [0,1], "west" => [-1,0], "east" => [1,0], _ => return None,
    };
    let owners = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners();
    structure::discover([controller.0, controller.1, controller.2], facing, |p, part| {
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
    })
}

pub(super) fn claim(game: &mut Game<'_>, controller: Pos, bounds: Option<([i32; 3], [i32; 3], u32)>) {
    let owners = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners_mut();
    owners.retain(|_, owner| *owner != controller);
    let Some((min, max, _)) = bounds else { return; };
    for x in min[0]+1..max[0] { for z in min[2]+1..max[2] { owners.insert((x,min[1],z), controller); } }
    for y in min[1]+1..=max[1] {
        for x in min[0]+1..max[0] { owners.insert((x,y,min[2]), controller); owners.insert((x,y,max[2]), controller); }
        for z in min[2]+1..max[2] { owners.insert((min[0],y,z), controller); owners.insert((max[0],y,z), controller); }
    }
}
