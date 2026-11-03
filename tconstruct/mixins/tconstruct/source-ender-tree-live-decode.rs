// Decode against the same bound registries and state providers as live features.
use super::*;
pub fn decode(features: &LiveFeatures, tall: bool) -> Result<crate::worldgen::tree::tconstruct_ender_tree_native::EnderTree, crate::worldgen::DecodeError> {
    let json=if tall {crate::worldgen::tree::tconstruct_ender_tree_config::TALL} else {crate::worldgen::tree::tconstruct_ender_tree_config::SMALL};
    crate::worldgen::tree::tconstruct_ender_tree_native::EnderTree::decode(json,&features.bindings)
}
