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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::block_api_v1::MemoryBlockView;
    use super::super::world_api_v1::BlockState;

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
