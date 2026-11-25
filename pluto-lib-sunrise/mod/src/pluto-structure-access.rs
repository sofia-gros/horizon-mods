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

#[cfg(not(test))]
use super::pluto_block_access::StateInfo;
#[cfg(test)]
use crate::block_access_tests::StateInfo;

/// Borrowed post-processing transport. Native implementations retain the real
/// world, chunk write policy and random stream; unsupported operations are errors.
pub trait MarkerWorld {
    fn state(&mut self,pos:[i32;3])->Result<StateInfo,String>;
    fn set_state(&mut self,pos:[i32;3],state:&StateInfo,flags:i32)->Result<bool,String>;
    fn face_sturdy(&mut self,pos:[i32;3],face:u8)->Result<bool,String>;
    fn can_survive_plant(&mut self,state:&StateInfo,pos:[i32;3])->Result<bool,String>;
    fn random_int(&mut self,bound:i32)->Result<i32,String>;
    fn random_bool(&mut self)->Result<bool,String>;
    fn place_feature(&mut self,name:&str,pos:[i32;3])->Result<bool,String>;
}

/// Owns an immutable API callback snapshot, never a borrowed host context.
pub trait PieceCallbacks:Send+Sync {
    /// Per-type saved-state validation must be connected before registered
    /// pieces can be published. The incomplete facade fails closed, not after
    /// a restored template has already written blocks.
    fn validate_state(&self,_state:&Value)->Result<(),String> {Err("registered piece state validator not connected".into())}
    fn marker(&self,world:&mut dyn MarkerWorld,name:&str,pos:[i32;3],state:&mut Value)->Result<(),String>;
}
/// Bound recursive JSON before cloning/serializing values supplied by codecs.
/// Account for queued children before extending the work list, so an oversized
/// array cannot allocate an unbounded traversal stack before rejection.
pub fn validate_value(value:&Value)->Result<(),String> {
    let mut pending=vec![(value,0usize)];
    let (mut nodes,mut bytes)=(0usize,0usize);
    while let Some((value,depth))=pending.pop() {
        nodes+=1;
        if depth>128 || nodes>65_536 {return Err("structure descriptor depth/node budget exceeded".into())}
        let children=match value {Value::Array(v)=>v.len(),Value::Object(v)=>v.len(),_=>0};
        if children>65_536-nodes-pending.len() {return Err("structure descriptor depth/node budget exceeded".into())}
        match value {
            Value::String(text)=>bytes=bytes.saturating_add(text.len()),
            Value::Array(values)=>pending.extend(values.iter().map(|v|(v,depth+1))),
            Value::Object(values)=>for (key,value) in values {bytes=bytes.saturating_add(key.len());pending.push((value,depth+1));},
            _=>{},
        }
        if bytes>1_048_576 {return Err("structure descriptor string budget exceeded".into())}
    }
    Ok(())
}

pub struct RegisteredTemplate {
    pub template:String,
    pub origin:[i32;3],
    pub rotation:u8,
    pub mirror:u8,
    pub initial_state:Value,
    pub callbacks:std::sync::Arc<dyn PieceCallbacks>,
}
pub type PieceDecoder=fn(&str,&Value)->Result<Option<RegisteredTemplate>,String>;
static PIECE_DECODER:RwLock<Option<PieceDecoder>>=RwLock::new(None);

pub fn set_piece_decoder(decoder:PieceDecoder) {
    if let Ok(mut slot)=PIECE_DECODER.write() {*slot=Some(decoder)}
}
pub fn decode_piece(kind:&str,configuration:&Value)->Result<Option<RegisteredTemplate>,String> {
    let decoder=*PIECE_DECODER.read().map_err(|_|"structure piece dispatcher poisoned".to_owned())?;
    match decoder {Some(decoder)=>decoder(kind,configuration),None=>Ok(None)}
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
    fn piece_decode_reentry_and_json_budgets_reject_before_recursive_work() {
        fn replacement(_: &str,_:&Value)->Result<Option<RegisteredTemplate>,String>{Ok(None)}
        fn first(kind:&str,value:&Value)->Result<Option<RegisteredTemplate>,String> {
            set_piece_decoder(replacement);
            assert!(decode_piece(kind,value)?.is_none());
            Err("piece codec rejected".into())
        }
        assert!(decode_piece("example:unknown",&Value::Null).unwrap().is_none());
        set_piece_decoder(first);
        assert_eq!(decode_piece("example:piece",&Value::Null).err().as_deref(),Some("piece codec rejected"));
        assert!(decode_piece("example:piece",&Value::Null).unwrap().is_none());
        assert!(validate_value(&Value::Array(vec![Value::Null;65_535])).is_ok());
        assert!(validate_value(&Value::Array(vec![Value::Null;65_536])).is_err());
        let at_limit=(0..128).fold(Value::Null,|v,_|Value::Array(vec![v]));
        assert!(validate_value(&at_limit).is_ok());
        assert!(validate_value(&Value::Array(vec![at_limit])).is_err());
        assert!(validate_value(&Value::String("x".repeat(1_048_576))).is_ok());
        assert!(validate_value(&Value::String("x".repeat(1_048_577))).is_err());
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
