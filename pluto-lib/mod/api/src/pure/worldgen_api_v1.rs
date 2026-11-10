//! Feature types and configuration codecs (Minecraft/Fabric: registered Feature and ConfiguredFeature).
//! The host adapter decides which placement paths invoke these generators. A direct call is not
//! evidence of natural-generation integration; see the support matrix.
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use serde_json::Value;
use super::base::Identifier;
use super::block_api_v1::BlockEditor;
use super::block_entity_api_v1::BlockPos;

/// The supplied live placement world and origin. Random draws go through this world's
/// BlockEditor, never a newly seeded generator. This borrowed context cannot be retained.
pub struct FeatureContext<'a> {
    pub world: &'a mut dyn BlockEditor,
    pub origin: BlockPos,
}

type Generator = dyn Fn(&mut FeatureContext<'_>) -> bool + Send + Sync;

/// An immutable, decoded configuration with its generation algorithm. A codec captures
/// validated typed settings here rather than reparsing JSON while writing the world.
#[derive(Clone)]
pub struct ConfiguredFeature {
    generate: Arc<Generator>,
}

impl ConfiguredFeature {
    /// Bind validated configuration to an algorithm. False reports placement failure;
    /// earlier successful writes are not rolled back automatically.
    pub fn new(generate: impl Fn(&mut FeatureContext<'_>) -> bool + Send + Sync + 'static) -> Self {
        Self { generate: Arc::new(generate) }
    }

    /// Execute using the caller's world and random stream. This direct call does not
    /// register the feature or connect any host generation path.
    pub fn generate(&self, context: &mut FeatureContext<'_>) -> bool {
        (self.generate)(context)
    }
}

type Codec = Arc<dyn Fn(&Value) -> Result<ConfiguredFeature, String> + Send + Sync>;
static TYPES: RwLock<BTreeMap<String, Codec>> = RwLock::new(BTreeMap::new());

/// Feature-type registration: each namespaced type supplies a configuration decoder.
/// A later registration replaces that type; callbacks run outside the registry lock.
pub struct FeatureTypeRegistry;

impl FeatureTypeRegistry {
    /// Register a codec for a feature type. It validates configuration before any world
    /// access and returns a ConfiguredFeature or a diagnostic. False means the registry
    /// could not be written. Destroy replaced user captures outside the write lock.
    pub fn register(id: Identifier, codec: impl Fn(&Value) -> Result<ConfiguredFeature, String> + Send + Sync + 'static) -> bool {
        let codec: Codec = Arc::new(codec);
        let previous = {
            let Ok(mut all) = TYPES.write() else { return false };
            all.insert(id.to_string(), codec)
        };
        drop(previous);
        true
    }

    /// Decode and generate directly. Ok(None) means an unregistered type (the host may
    /// delegate to native placement). Ok(Some(false)) is a handled placement failure,
    /// never permission to run a second algorithm. Err is a codec/registry failure.
    /// No world access is performed by this function before decoding succeeds.
    pub fn generate(id: &Identifier, configuration: &Value, context: &mut FeatureContext<'_>) -> Result<Option<bool>, String> {
        let codec = TYPES.read().map_err(|_| "feature registry poisoned".to_owned())?.get(&id.to_string()).cloned();
        let Some(codec) = codec else { return Ok(None) };
        let feature = codec(configuration)?;
        Ok(Some(feature.generate(context)))
    }
}

/// Registry-bound inputs compiled by the host before a root-placement run.
/// Lists preserve caller order and provider identity; entries are not sampled or
/// flattened into fixed states by the API. Native codecs validate their schemas.
#[derive(Clone, Default)]
pub struct WorldgenBindings {
    pub block_sets: Vec<Value>,
    pub state_providers: Vec<Value>,
}

/// A root-placement editor with precompiled registry bindings. Indices refer to
/// the RootPlacement's ordered binding lists, never to native global state IDs.
pub trait RootPlacementWorld: BlockEditor {
    /// Test the block at pos against a bound holder set, or report an unavailable
    /// binding. False is a real non-match, never a substitute for a lookup error.
    fn matches_block_set(&mut self, _index: usize, _pos: BlockPos) -> Result<bool, String> {
        Err("block-set bindings unavailable".into())
    }
    /// Sample exactly the selected provider using this placement's world/RNG.
    /// No produced state is an error, not implicit reuse of the current block.
    /// An error must not cause selection of a different provider.
    fn sample_state_provider(&mut self, _index: usize, _pos: BlockPos) -> Result<super::world_api_v1::BlockState, String> {
        Err("state-provider bindings unavailable".into())
    }
}

// Memory editors cannot bind native holders/providers. They explicitly report
// unsupported binding queries rather than inventing block/tag/provider results.
impl RootPlacementWorld for super::block_api_v1::MemoryBlockView {}

/// One native root position. Native simulation and trunk offset remain owned by
/// the tree's base root placer; this callback overrides its per-position write.
pub struct RootPlacementContext<'a> {
    pub world: &'a mut dyn RootPlacementWorld,
    pub pos: BlockPos,
}

type PlaceRoot = dyn Fn(&mut RootPlacementContext<'_>) -> Result<Option<super::world_api_v1::BlockState>, String> + Send + Sync;

/// An immutable per-root placement override with registry-binding descriptors.
/// Some(state) replaces this root (native waterlogging/tracking still apply);
/// None requests the original parent root placement; Err never falls through.
#[derive(Clone)]
pub struct RootPlacement {
    bindings: WorldgenBindings,
    place: Arc<PlaceRoot>,
}

impl RootPlacement {
    /// Capture binding descriptors and validated algorithm settings. The native
    /// host must compile every descriptor before starting the tree's writes.
    pub fn new(bindings: WorldgenBindings, place: impl Fn(&mut RootPlacementContext<'_>) -> Result<Option<super::world_api_v1::BlockState>, String> + Send + Sync + 'static) -> Self {
        Self { bindings, place: Arc::new(place) }
    }
    /// Ordered descriptors for host binding; no world access or random draw.
    pub fn bindings(&self) -> &WorldgenBindings { &self.bindings }
    /// Execute once for one root. Preserve errors and explicit parent fallback.
    pub fn place(&self, context: &mut RootPlacementContext<'_>) -> Result<Option<super::world_api_v1::BlockState>, String> { (self.place)(context) }
}

type RootCodec = Arc<dyn Fn(&Value) -> Result<RootPlacement, String> + Send + Sync>;
static ROOTS: RwLock<BTreeMap<String, RootCodec>> = RwLock::new(BTreeMap::new());

/// Codec registration for per-position root overrides, not replacement of a
/// native root simulator. Explicit tree placement supplies the decoded override.
pub struct RootPlacementTypeRegistry;

impl RootPlacementTypeRegistry {
    /// Register/replace a codec; user callbacks/destructors run outside locks.
    pub fn register(id: Identifier, codec: impl Fn(&Value) -> Result<RootPlacement, String> + Send + Sync + 'static) -> bool {
        let codec: RootCodec = Arc::new(codec);
        let previous = {
            let Ok(mut all) = ROOTS.write() else { return false };
            all.insert(id.to_string(), codec)
        };
        drop(previous);
        true
    }
    /// Decode without world writes. Unknown is None; registered errors remain
    /// errors. The host separately compiles registry bindings before generation.
    pub fn decode(id: &Identifier, configuration: &Value) -> Result<Option<RootPlacement>, String> {
        let codec = ROOTS.read().map_err(|_| "root placement registry poisoned".to_owned())?.get(&id.to_string()).cloned();
        let Some(codec) = codec else { return Ok(None) };
        codec(configuration).map(Some)
    }
}

/// Borrowed inputs to a tree decorator. The tree generator supplies its ordered
/// position lists and its own editor/RNG. This context does not discover leaves,
/// track writes for a native tree, or run the native final shape/distance passes.
pub struct TreeDecoratorContext<'a> {
    pub world: &'a mut dyn BlockEditor,
    pub logs: &'a [BlockPos],
    pub leaves: &'a [BlockPos],
    pub roots: &'a [BlockPos],
}

type Decorator = dyn Fn(&mut TreeDecoratorContext<'_>) -> Result<(), String> + Send + Sync;

/// An immutable decoded tree decorator. Errors preserve earlier writes and must
/// be handled as failure, not permission to run an alternative decorator.
#[derive(Clone)]
pub struct TreeDecorator {
    decorate: Arc<Decorator>,
}

impl TreeDecorator {
    /// Capture validated settings. The borrowed placement context cannot escape.
    pub fn new(decorate: impl Fn(&mut TreeDecoratorContext<'_>) -> Result<(), String> + Send + Sync + 'static) -> Self {
        Self { decorate: Arc::new(decorate) }
    }

    /// Run against caller-supplied tree positions, without reordering them.
    pub fn decorate(&self, context: &mut TreeDecoratorContext<'_>) -> Result<(), String> {
        (self.decorate)(context)
    }
}

/// One step in an explicit tree decoration plan. Native indices refer to the
/// tree configuration's decorator array; every native entry must occur once.
/// Custom algorithms may be interleaved without altering the native RNG stream.
#[derive(Clone)]
pub enum TreeDecoration {
    Native(usize),
    Custom(TreeDecorator),
}

type DecoratorCodec = Arc<dyn Fn(&Value) -> Result<TreeDecorator, String> + Send + Sync>;
static DECORATORS: RwLock<BTreeMap<String, DecoratorCodec>> = RwLock::new(BTreeMap::new());

/// Namespaced tree-decorator codecs. Tree generators explicitly decode before
/// placement, then invoke their decorators inside the same placement context.
/// Ordinary native tree JSON and natural generation do not decode these types.
/// Explicit BlockEditor::place_tree accepts already decoded decorators.
pub struct TreeDecoratorTypeRegistry;

impl TreeDecoratorTypeRegistry {
    /// Register or replace a codec. Callbacks and replaced capture destructors run
    /// outside the registry lock. False means the registry could not be written.
    pub fn register(id: Identifier, codec: impl Fn(&Value) -> Result<TreeDecorator, String> + Send + Sync + 'static) -> bool {
        let codec: DecoratorCodec = Arc::new(codec);
        let previous = {
            let Ok(mut all) = DECORATORS.write() else { return false };
            all.insert(id.to_string(), codec)
        };
        drop(previous);
        true
    }

    /// Decode without touching a world. Unknown types return Ok(None); a known
    /// type's codec error stays an error. Decode all decorators before tree writes.
    pub fn decode(id: &Identifier, configuration: &Value) -> Result<Option<TreeDecorator>, String> {
        let codec = DECORATORS.read().map_err(|_| "tree decorator registry poisoned".to_owned())?.get(&id.to_string()).cloned();
        let Some(codec) = codec else { return Ok(None) };
        codec(configuration).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::block_api_v1::MemoryBlockView;
    use super::super::world_api_v1::BlockState;

    #[test]
    fn root_overrides_keep_bindings_parent_fallback_errors_and_registry_reentry() {
        let id = Identifier::parse("example:bound_root_override").unwrap();
        assert!(RootPlacementTypeRegistry::decode(&id, &Value::Null).unwrap().is_none());
        let replacement = id.clone();
        assert!(RootPlacementTypeRegistry::register(id.clone(), move |value| {
            if value.as_bool() != Some(true) { return Err("root disabled".into()); }
            RootPlacementTypeRegistry::register(replacement.clone(), |_| Err("replaced root codec".into()));
            Ok(RootPlacement::new(WorldgenBindings {
                block_sets: vec![serde_json::json!("#example:original_holder")],
                state_providers: vec![serde_json::json!({"type":"minecraft:weighted_state_provider","entries":[]})],
            }, |context| {
                match context.pos[0] {
                    0 => Ok(None),
                    1 => Ok(Some(BlockState::parse("minecraft:mangrove_roots").unwrap())),
                    _ => {
                        context.world.matches_block_set(0, context.pos)?;
                        context.world.sample_state_provider(0, context.pos).map(Some)
                    }
                }
            }))
        }));
        assert_eq!(RootPlacementTypeRegistry::decode(&id, &Value::Bool(false)).err().as_deref(), Some("root disabled"));
        let roots = RootPlacementTypeRegistry::decode(&id, &Value::Bool(true)).unwrap().unwrap();
        assert_eq!(roots.bindings().block_sets, [serde_json::json!("#example:original_holder")]);
        assert_eq!(roots.bindings().state_providers[0]["type"], "minecraft:weighted_state_provider");
        assert_eq!(RootPlacementTypeRegistry::decode(&id, &Value::Bool(true)).err().as_deref(), Some("replaced root codec"));
        let mut world = MemoryBlockView::default();
        assert_eq!(roots.place(&mut RootPlacementContext { world: &mut world, pos: [0, 70, 0] }), Ok(None));
        assert_eq!(roots.clone().place(&mut RootPlacementContext { world: &mut world, pos: [1, 70, 0] }).unwrap().unwrap().block.to_string(), "minecraft:mangrove_roots");
        assert_eq!(roots.place(&mut RootPlacementContext { world: &mut world, pos: [-1, 70, 0] }), Err("block-set bindings unavailable".into()));
        assert!(world.writes.is_empty(), "selection does not itself perform native root writes");
    }

    #[test]
    fn decorators_decode_before_placement_keep_order_and_propagate_partial_failure() {
        let id = Identifier::parse("example:ordered_decorator").unwrap();
        assert!(TreeDecoratorTypeRegistry::decode(&id, &Value::Null).unwrap().is_none());
        let replacement = id.clone();
        assert!(TreeDecoratorTypeRegistry::register(id.clone(), move |value| {
            if value.as_bool() != Some(true) { return Err("disabled decorator".into()); }
            TreeDecoratorTypeRegistry::register(replacement.clone(), |_| Err("replaced".into()));
            Ok(TreeDecorator::new(|context| {
                TreeDecoratorTypeRegistry::register(Identifier::parse("example:inside_decorator").unwrap(), |_| Err("unused".into()));
                assert_eq!(context.logs, [[-2, 70, 0]]);
                assert_eq!(context.roots, [[-2, 69, 0]]);
                assert_eq!(context.world.random_int(5), 2);
                let stone = BlockState::parse("minecraft:stone").unwrap();
                for &pos in context.leaves {
                    if pos[0] == 4 { return Err("stop after first write".into()); }
                    if !context.world.set_block_state_with_flags(pos, &stone, 19) { return Err("write refused".into()); }
                }
                Ok(())
            }))
        }));
        assert_eq!(TreeDecoratorTypeRegistry::decode(&id, &Value::Bool(false)).err().as_deref(), Some("disabled decorator"));
        let decorator = TreeDecoratorTypeRegistry::decode(&id, &Value::Bool(true)).unwrap().unwrap();
        assert_eq!(TreeDecoratorTypeRegistry::decode(&id, &Value::Bool(true)).err().as_deref(), Some("replaced"));
        let mut world = MemoryBlockView::default();
        world.rolls = vec![7];
        assert_eq!(decorator.clone().decorate(&mut TreeDecoratorContext {
            world: &mut world, logs: &[[-2, 70, 0]], leaves: &[[9, 72, 0], [4, 71, 0]], roots: &[[-2, 69, 0]],
        }), Err("stop after first write".into()));
        assert_eq!(world.writes.len(), 1);
        assert_eq!(world.writes[0].0, [9, 72, 0]);
        assert_eq!(world.writes[0].2, 19);
        assert!(!world.blocks.contains_key(&[4, 71, 0]));
    }

    #[test]
    fn codecs_validate_before_writes_and_dispatch_keeps_results_rng_and_reentry() {
        let id = Identifier::parse("example:feature_column").unwrap();
        let replacement = id.clone();
        let mut world = MemoryBlockView::default();
        world.rolls = vec![7];
        let mut context = FeatureContext { world: &mut world, origin: [-3, 70, 8] };
        assert_eq!(FeatureTypeRegistry::generate(&id, &Value::Null, &mut context), Ok(None));
        assert!(FeatureTypeRegistry::register(id.clone(), move |configuration| {
            let height = configuration.get("height").and_then(Value::as_u64)
                .filter(|n| (1..=4).contains(n)).ok_or_else(|| "height must be 1..=4".to_owned())?;
            // Registration from inside a codec would deadlock if its read guard lived on.
            FeatureTypeRegistry::register(Identifier::parse("example:codec_reentry").unwrap(), |_| Err("unused".into()));
            let replacement = replacement.clone();
            Ok(ConfiguredFeature::new(move |c| {
                assert_eq!(c.world.random_int(5), 2);
                for y in 0..height as i32 {
                    let pos = [c.origin[0], c.origin[1] + y, c.origin[2]];
                    if !c.world.set_block_state(pos, &BlockState::parse("minecraft:glass").unwrap()) { return false; }
                }
                // Generation may replace even its own factory without retaining a lock.
                FeatureTypeRegistry::register(replacement.clone(), |_| Ok(ConfiguredFeature::new(|_| false)));
                true
            }))
        }));
        assert_eq!(FeatureTypeRegistry::generate(&id, &serde_json::json!({"height": 0}), &mut context), Err("height must be 1..=4".into()));
        assert!(context.world.is_air(context.origin));
        assert_eq!(FeatureTypeRegistry::generate(&id, &serde_json::json!({"height": 3}), &mut context), Ok(Some(true)));
        assert_eq!(context.world.block_state([-3, 72, 8]).unwrap().block.to_string(), "minecraft:glass");
        assert!(context.world.is_air([-3, 73, 8]));
        assert_eq!(FeatureTypeRegistry::generate(&id, &Value::Null, &mut context), Ok(Some(false)));
    }
}
