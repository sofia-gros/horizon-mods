//! Independent natural-start consumer. No native types or host injections.
use pluto_lib::{ConfiguredStructure,Identifier,StructurePiece,StructureStart,StructureTypeRegistry};

pub fn register() {
    let resume=std::env::var("PLUTO_PROBE_NATURAL_RESUME").as_deref()==Ok("1");
    assert!(StructureTypeRegistry::register(Identifier::parse("pluto_probe:natural_structure").unwrap(),move |source| {
        let case=source.get("probe_case").and_then(|v|v.as_str()).ok_or("missing natural probe case")?.to_owned();
        let origin:[i32;3]=serde_json::from_value(source.get("probe_origin").cloned().ok_or("missing natural probe origin")?)
            .map_err(|_|"invalid natural probe origin")?;
        Ok(ConfiguredStructure::new(move |context| {
            if context.chunk!=[0,0] {return Ok(None)}
            if resume {
                println!("PLUTO-PROBE natural unexpected_replan case={case} chunk={:?}",context.chunk);
                return Err("fresh planning at saved probe chunk is forbidden".into())
            }
            println!("PLUTO-PROBE natural attempt case={case} chunk={:?}",context.chunk);
            let Some(position)=context.world.top_of_chunk_center(context.chunk)? else {
                println!("PLUTO-PROBE natural declined case={case} reason=column");return Ok(None)
            };
            if !context.world.is_valid_biome(position)? {
                println!("PLUTO-PROBE natural declined case={case} reason=biome");return Ok(None)
            }
            if case.starts_with("reject") {println!("PLUTO-PROBE natural declined case={case} reason=consumer");return Ok(None)}
            let roll=context.world.random_int(7)?;
            if !(0..7).contains(&roll) {return Err("invalid natural random result".into())}
            let kind=if case=="marker" {"pluto_probe:marker_piece"} else {"pluto_lib:template_piece"};
            println!("PLUTO-PROBE natural planned case={case} position={position:?} roll={roll} kind={kind}");
            let mut configuration=serde_json::json!({"template":"pluto_probe:colors","origin":origin,
                "rotation":"NONE","mirror":"NONE"});
            if case=="marker" {
                configuration["template"]=serde_json::json!("pluto_probe:marker");
                configuration["case"]=serde_json::json!("natural_marker");
                configuration["initial_seen"]=serde_json::json!(0);
            }
            Ok(Some(StructureStart {position,pieces:vec![StructurePiece {
                kind:Identifier::parse(kind).unwrap(),configuration,
            }]}))
        }))
    }));
}
