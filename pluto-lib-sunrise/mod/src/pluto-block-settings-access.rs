// Static mod-block properties before per-state gameplay projection. No numeric
// IDs, world pointers, whole gameplay JSON copies or runtime state map.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
#[derive(Clone,Debug)]
pub struct Declaration {pub block:String,pub piston:u8}
pub type Provider=fn()->Result<Vec<Declaration>,String>;
static PROVIDER:OnceLock<Provider>=OnceLock::new();
pub fn set_provider(provider:Provider)->Result<(),String> {
    PROVIDER.set(provider).map_err(|_|"block settings provider already installed".into())
}
pub fn declarations()->Result<Settings,String> {
    Settings::new(PROVIDER.get().map_or(Ok(Vec::new()),|provider|provider())?)
}
#[derive(Default)]
pub struct Settings {pistons:BTreeMap<String,&'static str>}
impl Settings {
    pub fn new(declarations:Vec<Declaration>)->Result<Self,String> {
        let mut pistons=BTreeMap::new();
        for declaration in declarations {
            let Some((ns,path))=declaration.block.split_once(':') else {return Err("block settings require namespaced IDs".into())};
            if ns.is_empty() || path.is_empty()
                || !ns.bytes().all(|b|b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
                || !path.bytes().all(|b|b.is_ascii_lowercase() || b.is_ascii_digit() || b"_/.-".contains(&b)) {
                return Err(format!("invalid block settings ID {}",declaration.block));
            }
            let push=match declaration.piston {
                0=>"PUSH_PULL",1=>"POPPED",2=>"IMMOVEABLE",3=>"IGNORE_ENTITY",4=>"PUSH",
                _=>return Err("invalid piston behavior".into()),
            };
            if pistons.insert(declaration.block,push).is_some() {return Err("duplicate block settings".into());}
        }
        Ok(Self {pistons})
    }
    pub fn is_empty(&self)->bool {self.pistons.is_empty()}
    pub fn identity(&self)->Vec<u8> {
        let mut bytes=b"pluto-block-settings-v1".to_vec();
        for (id,push) in &self.pistons {
            bytes.extend_from_slice(&(id.len() as u64).to_le_bytes());bytes.extend_from_slice(id.as_bytes());
            bytes.extend_from_slice(&(push.len() as u64).to_le_bytes());bytes.extend_from_slice(push.as_bytes());
        }
        bytes
    }
    // This seam runs for declared mod blocks only. Reject vanilla/unregistered
    // names before a root is published instead of silently ignoring requests.
    pub fn validate_blocks<'a>(&self,blocks:impl IntoIterator<Item=&'a str>)->Result<(),String> {
        let blocks:BTreeSet<_>=blocks.into_iter().collect();
        for id in self.pistons.keys() {
            if !blocks.contains(id.as_str()) {return Err(format!("{id}: block settings need a declared mod block"));}
        }
        Ok(())
    }
    pub fn apply(&self,block:&str,set:impl FnOnce(&'static str))->bool {
        if let Some(push)=self.pistons.get(block) {set(push);true} else {false}
    }
}
#[cfg(test)]
mod tests {
    use sunrise_client_lib::*;
    fn declaration(block:&str,piston:u8)->Declaration {Declaration {block:block.into(),piston}}
    #[test]
    fn actual_settings_keep_reactions_owner_names_and_cache_identity_without_touching_fallback() {
        let make=||vec![declaration("example:floor",1),declaration("example:wall",0),
            declaration("example:blocked",2),declaration("example:ignored",3),declaration("example:push_only",4)];
        let settings=Settings::new(make()).unwrap();
        let mut reversed=make();reversed.reverse();
        assert_eq!(settings.identity(),Settings::new(reversed).unwrap().identity());
        let mut changed=make();changed[1].piston=1;
        assert_ne!(settings.identity(),Settings::new(changed).unwrap().identity());
        settings.validate_blocks(make().iter().map(|d|d.block.as_str())).unwrap();
        assert!(settings.validate_blocks(["example:floor"]).is_err());
        for (id,expected) in [("floor","POPPED"),("wall","PUSH_PULL"),("blocked","IMMOVEABLE"),
            ("ignored","IGNORE_ENTITY"),("push_only","PUSH")] {
            let mut row=("old template",17);
            assert!(settings.apply(&format!("example:{id}"),|push|row.0=push));
            assert_eq!(row,(expected,17));
        }
        assert!(!settings.apply("minecraft:piglin_wall_head",|_|panic!("undeclared template changed")));
        assert!(!Settings::default().apply("example:wall",|_|panic!("empty settings changed a row")));
        assert!(Settings::new(vec![declaration("example:wall",0),declaration("example:wall",1)]).is_err());
        assert!(Settings::new(vec![declaration("example:wall",5)]).is_err());
        assert!(Settings::new(vec![declaration("example:bad:id",0)]).is_err());
    }
}
