// Decode against the same registry-bound catalog used by native live features.
use sunrise_world::worldgen::*;

pub fn decode_tree(features: &LiveFeatures, configuration: &Value) -> Result<Box<sunrise_world::worldgen::tree::TreeFeature>, sunrise_world::worldgen::DecodeError> {
    match features.catalog.feature_with_blocks(&configuration.to_string(), &features.bindings)? {
        FeatureDefinition::Tree(tree) => Ok(tree),
        _ => Err(sunrise_world::worldgen::DecodeError::Field("expected a tree feature")),
    }
}

pub struct BoundRoots {
    pub block_sets: Vec<Arc<BTreeSet<sunrise_world::registry::BlockStateId>>>,
    pub state_providers: Vec<sunrise_world::worldgen::state_provider::BlockStateProvider>,
}

/// Compile every original holder/provider against the active native registry
/// before generation. A bad unselected provider is still a decode failure.
pub fn bind_roots(features: &LiveFeatures, bindings: &crate::pluto_block_access::RootBindings) -> Result<BoundRoots, sunrise_world::worldgen::DecodeError> {
    if bindings.block_sets.len() > 256 || bindings.state_providers.len() > 256 {
        return Err(sunrise_world::worldgen::DecodeError::Field("root binding count"));
    }
    let mut remaining = 65536;
    let mut decoder = sunrise_world::worldgen::tree::TreeDecoder { bindings: &features.bindings, depth: 0, remaining: &mut remaining };
    let mut block_sets = Vec::new();
    for value in &bindings.block_sets {
        let raw: Box<serde_json::value::RawValue> = serde_json::from_str(&value.to_string())?;
        block_sets.push(decoder.block_set(&raw)?);
    }
    let mut state_providers = Vec::new();
    for value in &bindings.state_providers {
        let raw: Box<serde_json::value::RawValue> = serde_json::from_str(&value.to_string())?;
        state_providers.push(decoder.provider(&raw)?);
    }
    Ok(BoundRoots { block_sets, state_providers })
}

pub fn feature_blocks(features: &LiveFeatures) -> FeatureBlocks { features.blocks }
