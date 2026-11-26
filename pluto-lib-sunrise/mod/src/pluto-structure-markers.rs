// Reuse native template placement/filtering, with TemplateStructurePiece's
// ignore-entities policy and DATA markers returned to the typed outer adapter.
use super::*;

pub fn place(piece:&TemplatePiece,context:&mut PieceContext<'_>)->Result<Option<Markers>> {
    let ignore=Processor::BlockIgnore(piece.ignore.iter().cloned().collect());
    let mut settings=piece.settings();
    settings.ignore_entities=true;
    settings.bounding_box=Some(context.chunk_box);
    settings.processors.push(&ignore);
    settings.processors.extend(piece.processors.0.iter());
    if !context.world.place_template(context.resources,&piece.template,piece.position,
        context.reference,&mut settings,&mut *context.random,2)? {return Ok(None)}
    let markers=filter_blocks(context.resources,&piece.template,piece.position,&settings).into_iter().filter_map(|info| {
        if context.name(info.state)!="minecraft:structure_block" {return None}
        let nbt=info.nbt.as_ref()?;
        let text=|key:&str|match nbt.get(&JavaString::from(key)) {Some(Tag::String(s))=>s.to_string_lossy(),_=>String::new()};
        text("mode").eq_ignore_ascii_case("data").then(||(text("metadata"),info.pos))
    }).collect();
    Ok(Some(markers))
}
