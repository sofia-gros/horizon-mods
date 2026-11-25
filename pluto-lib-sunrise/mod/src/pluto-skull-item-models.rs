// ItemRenderer-owned initial geometry. A whole candidate batch resolves before
// insertion; a bad registered layer never replaces a native model or leaks a
// partial custom prefix. Fresh renderers/reloads receive fresh owned trees.
use std::{collections::{HashMap,HashSet},path::Path,sync::OnceLock};
use crate::block_entity_render::ModelPart;
pub type Provider=fn()->Result<Vec<(String,ModelPart)>,String>;
static PROVIDER:OnceLock<Provider>=OnceLock::new();
pub fn set_provider(provider:Provider)->Result<(),String> {PROVIDER.set(provider).map_err(|_|"skull model provider already installed".into())}
pub fn extend(parts:&mut HashMap<String,ModelPart>,root:&Path)->Result<(),String> {
    extend_with(parts,|| {
        let bytes=std::fs::read(root.join("entity-models.json")).map_err(|e|e.to_string())?;
        serde_json::from_slice(&bytes).map(std::borrow::Cow::Owned).map_err(|e|e.to_string())
    })
}
pub fn extend_document(parts:&mut HashMap<String,ModelPart>,document:&serde_json::Value)->Result<(),String> {
    extend_with(parts,||Ok(std::borrow::Cow::Borrowed(document)))
}
fn extend_with<'a>(parts:&mut HashMap<String,ModelPart>,load:impl FnOnce()->Result<std::borrow::Cow<'a,serde_json::Value>,String>)->Result<(),String> {
    let mut load=Some(load);
    let definitions=sunrise_assets::item_model::pluto_skull_model_access::definitions()?;
    let definitions:Vec<_>=definitions.iter().collect();
    if definitions.is_empty() {return Ok(());}
    let supplied=PROVIDER.get().map_or(Ok(Vec::new()),|provider|provider())?;
    let known:HashSet<_>=definitions.iter().map(|d|d.item.as_str()).collect();
    let mut models=HashMap::new();
    for (id,part) in supplied {
        if !known.contains(id.as_str()) || parts.contains_key(&id) || models.insert(id,part).is_some() {
            return Err("duplicate, native or unregistered skull geometry".into());
        }
    }
    let mut vanilla=None;
    let mut candidate=Vec::new();
    for definition in definitions {
        let part=if let Some(part)=models.remove(&definition.item) {part} else {
            if !definition.layer.starts_with("minecraft:") {return Err(format!("{}: skull model layer unavailable",definition.layer));}
            if vanilla.is_none() {
                vanilla=Some(load.take().expect("native model document loads once")()?);
            }
            let data=vanilla.as_ref().unwrap().get(&definition.layer).ok_or_else(||format!("{}: native skull layer unavailable",definition.layer))?;
            ModelPart::parse(data)
        };
        if parts.contains_key(&definition.item) {return Err("registered skull geometry would replace native model".into());}
        candidate.push((definition.item.clone(),part));
    }
    parts.extend(candidate);Ok(())
}
