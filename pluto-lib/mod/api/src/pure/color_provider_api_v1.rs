//! Counterpart of Fabric's `fabric-rendering-v1` colour providers (`ColorProviderRegistry.BLOCK`): the tint of a block's faces, which may
//! depend on where the block stands (slime foliage follows a colour map across the world).
//!
//! Stub: the pinned Horizon Mixins cannot apply the injection that would call the providers while chunks are meshed (see the support
//! matrix), so providers are accepted and kept but not asked yet.

use std::sync::{Arc, Mutex};

use super::base::Identifier;
use super::block_entity_api_v1::BlockPos;
use super::world_api_v1::BlockState;

/// What a block colour provider is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockColorContext {
    /// The block's state.
    pub state: BlockState,
    /// Where it stands; None when it is drawn without a place (an item in a hand).
    pub pos: Option<BlockPos>,
    /// The tint index of the face being drawn (the model's `tintindex`).
    pub tint_index: i32,
}

type Provider = Arc<dyn Fn(&BlockColorContext) -> Option<u32> + Send + Sync>;

static BLOCKS: Mutex<Vec<(Vec<String>, Provider)>> = Mutex::new(Vec::new());

/// The registry of colour providers (Fabric: `ColorProviderRegistry`).
pub struct ColorProviderRegistry;

impl ColorProviderRegistry {
    /// Gives `blocks` a provider (Fabric: `ColorProviderRegistry.BLOCK.register`). It answers the colour as `0xRRGGBB`, or `None` to leave the
    /// face to the game's own tint. A block registered again takes the newer provider.
    pub fn register_block(blocks: &[Identifier], provider: impl Fn(&BlockColorContext) -> Option<u32> + Send + Sync + 'static) {
        if let Ok(mut all) = BLOCKS.lock() {
            let names: Vec<String> = blocks.iter().map(Identifier::to_string).collect();
            for (existing, _) in all.iter_mut() {
                existing.retain(|n| !names.contains(n));
            }
            all.retain(|(existing, _)| !existing.is_empty());
            all.push((names, Arc::new(provider)));
        }
    }

    /// The colour the provider of `context.state`'s block answers, if the block has one and it answers.
    pub fn block_color(context: &BlockColorContext) -> Option<u32> {
        let name = context.state.block.to_string();
        let provider = BLOCKS.lock().ok()?.iter().find(|(names, _)| names.contains(&name)).map(|(_, p)| p.clone())?;
        provider(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_provider_answers_for_its_blocks_only_and_a_later_registration_replaces_it() {
        let leaves = Identifier::parse("example:leaves").unwrap();
        ColorProviderRegistry::register_block(std::slice::from_ref(&leaves), |c| c.pos.map(|p| 0x00FF00 + p[0] as u32));
        let at = |block: &str, pos: Option<BlockPos>| BlockColorContext { state: BlockState::parse(block).unwrap(), pos, tint_index: 0 };
        assert_eq!(ColorProviderRegistry::block_color(&at("example:leaves", Some([5, 64, 0]))), Some(0x00FF05));
        assert_eq!(ColorProviderRegistry::block_color(&at("example:leaves", None)), None, "the provider declined");
        assert_eq!(ColorProviderRegistry::block_color(&at("example:stone", Some([0; 3]))), None);
        ColorProviderRegistry::register_block(&[leaves], |_| Some(0x123456));
        assert_eq!(ColorProviderRegistry::block_color(&at("example:leaves", None)), Some(0x123456));
    }
}
