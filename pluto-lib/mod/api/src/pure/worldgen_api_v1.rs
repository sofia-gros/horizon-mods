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

type DecoratorCodec = Arc<dyn Fn(&Value) -> Result<TreeDecorator, String> + Send + Sync>;
static DECORATORS: RwLock<BTreeMap<String, DecoratorCodec>> = RwLock::new(BTreeMap::new());

/// Namespaced tree-decorator codecs. Tree generators explicitly decode before
/// placement, then invoke their decorators inside the same placement context.
/// Native tree decoding/placement and natural generation are not connected yet.
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
