// Decode against the same registry-bound catalog used by native live features.
use super::*;

pub fn decode_tree(features: &LiveFeatures, configuration: &Value) -> Result<Box<crate::worldgen::tree::TreeFeature>, crate::worldgen::DecodeError> {
    match features.catalog.feature_with_blocks(&configuration.to_string(), &features.bindings)? {
        FeatureDefinition::Tree(tree) => Ok(tree),
        _ => Err(crate::worldgen::DecodeError::Field("expected a tree feature")),
    }
}

pub fn feature_blocks(features: &LiveFeatures) -> FeatureBlocks { features.blocks }
