// Only the non-generic command entry is injected. Feature/jigsaw commands keep
// their original body. Built-in structure starts still use native placement.
use super::*;

pub fn run(game:&mut Game<'_>,id:&str,context:&Context)->std::ops::ControlFlow<R<i32>> {
    if matches!(id,"place.feature"|"place.jigsaw") {return std::ops::ControlFlow::Continue(())}
    std::ops::ControlFlow::Break(place(game,context))
}

fn place(game:&mut Game<'_>,context:&Context)->R<i32> {
    let Some(Arg::Resource(structure))=context.get("structure").cloned() else {return Ok(0)};
    if !game.ui.registries.ids("minecraft:worldgen/structure").contains(&structure) {
        return Err(err("commands.place.structure.invalid",&[&structure]));
    }
    let pos=game.place_pos(context,"pos")?;
    let terrain=game.end.locator().ok_or_else(||err("command.failed",&[]))?;
    let start=terrain.structure_start(&structure,pos[0]>>4,pos[2]>>4).ok_or_else(||err("commands.place.structure.failed",&[]))?;
    let (min,max)=(start.bounding_box.min,start.bounding_box.max);
    game.place_check_loaded([min[0]>>4,min[2]>>4],[max[0]>>4,max[2]>>4])?;
    let registered=horizon_world::level::pluto_structure_level::has_registered(&start);
    let (placed,entities)=game.with_level(|behaviours,level,ticks| {
        if registered {horizon_world::level::pluto_structure_level::place(behaviours,level,ticks,&start)}
        else {behaviours.place_structure(level,ticks,&start)}
    });
    for entity in entities {game.spawn_entity_from_nbt(Tag::Compound(entity));}
    if !placed {return Err(err("commands.place.structure.failed",&[]))}
    game.feedback(Text::translate("commands.place.structure.success",&[
        Text::literal(structure),num(pos[0]),num(pos[1]),num(pos[2]),
    ]));
    Ok(1)
}
