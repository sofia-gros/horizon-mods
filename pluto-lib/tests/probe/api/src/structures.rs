//! Independent structure consumer; no native types or direct world writes.
use pluto_lib::{ConfiguredStructure, Identifier, StructurePiece, StructureStart, StructureTypeRegistry};

pub fn register() {
    if std::env::var("PLUTO_PROBE_STRUCTURE_MODE").as_deref() != Ok("1") { return; }
    assert!(StructureTypeRegistry::register(Identifier::parse("pluto_probe:template_structure").unwrap(), |source| {
        let case = source.get("probe_case").and_then(|v| v.as_str()).ok_or("missing probe case")?.to_owned();
        if case == "invalid" { return Err("probe deliberately invalid structure configuration".into()); }
        let pieces: Vec<StructurePiece> = source.get("probe_pieces").and_then(|v| v.as_array()).ok_or("missing probe pieces")?
            .iter().map(|value| {
                Ok(StructurePiece {
                    kind: Identifier::parse(value.get("kind").and_then(|v| v.as_str()).ok_or("missing piece kind")?)
                        .map_err(|e| format!("invalid piece kind: {e:?}"))?,
                    configuration: value.get("configuration").cloned().ok_or("missing piece configuration")?,
                })
            }).collect::<Result<_, String>>()?;
        Ok(ConfiguredStructure::new(move |context| {
            let Some(position) = context.world.top_of_chunk_center(context.chunk)? else { return Ok(None); };
            if !context.world.is_valid_biome(position)? { return Ok(None); }
            let occupied = context.world.first_occupied_height(position[0], position[2])?;
            let depth = context.world.generation_depth();
            let sea = context.world.sea_level();
            let roll = context.world.random_int(7)?;
            if !(0..7).contains(&roll) || depth <= 0 { return Err("invalid native structure query result".into()); }
            println!("PLUTO-PROBE structure case={case} chunk={:?} position={position:?} occupied={occupied} depth={depth} sea={sea} roll={roll}", context.chunk);
            if case == "reject" { return Ok(None); }
            if case == "error" { return Err("probe deliberately failed structure planner".into()); }
            Ok(Some(StructureStart { position, pieces: pieces.clone() }))
        }))
    }));
    // A registered vanilla type must not override a native built-in kind.
    assert!(StructureTypeRegistry::register(Identifier::parse("minecraft:swamp_hut").unwrap(), |_| {
        println!("PLUTO-PROBE forbidden built-in structure override");
        Err("built-in structure kind must bypass custom codecs".into())
    }));
}
