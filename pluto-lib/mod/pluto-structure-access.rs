// Owned structure-plan transport below the client crate. Borrowed native
// contexts never enter globals: only a plain function pointer is installed.
use serde_json::Value;
use std::sync::RwLock;

pub trait PlanningWorld {
    fn top_of_chunk_center(&mut self, chunk:[i32;2])->Result<Option<[i32;3]>,String>;
    fn is_valid_biome(&mut self,pos:[i32;3])->Result<bool,String>;
    fn first_occupied_height(&mut self,x:i32,z:i32)->Result<i32,String>;
    fn generation_depth(&self)->i32;
    fn sea_level(&self)->i32;
    fn random_int(&mut self,bound:i32)->Result<i32,String>;
}

#[derive(Debug,PartialEq)]
pub enum PlanResult {
    Unregistered,
    Rejected,
    Start(Value),
}

pub type Dispatcher=fn(&str,&Value,&mut dyn PlanningWorld,[i32;2])->Result<PlanResult,String>;
pub type Registered=fn(&str)->Result<bool,String>;
static DISPATCHER:RwLock<Option<(Registered,Dispatcher)>>=RwLock::new(None);

pub fn set_dispatcher(registered:Registered,dispatcher:Dispatcher) {
    if let Ok(mut slot)=DISPATCHER.write() {*slot=Some((registered,dispatcher));}
}

pub fn is_registered(kind:&str)->Result<bool,String> {
    let dispatcher=*DISPATCHER.read().map_err(|_|"structure dispatcher poisoned".to_owned())?;
    match dispatcher {Some((registered,_))=>registered(kind),None=>Ok(false)}
}

pub fn plan(kind:&str,configuration:&Value,world:&mut dyn PlanningWorld,chunk:[i32;2])->Result<PlanResult,String> {
    let dispatcher=*DISPATCHER.read().map_err(|_|"structure dispatcher poisoned".to_owned())?;
    match dispatcher {
        Some((_,dispatcher))=>dispatcher(kind,configuration,world,chunk),
        None=>Ok(PlanResult::Unregistered),
    }
}

/// Only owned source data and a weak resource identity are retained here, never
/// a borrowed world. Ambiguous identities fail closed instead of replacing an
/// older world's settings. Entries whose owners died are pruned at capture.
#[derive(Debug)]
pub struct SourceSnapshot {
    pub structures:std::collections::BTreeMap<String,String>,
    pub sets:std::collections::BTreeMap<String,String>,
}
pub struct SourceCatalogs<T> {
    entries:RwLock<std::collections::BTreeMap<usize,(std::sync::Weak<T>,Option<std::sync::Arc<SourceSnapshot>>)>>,
}
impl<T> SourceCatalogs<T> {
    pub const fn new()->Self {Self {entries:RwLock::new(std::collections::BTreeMap::new())}}
    pub fn capture(&self,owner:&std::sync::Arc<T>,structures:&std::collections::BTreeMap<String,String>,sets:&std::collections::BTreeMap<String,String>)->Result<(),String> {
        let mut entries=self.entries.write().map_err(|_|"structure source catalog poisoned".to_owned())?;
        entries.retain(|_,(owner,_)|owner.strong_count()>0);
        let key=std::sync::Arc::as_ptr(owner) as usize;
        if let Some((weak,snapshot))=entries.get_mut(&key) {
            if weak.upgrade().is_some_and(|previous|std::sync::Arc::ptr_eq(&previous,owner)) {
                if snapshot.as_ref().is_some_and(|source|source.structures==*structures&&source.sets==*sets) {return Ok(())}
                *snapshot=None;
                return Err("shared structure resources have ambiguous source snapshots".into());
            }
        }
        entries.insert(key,(std::sync::Arc::downgrade(owner),Some(std::sync::Arc::new(SourceSnapshot {structures:structures.clone(),sets:sets.clone()}))));
        Ok(())
    }
    pub fn get(&self,owner:&std::sync::Arc<T>)->Result<std::sync::Arc<SourceSnapshot>,String> {
        let entries=self.entries.read().map_err(|_|"structure source catalog poisoned".to_owned())?;
        let (weak,snapshot)=entries.get(&(std::sync::Arc::as_ptr(owner) as usize)).ok_or("structure source snapshot unavailable")?;
        if !weak.upgrade().is_some_and(|previous|std::sync::Arc::ptr_eq(&previous,owner)) {return Err("stale structure source identity".into())}
        snapshot.clone().ok_or_else(||"shared structure resources have ambiguous source snapshots".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct World;
    impl PlanningWorld for World {
        fn top_of_chunk_center(&mut self,_:[i32;2])->Result<Option<[i32;3]>,String>{panic!("no context query expected")}
        fn is_valid_biome(&mut self,_:[i32;3])->Result<bool,String>{panic!("no context query expected")}
        fn first_occupied_height(&mut self,_:i32,_:i32)->Result<i32,String>{panic!("no context query expected")}
        fn generation_depth(&self)->i32{384}
        fn sea_level(&self)->i32{63}
        fn random_int(&mut self,_:i32)->Result<i32,String>{panic!("no RNG draw expected")}
    }
    #[test]
    fn owned_catalog_snapshots_isolate_owners_and_reject_shared_identity_changes() {
        use std::collections::BTreeMap;
        use std::sync::Arc;
        let catalogs=SourceCatalogs::new();
        let first=Arc::new(0u8); let second=Arc::new(0u8);
        let a=BTreeMap::from([("example:island".into(),"first settings".into())]);
        let b=BTreeMap::from([("example:island".into(),"second settings".into())]);
        let sets=BTreeMap::new();
        catalogs.capture(&first,&a,&sets).unwrap();
        assert_eq!(Arc::strong_count(&first),1,"catalog must not retain native resources");
        let old=catalogs.get(&first).unwrap();
        catalogs.capture(&second,&b,&sets).unwrap();
        assert_eq!(catalogs.get(&second).unwrap().structures,b);
        assert_eq!(catalogs.get(&first).unwrap().structures,a);
        catalogs.capture(&first,&a,&sets).unwrap();
        assert!(catalogs.capture(&first,&b,&sets).is_err());
        assert!(catalogs.get(&first).is_err());
        assert_eq!(old.structures,a,"already-owned snapshots remain immutable");
        assert!(catalogs.capture(&first,&a,&sets).is_err(),"ambiguity must not be silently cleared");
        drop(first);
        let third=Arc::new(1u8);
        catalogs.capture(&third,&a,&sets).unwrap();
        assert_eq!(catalogs.entries.read().unwrap().len(),2,"dead owner entry is pruned");
        assert_eq!(catalogs.get(&third).unwrap().structures,a);
    }

    #[test]
    fn dispatcher_replacement_and_reentry_release_locks_and_keep_rejection_distinct() {
        fn replacement(_: &str,_:&Value,_:&mut dyn PlanningWorld,_:[i32;2])->Result<PlanResult,String>{Ok(PlanResult::Rejected)}
        fn first(kind:&str,configuration:&Value,world:&mut dyn PlanningWorld,chunk:[i32;2])->Result<PlanResult,String>{
            set_dispatcher(|_|Ok(true),replacement);
            assert_eq!(plan(kind,configuration,world,chunk)?,PlanResult::Rejected);
            Ok(PlanResult::Start(serde_json::json!({"position":[-8,70,24],"pieces":[]})))
        }
        assert_eq!(plan("example:unknown",&Value::Null,&mut World,[0,0]),Ok(PlanResult::Unregistered));
        assert_eq!(is_registered("example:test"),Ok(false));
        set_dispatcher(|_|Ok(true),first);
        assert_eq!(is_registered("example:test"),Ok(true));
        assert!(matches!(plan("example:test",&Value::Null,&mut World,[-1,1]),Ok(PlanResult::Start(_))));
        assert_eq!(plan("example:test",&Value::Null,&mut World,[-1,1]),Ok(PlanResult::Rejected));
    }
}
