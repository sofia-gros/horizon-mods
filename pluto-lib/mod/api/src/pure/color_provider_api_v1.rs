//! Counterpart of Fabric's `fabric-rendering-v1` colour providers (`ColorProviderRegistry.BLOCK`): the tint of a block's faces, which may
//! depend on where the block stands (slime foliage follows a colour map across the world).
//!
//! The game-facing adapter resolves the real state, properties, position and tint index before asking a registered provider.
//! World-mesher wiring is owned by PlutoLib's pinned source Mixins; unverified paths remain recorded in the support matrix.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

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

// Meshing workers may query concurrently; lookup never scans every registration.
static BLOCKS: RwLock<BTreeMap<String, Provider>> = RwLock::new(BTreeMap::new());

fn provider_for(block: &str) -> Option<Provider> {
    BLOCKS.read().ok()?.get(block).cloned()
}

/// Cheap host preflight: do not decode/copy state properties for an unregistered block.
pub(crate) fn has_block_provider(block: &str) -> bool {
    BLOCKS.read().ok().is_some_and(|all| all.contains_key(block))
}

/// Translate host-owned state data only after finding a provider. Call mod code without holding the registry lock.
pub(crate) fn block_color_from_host(
    block: &str,
    properties: &std::collections::BTreeMap<String, String>,
    pos: Option<BlockPos>,
    tint_index: i32,
) -> Option<u32> {
    let provider = provider_for(block)?;
    let state = BlockState { block: Identifier::parse(block).ok()?, properties: properties.clone() };
    provider(&BlockColorContext { state, pos, tint_index })
}

/// The registry of colour providers (Fabric: `ColorProviderRegistry`).
pub struct ColorProviderRegistry;

impl ColorProviderRegistry {
    /// Gives `blocks` a provider (Fabric: `ColorProviderRegistry.BLOCK.register`). It answers the colour as `0xRRGGBB`, or `None` to leave the
    /// face to the game's own tint. A block registered again takes the newer provider.
    pub fn register_block(blocks: &[Identifier], provider: impl Fn(&BlockColorContext) -> Option<u32> + Send + Sync + 'static) {
        let provider: Provider = Arc::new(provider);
        let replaced = {
            let Ok(mut all) = BLOCKS.write() else { return };
            blocks.iter().filter_map(|block| all.insert(block.to_string(), provider.clone())).collect::<Vec<_>>()
        };
        // Dropping a captured user value may itself register another provider.
        drop(replaced);
    }

    /// The colour the provider of `context.state`'s block answers, if the block has one and it answers.
    pub fn block_color(context: &BlockColorContext) -> Option<u32> {
        let name = context.state.block.to_string();
        let provider = provider_for(&name)?;
        provider(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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

        let host_block = Identifier::parse("example:host_leaves").unwrap();
        let expected_state = BlockState::parse("example:host_leaves[facing=west,persistent=true]").unwrap();
        let properties = expected_state.properties.clone();
        let expected = BlockColorContext { state: expected_state, pos: Some([-9, 71, 23]), tint_index: 2 };
        let seen = Arc::new(Mutex::new(Vec::new()));
        let observed = seen.clone();
        let replace = host_block.clone();
        assert!(!has_block_provider("example:host_leaves"));
        ColorProviderRegistry::register_block(std::slice::from_ref(&host_block), move |context| {
            observed.lock().unwrap().push(context.clone());
            context.pos?;
            // This would deadlock if either adapter dispatch or block_color held BLOCKS.
            ColorProviderRegistry::register_block(std::slice::from_ref(&replace), |_| Some(0x010203));
            Some(0xAA55CC)
        });
        assert!(has_block_provider("example:host_leaves"));
        assert!(!has_block_provider("example:unregistered_host_block"));
        assert_eq!(block_color_from_host("example:unregistered_host_block", &properties, expected.pos, 2), None);
        assert!(seen.lock().unwrap().is_empty(), "unknown blocks never reach a provider");
        assert_eq!(block_color_from_host("example:host_leaves", &properties, None, 2), None, "a provider can decline");
        assert_eq!(block_color_from_host("example:host_leaves", &properties, expected.pos, 2), Some(0xAA55CC));
        let recorded = seen.lock().unwrap().clone();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0], BlockColorContext { pos: None, ..expected.clone() });
        assert_eq!(recorded[1], expected, "host properties, negative/world coordinates and tint index are kept");
        assert_eq!(block_color_from_host("example:host_leaves", &properties, expected.pos, 2), Some(0x010203), "reentrant registration replaced the callback");

        struct ReenterOnDrop;
        impl Drop for ReenterOnDrop {
            fn drop(&mut self) {
                let id = Identifier::parse("example:drop_reentry").unwrap();
                ColorProviderRegistry::register_block(&[id], |_| Some(0xABCDEF));
            }
        }
        let drop_block = Identifier::parse("example:dropped_provider").unwrap();
        let captured = ReenterOnDrop;
        ColorProviderRegistry::register_block(std::slice::from_ref(&drop_block), move |_| {
            let _keep_capture = &captured;
            Some(0)
        });
        let (finished, completion) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            ColorProviderRegistry::register_block(&[drop_block], |_| None);
            finished.send(()).unwrap();
        });
        completion.recv_timeout(std::time::Duration::from_secs(5)).expect("provider destructors must run outside the registry write lock");
        worker.join().unwrap();
        assert_eq!(ColorProviderRegistry::block_color(&at("example:drop_reentry", None)), Some(0xABCDEF));
    }
}
