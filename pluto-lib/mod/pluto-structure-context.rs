// Native structure-attempt queries. This module is attached to the safe
// structure manager, never to structure/start.rs (whose token reprinting fails).
// Explicit placement and native saved-piece loading use these entries. Natural
// start generation and arbitrary custom marker codecs remain unconnected.
use super::*;
use crate::heightmap::HeightmapType;
use crate::level::pluto_structure_access::{self,PlanResult,PlanningWorld};
use horizon_core::random::{LegacyRandom,RandomSource,WorldgenRandom};

// Owned catalog data with weak resource identities, never borrowed contexts.
// The native constructor cannot accept a Mixins state field on this pin. A
// shared resource identity with different source data is therefore unsupported:
// fail closed instead of mixing settings across reloads or concurrent worlds.
static SOURCES:pluto_structure_access::SourceCatalogs<StructureResources>=pluto_structure_access::SourceCatalogs::new();
pub fn capture_sources(data:&Arc<GeneratorData>) {
    let (Some(resources),Some(raw))=(&data.structures,&data.structure_resources) else {return};
    if let Err(error)=SOURCES.capture(resources,&raw.structures,&raw.sets) {
        eprintln!("pluto_lib: structure source capture rejected: {error}");
    }
}

struct Context<'a> {
    native:&'a mut dyn StartContext,
    random:WorldgenRandom<LegacyRandom>,
    biomes:Option<&'a BTreeSet<BiomeId>>,
}
impl PlanningWorld for Context<'_> {
    fn top_of_chunk_center(&mut self,chunk:[i32;2])->Result<Option<[i32;3]>,String> {
        let x=chunk[0].wrapping_mul(16).wrapping_add(8);
        let z=chunk[1].wrapping_mul(16).wrapping_add(8);
        let min=self.native.min_y(); let max=self.native.max_y();
        if !self.native.column_has_valid_biome(x,z,min.wrapping_sub(1),max) {return Ok(None)}
        Ok(Some([x,self.native.base_height(x,z,HeightmapType::WorldSurface).wrapping_sub(1),z]))
    }
    fn is_valid_biome(&mut self,pos:[i32;3])->Result<bool,String> {
        match self.biomes {
            None=>Ok(true),
            Some(biomes)=>Ok(biomes.contains(&self.native.noise_biome(pos.map(|v|v>>2)))),
        }
    }
    fn first_occupied_height(&mut self,x:i32,z:i32)->Result<i32,String> {
        Ok(self.native.base_height(x,z,HeightmapType::WorldSurfaceWg).wrapping_sub(1))
    }
    fn generation_depth(&self)->i32 {self.native.generation().height}
    fn sea_level(&self)->i32 {self.native.sea_level()}
    fn random_int(&mut self,bound:i32)->Result<i32,String> {
        if bound<=0 {return Err("non-positive structure random bound".into())}
        self.random.next_int_bound(bound).map_err(|error|format!("structure random: {error:?}"))
    }
}

/// Begin precisely one native structure attempt. The host's generator normally
/// seeds this large-feature RNG at this boundary; a planner never seeds it.
/// A rejected/failed registered plan must not delegate to another generator.
pub fn plan(
    native:&mut impl StartContext,kind:&str,configuration:&Value,chunk:ChunkPos,
    biomes:Option<&BTreeSet<BiomeId>>,
)->Result<PlanResult,String> {
    native.set_valid_biomes(biomes);
    let mut random=WorldgenRandom::new(LegacyRandom::new(0));
    random.set_large_feature_seed(native.seed(),chunk.x,chunk.z);
    pluto_structure_access::plan(kind,configuration,&mut Context {native,random,biomes},[chunk.x,chunk.z])
}

fn try_start(state:&StructureState,native:&mut impl StartContext,name:&str,chunk:ChunkPos,any_biome:bool)->Result<Option<Option<StructureStart>>,String> {
    let Some(index)=state.registry.structures.iter().position(|s|s.name==name) else {return Ok(None)};
    let def=&state.registry.structures[index];
    let super::super::structure::start::StructureKind::Unsupported(kind)=&def.kind else {return Ok(None)};
    if !pluto_structure_access::is_registered(kind)? {return Ok(None)}
    let snapshot=SOURCES.get(&state.resources)?;
    let source=snapshot.structures.get(name).ok_or_else(||format!("missing structure source snapshot: {name}"))?;
    if source.len()>1_048_576 {return Err("structure configuration exceeds source budget".into())}
    let configuration:Value=serde_json::from_str(source).map_err(|e|format!("invalid structure snapshot: {e}"))?;
    if configuration.get("type").and_then(Value::as_str)!=Some(kind.as_str()) {return Err("structure source type mismatch".into())}
    let biomes=(!any_biome).then_some(&def.biomes);
    match plan(native,kind,&configuration,chunk,biomes)? {
        PlanResult::Unregistered=>Err("registered structure dispatcher disappeared".into()),
        PlanResult::Rejected=>Ok(Some(None)),
        PlanResult::Start(value)=>{
            use super::super::structure::pluto_structure_piece;
            pluto_structure_piece::validate_value(&value)?;
            let object=value.as_object().ok_or("structure plan must be an object")?;
            if object.keys().any(|key|!matches!(key.as_str(),"position"|"pieces")) {return Err("unsupported structure plan field".into())}
            let position:[i32;3]=serde_json::from_value(value.get("position").cloned().ok_or("missing structure plan position")?)
                .map_err(|_|"invalid structure plan position")?;
            if let Some(biomes)=biomes {
                if !biomes.contains(&native.noise_biome(position.map(|v|v>>2))) {return Ok(Some(None))}
            }
            let descriptors=value.get("pieces").and_then(Value::as_array).filter(|v|!v.is_empty()&&v.len()<=256)
                .ok_or("structure start must contain 1..=256 pieces")?;
            // All pieces are bound before a native start is published; a bad
            // later descriptor cannot leave an earlier piece in the world.
            let pieces=descriptors.iter().map(|piece|pluto_structure_piece::decode(&state.resources,piece))
                .collect::<Result<Vec<_>,_>>()?;
            Ok(Some(Some(StructureStart::new(index,def,chunk,pieces))))
        },
    }
}

/// Only unregistered types continue into the native implementation. Rejection,
/// malformed plans and unsupported pieces are handled failures, not fallback.
pub fn explicit_start(state:&StructureState,native:&mut impl StartContext,name:&str,chunk:ChunkPos)->std::ops::ControlFlow<Option<StructureStart>> {
    use std::ops::ControlFlow::{Break,Continue};
    match try_start(state,native,name,chunk,true) {
        Ok(None)=>Continue(()),
        Ok(Some(start))=>Break(start),
        Err(error)=>{eprintln!("pluto_lib: structure {name} rejected: {error}"); Break(None)},
    }
}

/// The native saved-start path owns cache publication/locking. This hook only
/// restores our piece; unrelated native piece types keep their original path.
pub fn saved_piece(state:&StructureState,piece:&crate::nbt::Tag)->std::ops::ControlFlow<Option<Piece>> {
    use std::ops::ControlFlow::{Break,Continue};
    use super::super::structure::pluto_structure_piece;
    let value=tag_json(piece);
    let restored=match value.get("id").and_then(Value::as_str) {
        Some(pluto_structure_piece::TEMPLATE_KIND)=>pluto_structure_piece::restore(&state.resources,&value),
        Some(super::super::structure::pluto_registered_piece::KIND)=>super::super::structure::pluto_registered_piece::restore(&state.resources,&value),
        _=>return Continue(()),
    };
    match restored {
        Ok(piece)=>Break(Some(piece)),
        Err(error)=>{eprintln!("pluto_lib: saved structure piece rejected: {error}"); Break(None)},
    }
}
