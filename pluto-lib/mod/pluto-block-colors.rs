// PlutoLib's render-thread bridge. The caller supplies its actual asset registry,
// so numeric state ids are never decoded against a stale global registry.
use std::collections::BTreeMap;
use std::sync::OnceLock;
use horizon_world::registry::{BlockRegistry, BlockStateId};

type Query = fn(&str, &BTreeMap<String, String>, Option<[i32; 3]>, i32) -> Option<u32>;
#[derive(Clone, Copy)]
struct Provider {
    has: fn(&str) -> bool,
    query: Query,
}
static PROVIDER: OnceLock<Provider> = OnceLock::new();

pub fn set_provider(has: fn(&str) -> bool, query: Query) {
    let _ = PROVIDER.set(Provider { has, query });
}

pub fn color_in(registry: &BlockRegistry, id: BlockStateId, pos: Option<[i32; 3]>, tint_index: i32) -> Option<u32> {
    if tint_index < 0 { return None; }
    let provider = PROVIDER.get()?;
    let state = registry.state(id)?;
    if !(provider.has)(&state.name) { return None; }
    let properties = state.properties.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
    (provider.query)(&state.name, &properties, pos, tint_index).map(|color| color & 0xFF_FFFF)
}
