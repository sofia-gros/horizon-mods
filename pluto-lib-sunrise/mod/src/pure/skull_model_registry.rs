//! Custom item/worn skull models, separate from block-entity renderer dispatch.
use std::{collections::BTreeMap,sync::Mutex};
use super::base::Identifier;
/// The native skull model's inherited animation family.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(u8)]
pub enum SkullModelAnimation {None=0,Piglin=1,Dragon=2}
#[derive(Clone,Debug)]
pub(crate) struct Definition {pub item:Identifier,pub layer:String,pub texture:Identifier,pub animation:SkullModelAnimation}
#[derive(Default)]
struct Registry {frozen:bool,entries:BTreeMap<String,Definition>}
static REGISTRY:Mutex<Registry>=Mutex::new(Registry {frozen:false,entries:BTreeMap::new()});
fn register(registry:&Mutex<Registry>,definition:Definition)->Result<(),String> {
    if definition.item.to_string().starts_with("minecraft:") {return Err("custom skull items require a mod namespace".into());}
    let (id,layer)=definition.layer.split_once('#').ok_or("skull layer requires namespace:path#layer")?;
    Identifier::parse(id).map_err(|e|e.to_string())?;
    if layer.is_empty() || layer.contains(['#',':']) {return Err("invalid skull model layer".into());}
    let texture=definition.texture.to_string();
    let path=texture.split_once(':').unwrap().1;
    if path.starts_with("textures/") || path.ends_with(".png") {return Err("skull texture needs a sprite ID, not a PNG path".into());}
    let mut registry=registry.lock().map_err(|_|"skull model registry poisoned")?;
    if registry.frozen {return Err("skull models have already been collected".into());}
    let key=definition.item.to_string();
    if registry.entries.contains_key(&key) {return Err(format!("skull model {key} already registered"));}
    registry.entries.insert(key,definition);Ok(())
}
/// Registers a custom skull's item/worn model without mod-owned injections.
pub struct SkullModelRegistry;
impl SkullModelRegistry {
    /// The item ID also identifies its custom minecraft:head special-model kind.
    /// `layer` is an EntityModelLayerRegistry ID; `texture` is an atlas sprite ID
    /// (namespace:entity/path), not a textures/ path or a PNG filename.
    pub fn register(item:Identifier,layer:&str,texture:Identifier,animation:SkullModelAnimation)->Result<(),String> {
        register(&REGISTRY,Definition {item,layer:layer.into(),texture,animation})
    }
}
fn collect(registry:&Mutex<Registry>)->Result<Vec<Definition>,String> {
    let mut registry=registry.lock().map_err(|_|"skull model registry poisoned")?;
    registry.frozen=true;Ok(registry.entries.values().cloned().collect())
}
pub(crate) fn definitions()->Result<Vec<Definition>,String> {collect(&REGISTRY)}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn skull_registration_keeps_original_on_duplicate_and_freezes_owned_definitions() {
        let registry=Mutex::new(Registry::default());
        let definition=Definition {item:Identifier::parse("example:head").unwrap(),layer:"example:head#main".into(),
            texture:Identifier::parse("example:entity/head").unwrap(),animation:SkullModelAnimation::Piglin};
        register(&registry,definition.clone()).unwrap();
        let mut duplicate=definition.clone();duplicate.layer="example:wrong#main".into();
        assert!(register(&registry,duplicate).is_err());
        let mut first=collect(&registry).unwrap();
        assert_eq!(first[0].layer,"example:head#main");assert_eq!(first[0].animation,SkullModelAnimation::Piglin);
        first[0].layer="changed:copy#main".into();
        assert_eq!(collect(&registry).unwrap()[0].layer,"example:head#main");
        assert!(register(&registry,Definition {item:Identifier::parse("example:late").unwrap(),..definition}).is_err());
    }
}
