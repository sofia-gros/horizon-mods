// Restore our piece without changing the host's handling of other structures.
fn tconstruct_resume_island(
    state: &StructureState,
    context: &mut impl super::structure::start::StartContext,
    chunk: ChunkPos,
    structure: &str,
    pieces: &[Value],
) -> std::ops::ControlFlow<Result<(), String>> {
    use std::ops::ControlFlow::{Break, Continue};
    if !pieces.iter().any(|p| p.get("id").and_then(Value::as_str) == Some("tconstruct:island_piece")) {
        return Continue(());
    }
    let restore = || -> Result<_, String> {
        let index = state.registry.structures.iter().position(|s| s.name == structure)
            .ok_or_else(|| format!("unknown island structure {structure}"))?;
        if pieces.len() != 1 { return Err(format!("{structure} at {chunk:?}: expected one island piece")); }
        let piece = super::structure::start::tconstruct_slime_island_piece::IslandPiece::from_saved(&state.resources, &pieces[0], super::structure::start::tconstruct_slime_island_start::fungus_replaceable(&state.bindings, &state.resources), context.generation().height)
            .ok_or_else(|| format!("{structure} at {chunk:?}: invalid island piece"))?;
        if piece.structure_name() != structure { return Err(format!("{structure} at {chunk:?}: island structure mismatch")); }
        Ok((index, piece))
    };
    let (index, piece) = match restore() { Ok(value) => value, Err(error) => return Break(Err(error)) };
    // Keep starts from other structure sets, as the normal restore path does.
    let sets: Vec<_> = state.registry.sets.iter()
        .filter(|set| set.structures.iter().any(|(s, _)| *s == index)).collect();
    let mut starts: Vec<_> = state.starts_at(context, chunk).iter().filter(|start| {
        start.structure != index && !sets.iter().any(|set| set.structures.iter().any(|(s, _)| *s == start.structure))
    }).cloned().collect();
    starts.push(Arc::new(StructureStart::new(index, &state.registry.structures[index], chunk,
        vec![Piece::Legacy(Box::new(piece))])));
    state.starts.replace((chunk.x, chunk.z), Arc::new(starts));
    Break(Ok(()))
}
