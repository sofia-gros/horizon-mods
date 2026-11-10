// Decode against the same registry-bound catalog used by native live features.
use super::*;

pub fn decode_tree(features: &LiveFeatures, configuration: &Value) -> Result<Box<crate::worldgen::tree::TreeFeature>, crate::worldgen::DecodeError> {
    match features.catalog.feature_with_blocks(&configuration.to_string(), &features.bindings)? {
        FeatureDefinition::Tree(tree) => Ok(tree),
        _ => Err(crate::worldgen::DecodeError::Field("expected a tree feature")),
    }
}

pub struct BoundRoots {
    pub block_sets: Vec<Arc<BTreeSet<crate::registry::BlockStateId>>>,
    pub state_providers: Vec<crate::worldgen::state_provider::BlockStateProvider>,
}

/// Compile every original holder/provider against the active native registry
/// before generation. A bad unselected provider is still a decode failure.
pub fn bind_roots(features: &LiveFeatures, bindings: &crate::level::pluto_block_access::RootBindings) -> Result<BoundRoots, crate::worldgen::DecodeError> {
    if bindings.block_sets.len() > 256 || bindings.state_providers.len() > 256 {
        return Err(crate::worldgen::DecodeError::Field("root binding count"));
    }
    let mut remaining = 65536;
    let mut decoder = crate::worldgen::tree::TreeDecoder { bindings: &features.bindings, depth: 0, remaining: &mut remaining };
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
