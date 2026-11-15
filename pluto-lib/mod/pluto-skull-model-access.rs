// Shared symbolic skull definitions in horizon-assets. No renderer/Game/world
// objects live here; a registered custom kind is never a vanilla-player fallback.
use std::{collections::BTreeMap,sync::OnceLock};
#[derive(Clone,Debug)]
pub struct Definition {pub item:String,pub layer:String,pub texture:String,pub animation:u8}
pub type Provider=fn()->Result<Vec<Definition>,String>;
static PROVIDER:OnceLock<Provider>=OnceLock::new();
static SNAPSHOT:OnceLock<Result<Definitions,String>>=OnceLock::new();
static EMPTY:Definitions=Definitions {entries:BTreeMap::new()};
pub fn set_provider(provider:Provider)->Result<(),String> {PROVIDER.set(provider).map_err(|_|"skull definition provider already installed".into())}
#[derive(Default)]
pub struct Definitions {entries:BTreeMap<String,Definition>}
fn identifier(id:&str)->bool {
    let Some((ns,path))=id.split_once(':') else {return false};
    !ns.is_empty() && !path.is_empty()
        && ns.bytes().all(|b|b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
        && path.bytes().all(|b|b.is_ascii_lowercase() || b.is_ascii_digit() || b"_/.-".contains(&b))
}
impl Definitions {
    pub fn new(definitions:Vec<Definition>)->Result<Self,String> {
        let mut entries=BTreeMap::new();
        for definition in definitions {
            let (layer,name)=definition.layer.split_once('#').ok_or("invalid skull layer")?;
            if !identifier(&definition.item) || definition.item.starts_with("minecraft:")
                || !identifier(layer) || name.is_empty() || name.contains(['#',':'])
                || !identifier(&definition.texture) || definition.animation>2 {
                return Err("invalid custom skull definition".into());
            }
            if entries.insert(definition.item.clone(),definition).is_some() {return Err("duplicate custom skull definition".into());}
        }
        Ok(Self {entries})
    }
    pub fn get(&self,kind:&str)->Option<&Definition> {self.entries.get(kind)}
    pub fn iter(&self)->impl Iterator<Item=&Definition> {self.entries.values()}
}
pub fn definitions()->Result<&'static Definitions,String> {
    let Some(provider)=PROVIDER.get() else {return Ok(&EMPTY)};
    SNAPSHOT.get_or_init(||Definitions::new(provider()?)).as_ref().map_err(Clone::clone)
}
pub fn definition(kind:&str)->Result<Option<&'static Definition>,String> {Ok(definitions()?.get(kind))}
pub fn worn_kind(item:&str)->Result<Option<&'static str>,String> {Ok(definition(item)?.map(|definition|definition.item.as_str()))}
pub fn animation_kind(animation:u8)->&'static str {match animation {1=>"piglin",2=>"dragon",_=>"skeleton"}}
// Head special-model texture overrides use entity-relative resource locations;
// definitions themselves use full sprite IDs. Keep custom namespaces intact.
pub fn sprite(definition:&Definition,overridden:Option<&str>)->String {
    let Some(texture)=overridden else {return definition.texture.strip_prefix("minecraft:").unwrap_or(&definition.texture).to_owned()};
    let (ns,path)=texture.split_once(':').unwrap_or(("minecraft",texture));
    if ns=="minecraft" {format!("entity/{path}")} else {format!("{ns}:entity/{path}")}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_definition_transport_keeps_custom_identity_styles_texture_namespaces_and_refuses_bad_batches() {
        let make=||Definition {item:"example:head".into(),layer:"example:head#main".into(),texture:"example:entity/head".into(),animation:1};
        let definitions=Definitions::new(vec![make()]).unwrap();let head=definitions.get("example:head").unwrap();
        assert!(definitions.get("minecraft:piglin_head").is_none());assert!(definitions.get("other:head").is_none());
        assert_eq!(animation_kind(head.animation),"piglin");assert_eq!(animation_kind(0),"skeleton");assert_eq!(animation_kind(2),"dragon");
        assert_eq!(sprite(head,None),"example:entity/head");assert_eq!(sprite(head,Some("pack:replaced")),"pack:entity/replaced");
        assert_eq!(sprite(head,Some("minecraft:piglin/piglin")),"entity/piglin/piglin");
        assert!(Definitions::new(vec![make(),make()]).is_err());
        assert!(Definitions::new(vec![Definition {animation:3,..make()}]).is_err());
        assert!(Definitions::new(vec![Definition {item:"minecraft:head".into(),..make()}]).is_err());
    }
}
