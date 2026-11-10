//! Independent structure consumer; no native types or host injections.
use pluto_lib::{BlockState, ConfiguredStructure, ConfiguredStructurePiece, Identifier, StructurePiece,
    StructurePieceTemplate, StructurePieceTypeRegistry, StructureRotation, StructureMirror, StructureStart, StructureTypeRegistry};

pub fn register() {
    if std::env::var("PLUTO_PROBE_STRUCTURE_MODE").as_deref() != Ok("1") { return; }
    register_pieces();
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

fn register_pieces() {
    assert!(StructurePieceTypeRegistry::register(Identifier::parse("pluto_probe:marker_piece").unwrap(), |configuration| {
        let template=Identifier::parse(configuration.get("template").and_then(|v|v.as_str()).ok_or("missing marker template")?)
            .map_err(|_|"invalid marker template")?;
        let origin=serde_json::from_value(configuration.get("origin").cloned().ok_or("missing marker origin")?).map_err(|_|"invalid marker origin")?;
        let case=configuration.get("case").and_then(|v|v.as_str()).ok_or("missing marker case")?.to_owned();
        let initial=configuration.get("initial_seen").cloned().unwrap_or_else(||serde_json::json!(0));
        let rotation=match configuration.get("rotation").and_then(|v|v.as_str()).unwrap_or("NONE") {
            "NONE"=>StructureRotation::None,"CLOCKWISE_90"=>StructureRotation::Clockwise90,
            "CLOCKWISE_180"=>StructureRotation::Clockwise180,"COUNTERCLOCKWISE_90"=>StructureRotation::Counterclockwise90,
            _=>return Err("invalid probe rotation".into()),
        };
        let mirror=match configuration.get("mirror").and_then(|v|v.as_str()).unwrap_or("NONE") {
            "NONE"=>StructureMirror::None,"LEFT_RIGHT"=>StructureMirror::LeftRight,"FRONT_BACK"=>StructureMirror::FrontBack,
            _=>return Err("invalid probe mirror".into()),
        };
        Ok(ConfiguredStructurePiece::new(StructurePieceTemplate {
            template,origin,rotation,mirror,
        },serde_json::json!({"seen":initial}),move |context| {
            let before=context.state["seen"].as_u64().ok_or("invalid probe marker state")?;
            if case=="error" {
                println!("PLUTO-PROBE marker case={case} before={before} rejected=true");
                return Err("probe deliberately failed marker callback".into());
            }
            let roll=context.world.random_int(7)?;
            if !(0..7).contains(&roll) {return Err("invalid marker RNG result".into())}
            let state=BlockState::parse("minecraft:oak_log[axis=x]").unwrap();
            context.world.set_block_state(context.position,&state,2)?;
            let actual=context.world.block_state(context.position)?;
            if actual.block!=state.block || actual.property("axis")!=Some("x") {return Err("marker write/property mismatch".into())}
            context.state["seen"]=serde_json::json!(before+1);
            println!("PLUTO-PROBE marker case={case} pos={:?} before={before} after={} roll={roll}",context.position,before+1);
            Ok(())
        }).with_state_validator(|state| {
            if state.get("seen").and_then(|v|v.as_u64()).is_some_and(|n|n<=16) {Ok(())}
            else {Err("probe rejected saved marker state".into())}
        }))
    }));
    // Wrap the actual Tinkers codec only to observe its state; do not replace
    // its RNG, world writes, marker order, success count or error result.
    assert!(StructurePieceTypeRegistry::register(Identifier::parse("pluto_probe:island_piece").unwrap(), |configuration| {
        let original=StructurePieceTypeRegistry::decode(&Identifier::parse("tconstruct:island_piece").unwrap(),configuration)?
            .ok_or("production island piece codec not registered")?;
        let validator=original.clone();
        Ok(ConfiguredStructurePiece::new(original.template().clone(),original.initial_state().clone(),move |context| {
            let before=context.state["trees_placed"].as_i64().ok_or("invalid island probe state")?;
            let result=original.handle_data_marker(context);
            println!("PLUTO-PROBE island_marker marker={} pos={:?} before={before} after={} ok={}",
                context.marker,context.position,context.state["trees_placed"],result.is_ok());
            result
        }).with_state_validator(move |state|validator.validate_state(state)))
    }));
}
