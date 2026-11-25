//! Structure/StructureType planning, kept separate from block-writing pieces.
//! The host still has to dispatch these codecs and restore/place their pieces.
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use serde_json::Value;
use super::super::base::Identifier;
use super::super::block_entity_api_v1::BlockPos;

/// Borrowed queries for one structure attempt. Random draws use that attempt's
/// existing large-feature stream, not the region RNG or a newly seeded stream.
/// Implementations must return errors for unavailable queries, not invented data.
pub trait StructureWorld {
    /// Structure.onTopOfChunkCenter with WORLD_SURFACE: None when the center
    /// column has no possible valid biome. This is not itself a biome approval.
    fn top_of_chunk_center(&mut self, chunk: [i32; 2]) -> Result<Option<BlockPos>, String>;
    /// Test the attempt's native valid-biome predicate at a block position.
    /// Explicit placement uses its native accept-all predicate, not a new policy.
    fn is_valid_biome(&mut self, pos: BlockPos) -> Result<bool, String>;
    /// First occupied WORLD_SURFACE_WG height, rather than the first free Y.
    fn first_occupied_height(&mut self, x: i32, z: i32) -> Result<i32, String>;
    /// ChunkGenerator.getGenDepth, not the maximum build coordinate.
    fn generation_depth(&self) -> i32;
    /// ChunkGenerator.getSeaLevel.
    fn sea_level(&self) -> i32;
    /// One bounded native structure draw. Bound must be positive; errors must not
    /// consume substitute draws or silently return zero.
    fn random_int(&mut self, bound: i32) -> Result<i32, String>;
}

/// Borrowed planning context; no block-writing world is supplied at this phase.
pub struct StructureContext<'a> {
    pub world: &'a mut dyn StructureWorld,
    pub chunk: [i32; 2],
}

/// An owned piece-type identifier and its codec data. These are descriptors,
/// not native pieces or proof that a host supports this piece type. The host
/// must decode every piece before publishing a start or writing its blocks.
#[derive(Clone, Debug, PartialEq)]
pub struct StructurePiece {
    pub kind: Identifier,
    pub configuration: Value,
}

/// Owned generation plan. Position is the generation stub's biome-check origin,
/// not necessarily any template's anchor. Piece codec data carries its anchors.
#[derive(Clone, Debug, PartialEq)]
pub struct StructureStart {
    pub position: BlockPos,
    pub pieces: Vec<StructurePiece>,
}

type Planner = dyn Fn(&mut StructureContext<'_>) -> Result<Option<StructureStart>, String> + Send + Sync;

/// A decoded structure algorithm. None rejects this attempt; an error is not
/// permission to fall back to another algorithm for the same structure type.
#[derive(Clone)]
pub struct ConfiguredStructure { plan: Arc<Planner> }
impl ConfiguredStructure {
    /// Capture validated settings. A planner owns its captured values and cannot
    /// retain borrowed host contexts. Planning must not place blocks.
    pub fn new(plan: impl Fn(&mut StructureContext<'_>) -> Result<Option<StructureStart>, String> + Send + Sync + 'static) -> Self {
        Self { plan: Arc::new(plan) }
    }
    /// Find the candidate and build its owned piece descriptors. Call the native
    /// biome predicate before consuming piece-building RNG when the algorithm
    /// requires that order. This direct call does not connect a generation path.
    pub fn find_generation_point(&self, context: &mut StructureContext<'_>) -> Result<Option<StructureStart>, String> {
        (self.plan)(context)
    }
}

type Codec = Arc<dyn Fn(&Value) -> Result<ConfiguredStructure, String> + Send + Sync>;
static STRUCTURES: RwLock<BTreeMap<String, Codec>> = RwLock::new(BTreeMap::new());

/// StructureType codecs, analogous to Minecraft/Fabric registered structures.
/// Direct decode/planning works; host generation/commands/persistence must have
/// their own adapters. Registration alone does not connect those paths.
pub struct StructureTypeRegistry;
impl StructureTypeRegistry {
    pub(crate) fn is_registered(id:&Identifier)->Result<bool,String> {
        Ok(STRUCTURES.read().map_err(|_|"structure registry poisoned".to_owned())?.contains_key(&id.to_string()))
    }

    /// Register or replace a codec. Decode callbacks and destructors of replaced
    /// captures execute outside the write lock. False reports a poisoned lock.
    pub fn register(id: Identifier, codec: impl Fn(&Value) -> Result<ConfiguredStructure, String> + Send + Sync + 'static) -> bool {
        let codec: Codec = Arc::new(codec);
        let previous = {
            let Ok(mut all) = STRUCTURES.write() else { return false };
            all.insert(id.to_string(), codec)
        };
        drop(previous);
        true
    }
    /// Decode before any context queries. Unknown types return Ok(None); a
    /// registered codec's error stays an error. Existing decoded snapshots keep
    /// their captured settings after registration is replaced.
    pub fn decode(id: &Identifier, configuration: &Value) -> Result<Option<ConfiguredStructure>, String> {
        let codec = STRUCTURES.read().map_err(|_| "structure registry poisoned".to_owned())?.get(&id.to_string()).cloned();
        let Some(codec) = codec else { return Ok(None) };
        codec(configuration).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct World { calls: Vec<&'static str>, valid: bool }
    impl StructureWorld for World {
        fn top_of_chunk_center(&mut self, _: [i32;2]) -> Result<Option<BlockPos>,String> { self.calls.push("center"); Ok(Some([-8,70,24])) }
        fn is_valid_biome(&mut self, _: BlockPos) -> Result<bool,String> { self.calls.push("biome"); Ok(self.valid) }
        fn first_occupied_height(&mut self, _:i32, _:i32) -> Result<i32,String> { Err("no column".into()) }
        fn generation_depth(&self)->i32 {384}
        fn sea_level(&self)->i32 {63}
        fn random_int(&mut self,bound:i32)->Result<i32,String> { self.calls.push("random"); assert_eq!(bound,3); Ok(2) }
    }
    #[test]
    fn codecs_reenter_without_locks_and_snapshots_plan_without_native_fallback() {
        let id = Identifier::parse("example:structure_snapshot").unwrap();
        assert!(StructureTypeRegistry::decode(&id,&Value::Null).unwrap().is_none());
        let replaced = id.clone();
        assert!(StructureTypeRegistry::register(id.clone(),move |value| {
            if value.as_bool()!=Some(true) { return Err("invalid settings".into()); }
            StructureTypeRegistry::register(replaced.clone(), |_| Err("replacement".into()));
            Ok(ConfiguredStructure::new(|context| {
                let Some(position) = context.world.top_of_chunk_center(context.chunk)? else { return Ok(None) };
                if !context.world.is_valid_biome(position)? { return Ok(None) }
                let choice=context.world.random_int(3)?;
                Ok(Some(StructureStart { position, pieces: vec![StructurePiece {
                    kind: Identifier::parse("example:template_piece").unwrap(), configuration: serde_json::json!({"choice":choice}),
                }] }))
            }))
        }));
        assert_eq!(StructureTypeRegistry::decode(&id,&Value::Bool(false)).err().as_deref(),Some("invalid settings"));
        let snapshot=StructureTypeRegistry::decode(&id,&Value::Bool(true)).unwrap().unwrap();
        assert_eq!(StructureTypeRegistry::decode(&id,&Value::Bool(true)).err().as_deref(),Some("replacement"));
        let mut world=World {calls:vec![],valid:false};
        assert_eq!(snapshot.find_generation_point(&mut StructureContext {world:&mut world,chunk:[-1,1]}),Ok(None));
        assert_eq!(world.calls,["center","biome"]);
        world.calls.clear(); world.valid=true;
        let start=snapshot.clone().find_generation_point(&mut StructureContext {world:&mut world,chunk:[-1,1]}).unwrap().unwrap();
        assert_eq!(world.calls,["center","biome","random"]);
        assert_eq!(start.position,[-8,70,24]); assert_eq!(start.pieces[0].configuration["choice"],2);
        let error=ConfiguredStructure::new(|context| {context.world.first_occupied_height(0,0)?; Ok(None)});
        assert_eq!(error.find_generation_point(&mut StructureContext {world:&mut world,chunk:[0,0]}),Err("no column".into()));
    }
}
