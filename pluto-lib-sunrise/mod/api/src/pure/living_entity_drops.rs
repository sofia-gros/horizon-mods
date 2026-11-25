//! Borrowed death-drop callbacks. Native collection/counter forwarding remains
//! source-only until native attachment is verified; no synthetic death or item publication.
use std::{collections::BTreeMap,sync::{Arc,Mutex}};
use super::{base::Identifier,entity_events_v1::EntityInfo,player_api::StackData};

/// Owned death metadata; no live entity or Game pointers escape the callback.
#[derive(Clone,Debug,PartialEq)]
pub struct LivingDropInfo {
    /// The dying entity's identity and dimension.
    pub entity:EntityInfo,
    /// Its actual world position.
    pub position:[f64;3],
    /// The killing damage type, or None when no damage source was captured.
    pub damage_type:Option<String>,
    /// The damage owner's type, distinct from a direct projectile's type.
    pub killer_type:Option<String>,
}
/// A callback-local drop collection. Native capabilities fail explicitly when
/// unavailable; a caller must not fabricate eligibility or successful writes.
pub trait LivingEntityDrops {
    /// The retained death metadata.
    fn info(&self)->&LivingDropInfo;
    /// The native killer's current head-drop eligibility, without mutation.
    fn can_drop_head(&self)->Result<bool,String>;
    /// Atomically consume that eligibility once, before adding a head.
    fn claim_head_drop(&mut self)->Result<bool,String>;
    /// Append a full reduced stack without rewriting untouched native drops.
    fn append_stack(&mut self,stack:StackData)->Result<(),String>;
}
type Callback=Arc<dyn Fn(&mut dyn LivingEntityDrops)->Result<(),String>+Send+Sync>;
#[derive(Default)]
struct Registry {callbacks:BTreeMap<String,Callback>}
impl Registry {
    fn register(&mut self,id:Identifier,callback:Callback)->Result<(),String> {
        let id=id.to_string();
        if self.callbacks.contains_key(&id) {return Err(format!("drop callback {id} already registered"));}
        self.callbacks.insert(id,callback);Ok(())
    }
    fn snapshot(&self)->Vec<Callback> {self.callbacks.values().cloned().collect()}
}
static CALLBACKS:Mutex<Registry>=Mutex::new(Registry {callbacks:BTreeMap::new()});
/// Ordered living-death drop callbacks, outside the registry lock. Errors stop
/// later callbacks and propagate; earlier claims/appends are not rolled back.
pub struct LivingEntityDropsCallback;
impl LivingEntityDropsCallback {
    /// Register an ID-ordered callback; duplicate IDs reject without replacement.
    pub fn register(id:Identifier,callback:impl Fn(&mut dyn LivingEntityDrops)->Result<(),String>+Send+Sync+'static)->Result<(),String> {
        CALLBACKS.lock().map_err(|_|"drop callback registry poisoned")?.register(id,Arc::new(callback))
    }
}
pub(crate) fn has_callbacks()->Result<bool,String> {
    Ok(!CALLBACKS.lock().map_err(|_|"drop callback registry poisoned")?.callbacks.is_empty())
}
pub(crate) fn dispatch(context:&mut dyn LivingEntityDrops)->Result<(),String> {
    let callbacks=CALLBACKS.lock().map_err(|_|"drop callback registry poisoned")?.snapshot();
    run(callbacks,context)
}
fn run(callbacks:Vec<Callback>,context:&mut dyn LivingEntityDrops)->Result<(),String> {
    for callback in callbacks {callback(context)?;}
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_dispatch_preserves_order_claim_before_append_errors_and_old_callback_snapshots() {
        struct Context {info:LivingDropInfo,eligible:bool,trace:Vec<&'static str>,stacks:Vec<StackData>}
        impl LivingEntityDrops for Context {
            fn info(&self)->&LivingDropInfo {&self.info}
            fn can_drop_head(&self)->Result<bool,String> {Ok(self.eligible)}
            fn claim_head_drop(&mut self)->Result<bool,String> {self.trace.push("claim");Ok(std::mem::replace(&mut self.eligible,false))}
            fn append_stack(&mut self,stack:StackData)->Result<(),String> {self.trace.push("append");self.stacks.push(stack);Ok(())}
        }
        let mut registry=Registry::default();
        registry.register(Identifier::parse("example:z_stop").unwrap(),Arc::new(|_|Err("drop callback refused".into()))).unwrap();
        registry.register(Identifier::parse("example:a_head").unwrap(),Arc::new(|host| {
            if host.can_drop_head()? && host.claim_head_drop()? {host.append_stack(StackData {item:"example:head".into(),count:1,..Default::default()})?;}
            Ok(())
        })).unwrap();
        let original=registry.snapshot();
        assert!(registry.register(Identifier::parse("example:a_head").unwrap(),Arc::new(|_|panic!("replacement"))).is_err());
        registry.register(Identifier::parse("example:zz_later").unwrap(),Arc::new(|_|panic!("after error"))).unwrap();
        let mut host=Context {info:LivingDropInfo {entity:EntityInfo {id:0,kind:"example:victim".into(),dimension:"minecraft:overworld".into()},
            position:[-1.0,70.0,2.0],damage_type:Some("minecraft:explosion".into()),killer_type:Some("minecraft:creeper".into())},eligible:true,trace:vec![],stacks:vec![]};
        assert_eq!(run(original,&mut host).unwrap_err(),"drop callback refused");
        assert_eq!(host.trace,vec!["claim","append"]);assert_eq!(host.stacks[0].item,"example:head");
        host.trace.clear();assert_eq!(run(registry.snapshot(),&mut host).unwrap_err(),"drop callback refused");
        assert!(host.trace.is_empty());assert_eq!(host.stacks.len(),1,"consumed eligibility cannot append twice");
    }
}
