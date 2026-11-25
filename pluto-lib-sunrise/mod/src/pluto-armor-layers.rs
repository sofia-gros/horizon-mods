// Worn-armour layer bridge: the layers of a worn stack come from the registered renderer, else the equipment asset.
// No live inventory/world lookup; the resolver's merged defaults and patch are the stack's components.
use std::{collections::BTreeMap,sync::OnceLock};
use serde_json::Value;
use super::ItemRenderer;

/// (sprite id as the equipment loader forms it, layer colour 0xRRGGBB) in draw order.
pub type Layers=Vec<(String,Option<u32>)>;
type Query=fn(&str,i32,&BTreeMap<String,Value>,&str)->Option<Layers>;
struct Provider { has:fn(&str)->bool, query:Query }
static PROVIDER:OnceLock<Provider>=OnceLock::new();

pub fn set_provider(has:fn(&str)->bool,query:Query)->Result<(), &'static str> {
    PROVIDER.set(Provider {has,query}).map_err(|_|"armour layer provider already installed")
}

/// `kind` is the equipment layer set: `humanoid` or `humanoid_leggings`. `None` leaves the equipment asset's layers.
pub fn layers(renderer:&ItemRenderer,stack:&crate::items::ItemStack,kind:&str)->Option<Layers> {
    let provider=PROVIDER.get().filter(|provider|(provider.has)(&stack.item))?;
    let view=super::stack_view(stack);
    let components=renderer.models.components(&view);
    (provider.query)(&stack.item,stack.count as i32,&components,kind)
}
