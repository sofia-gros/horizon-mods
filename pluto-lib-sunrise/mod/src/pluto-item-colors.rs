// Common model-resolution bridge: GUI/hand/CIT all reach resolve_with_rule.
// No live inventory/world lookup; use the resolver's merged defaults and patch.
use std::{collections::{BTreeMap,BTreeSet},sync::OnceLock};
use serde_json::Value;
use super::{ItemRenderer,LayerGeometry,LayerRender,StackView};

type Query=fn(&str,i32,&BTreeMap<String,Value>,i32)->Option<u32>;
struct Provider { has:fn(&str)->bool, query:Query }
static PROVIDER:OnceLock<Provider>=OnceLock::new();
// Malformed models must not cause an unbounded tint-array allocation.
const MAX_TINT_INDEX:i32=4095;

pub fn set_provider(has:fn(&str)->bool,query:Query)->Result<(), &'static str> {
    PROVIDER.set(Provider {has,query}).map_err(|_|"item colour provider already installed")
}

pub fn tint(renderer:&ItemRenderer,stack:&StackView,mut layers:Vec<LayerRender>)->Vec<LayerRender> {
    let Some(provider)=PROVIDER.get().filter(|provider|(provider.has)(&stack.item)) else {return layers};
    let components=renderer.models.components(stack);
    for layer in &mut layers {
        let LayerGeometry::Quads(baked)=&layer.geometry else {continue};
        let indices:BTreeSet<_>=baked.quads.iter().map(|quad|quad.tint_index)
            .filter(|index|(0..=MAX_TINT_INDEX).contains(index)).collect();
        for index in indices {
            let Some(rgb)=(provider.query)(&stack.item,stack.count,&components,index) else {continue};
            let index=index as usize;
            if index>=layer.tints.len() {layer.tints.resize(index+1,-1);}
            // ItemRenderState expects ARGB, unlike the public RGB provider API.
            layer.tints[index]=(0xFF00_0000|(rgb&0xFF_FFFF)) as i32;
        }
    }
    layers
}
