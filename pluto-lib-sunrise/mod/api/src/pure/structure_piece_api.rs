//! TemplateStructurePiece/StructurePieceType contracts, independent of a host.
//! Registered callbacks never retain native worlds or random streams.
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use serde_json::Value;
use super::super::{base::Identifier, block_api_v1::Direction, block_entity_api_v1::BlockPos, world_api_v1::BlockState};

/// A template's block rotation about its configured origin (zero pivot).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StructureRotation {
    #[default]
    None,
    Clockwise90,
    Clockwise180,
    Counterclockwise90,
}

/// The mirror applied before rotation, matching Minecraft's Mirror.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StructureMirror {
    #[default]
    None,
    LeftRight,
    FrontBack,
}

/// Owned immutable placement settings, bound against native templates before
/// any piece of a start writes. Data markers are handled separately; template
/// entities are ignored, as with TemplateStructurePiece.setIgnoreEntities(true).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructurePieceTemplate {
    pub template: Identifier,
    pub origin: BlockPos,
    pub rotation: StructureRotation,
    pub mirror: StructureMirror,
}

/// The real structure post-processing world and random source. Implementations
/// must preserve write clipping, update flags and the supplied piece RNG.
/// Unavailable operations are errors, not guessed states, heights or draws.
pub trait StructurePieceWorld {
    fn block_state(&mut self, pos: BlockPos) -> Result<BlockState, String>;
    fn set_block_state(&mut self, pos: BlockPos, state: &BlockState, flags: i32) -> Result<bool, String>;
    fn face_sturdy(&mut self, pos: BlockPos, face: Direction) -> Result<bool, String>;
    /// True only for a plant with its native/registered survival test passing;
    /// do not treat an arbitrary non-plant's default survival as plant support.
    fn can_survive_plant(&mut self, state: &BlockState, pos: BlockPos) -> Result<bool, String>;
    fn random_int(&mut self, bound: i32) -> Result<i32, String>;
    /// Use the existing RNG's boolean draw, not a substituted nextInt(2).
    fn random_bool(&mut self) -> Result<bool, String>;
    /// Use the same post-processing world and RNG. False means an attempted
    /// feature did not place; an unavailable path must return an error instead.
    fn place_feature(&mut self, feature: &Identifier, pos: BlockPos) -> Result<bool, String>;
}

/// One native DATA marker after template placement, in native marker order and
/// already transformed/clipped to this chunk. State belongs to this piece and
/// is saved separately from immutable codec configuration. Errors preserve
/// earlier writes and state changes; this is not a rollback transaction.
pub struct StructureMarkerContext<'a> {
    pub world: &'a mut dyn StructurePieceWorld,
    pub marker: &'a str,
    pub position: BlockPos,
    pub state: &'a mut Value,
}

type Marker = dyn Fn(&mut StructureMarkerContext<'_>) -> Result<(), String> + Send + Sync;
type StateValidator = dyn Fn(&Value) -> Result<(), String> + Send + Sync;

/// Immutable decoded piece behavior. Clone copies settings/initial state and
/// shares the callback, never a live piece's mutable persisted state. A host
/// creates one state value per actual piece and serializes access to it.
#[derive(Clone)]
pub struct ConfiguredStructurePiece {
    template: StructurePieceTemplate,
    initial_state: Value,
    marker: Arc<Marker>,
    state_validator: Arc<StateValidator>,
}
impl ConfiguredStructurePiece {
    pub fn new(template: StructurePieceTemplate, initial_state: Value,
               marker: impl Fn(&mut StructureMarkerContext<'_>) -> Result<(), String> + Send + Sync + 'static) -> Self {
        Self { template, initial_state, marker: Arc::new(marker), state_validator: Arc::new(|_| Ok(())) }
    }
    /// Add a type-specific saved-state codec check. The default accepts opaque
    /// JSON; host depth/size budgets apply independently of this validator.
    pub fn with_state_validator(mut self, validator: impl Fn(&Value) -> Result<(), String> + Send + Sync + 'static) -> Self {
        self.state_validator=Arc::new(validator);self
    }
    /// Validate state before publishing/restoring a piece or writing a template.
    pub fn validate_state(&self,state:&Value)->Result<(),String> { (self.state_validator)(state) }
    pub fn template(&self) -> &StructurePieceTemplate { &self.template }
    pub fn initial_state(&self) -> &Value { &self.initial_state }
    /// Direct callback execution, not evidence of native marker integration.
    pub fn handle_data_marker(&self, context: &mut StructureMarkerContext<'_>) -> Result<(), String> {
        self.validate_state(context.state)?;
        (self.marker)(context)?;
        self.validate_state(context.state)
    }
}

type Codec = Arc<dyn Fn(&Value) -> Result<ConfiguredStructurePiece, String> + Send + Sync>;
static PIECES: RwLock<BTreeMap<String, Codec>> = RwLock::new(BTreeMap::new());

/// StructurePieceType codecs. Unknown kinds return None; a registered codec's
/// errors remain errors. This registry is not yet connected to native pieces.
pub struct StructurePieceTypeRegistry;
impl StructurePieceTypeRegistry {
    pub fn register(id: Identifier, codec: impl Fn(&Value) -> Result<ConfiguredStructurePiece, String> + Send + Sync + 'static) -> bool {
        let previous = {
            let Ok(mut all) = PIECES.write() else { return false };
            all.insert(id.to_string(), Arc::new(codec) as Codec)
        };
        drop(previous);
        true
    }
    pub fn decode(id: &Identifier, configuration: &Value) -> Result<Option<ConfiguredStructurePiece>, String> {
        let codec = PIECES.read().map_err(|_| "structure piece registry poisoned")?.get(&id.to_string()).cloned();
        codec.map(|codec| codec(configuration)).transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
    fn settings() -> StructurePieceTemplate {
        StructurePieceTemplate { template: id("piece_test:template"), origin: [-8,70,24],
            rotation: StructureRotation::Clockwise90, mirror: StructureMirror::FrontBack }
    }
    struct World { writes: Vec<(BlockPos, BlockState, i32)>, feature_result: Result<bool, String>, features: usize }
    impl StructurePieceWorld for World {
        fn block_state(&mut self, _: BlockPos) -> Result<BlockState,String> { Err("unavailable block".into()) }
        fn set_block_state(&mut self, p: BlockPos, s: &BlockState, f:i32) -> Result<bool,String> { self.writes.push((p,s.clone(),f)); Ok(true) }
        fn face_sturdy(&mut self, _: BlockPos, _: Direction) -> Result<bool,String> { Err("unavailable face".into()) }
        fn can_survive_plant(&mut self, _: &BlockState, _: BlockPos) -> Result<bool,String> { Err("unavailable plant".into()) }
        fn random_int(&mut self, _: i32) -> Result<i32,String> { Err("unavailable random".into()) }
        fn random_bool(&mut self) -> Result<bool,String> { Ok(true) }
        fn place_feature(&mut self, _: &Identifier, _: BlockPos) -> Result<bool,String> { self.features+=1; self.feature_result.clone() }
    }
    #[test]
    fn codecs_reenter_and_snapshots_keep_separate_mutable_piece_state_and_errors() {
        let key=id("piece_test:registered");
        assert!(StructurePieceTypeRegistry::decode(&key,&Value::Null).unwrap().is_none());
        assert!(StructurePieceTypeRegistry::register(key.clone(), |value| {
            if !value.is_null() { return Err("bad piece configuration".into()); }
            assert!(StructurePieceTypeRegistry::register(id("piece_test:other"), |_| Err("other".into())));
            Ok(ConfiguredStructurePiece::new(settings(), serde_json::json!({"placed":0}), |context| {
                if context.marker != "piece_test:feature" { return Ok(()); }
                if context.state["placed"].as_u64().ok_or("bad piece state")? >= 3 { return Ok(()); }
                if context.world.random_bool()? && context.world.place_feature(&id("piece_test:tree"),context.position)? {
                    let n=context.state["placed"].as_u64().unwrap(); context.state["placed"]=Value::from(n+1);
                }
                Ok(())
            }).with_state_validator(|state| {
                if state.get("placed").and_then(Value::as_u64).is_some_and(|n|n<=3) {Ok(())} else {Err("invalid saved piece counter".into())}
            }))
        }));
        assert!(StructurePieceTypeRegistry::decode(&key,&Value::Bool(false)).is_err());
        let decoded=StructurePieceTypeRegistry::decode(&key,&Value::Null).unwrap().unwrap();
        assert!(StructurePieceTypeRegistry::register(key.clone(), |_| Err("replacement".into())));
        assert!(StructurePieceTypeRegistry::decode(&key,&Value::Null).is_err());
        assert_eq!(decoded.template(),&settings());
        let mut a=decoded.initial_state().clone(); let b=decoded.clone().initial_state().clone();
        let mut world=World {writes:vec![],feature_result:Ok(false),features:0};
        let call=|world:&mut World,state:&mut Value| decoded.handle_data_marker(&mut StructureMarkerContext {
            world,marker:"piece_test:feature",position:[-9,71,23],state });
        call(&mut world,&mut a).unwrap(); assert_eq!(a["placed"],0);
        world.feature_result=Err("feature unavailable".into());
        assert_eq!(call(&mut world,&mut a),Err("feature unavailable".into())); assert_eq!(a["placed"],0);
        world.feature_result=Ok(true);
        for _ in 0..4 {call(&mut world,&mut a).unwrap()}
        assert_eq!(world.features,5); assert_eq!(a["placed"],3); assert_eq!(b["placed"],0);
        assert_eq!(decoded.initial_state()["placed"],0);
        assert!(world.writes.is_empty());
        let mut bad=serde_json::json!({"placed":-1});
        assert!(decoded.validate_state(&bad).is_err());
        assert_eq!(call(&mut world,&mut bad),Err("invalid saved piece counter".into()));
        assert_eq!(world.features,5,"invalid saved state must fail before feature callbacks");
    }
}
