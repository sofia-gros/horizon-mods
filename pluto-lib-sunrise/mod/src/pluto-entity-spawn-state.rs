// Transient Mob-owned lifecycle state, never keyed by a temporary entity ID.
// Pending spawn payloads are not serialized or copied into render/network Mob
// clones. A head-claim bit shares identity across value snapshots of the same
// Mob, so an older AI/render copy cannot restore eligibility. New/load resets it.
// Native storage is the Mob-owned Navigation (private fields, concrete Default
// literal), not the externally constructible Mob. Spawn tails do not replace it;
// tick-only rider navigation loans occur later. Native attachment validation remains pending.
use std::{any::Any,cell::Cell,fmt,sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}}};
#[derive(Clone)]
pub struct SpawnToken(Arc<()>);
// Reuse the existing clone-shared logical entity identity, not its spent value.
// Holding this token prevents allocation reuse during a borrowed callback.
#[derive(Clone)]
pub struct EntityToken(Arc<AtomicBool>);
#[derive(Default)]
struct Inner { value:Option<Box<dyn Any+Send+Sync>>,token:Option<SpawnToken>,claimed:bool }
#[derive(Default)]
pub struct SpawnState { pending:Mutex<Inner>,head_spent:Arc<AtomicBool> }
impl Clone for SpawnState {
    fn clone(&self)->Self { Self {pending:Mutex::new(Inner::default()),head_spent:self.head_spent.clone()} }
}
impl fmt::Debug for SpawnState {
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result {
        f.debug_struct("SpawnState").field("prepared",&self.pending.lock().map(|v|v.claimed).ok()).finish()
    }
}
impl SpawnState {
    pub fn entity_token(&self)->EntityToken {EntityToken(self.head_spent.clone())}
    pub fn owns_entity(&self,token:&EntityToken)->bool {Arc::ptr_eq(&self.head_spent,&token.0)}
    // Creeper's droppedSkulls is independent of powered and of spawn completion.
    // Native counter/drop callers still require source attachment review.
    pub fn head_drop_available(&self)->bool { !self.head_spent.load(Ordering::Acquire) }
    pub fn claim_head_drop(&self)->bool {
        self.head_spent.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_ok()
    }
    // A second prepare cannot overwrite/reroll a pending plan. Errors return
    // ownership of the rejected value instead of dropping it under the lock.
    pub fn prepare<T:Any+Send+Sync>(&self,value:T)->Result<(),(String,T)> {
        let mut pending=match self.pending.lock() {
            Ok(pending)=>pending,
            Err(_)=>return Err(("spawn state poisoned".into(),value)),
        };
        if pending.claimed { return Err(("spawn already prepared".into(),value)); }
        pending.claimed=true;
        pending.token=Some(SpawnToken(Arc::new(())));
        pending.value=Some(Box::new(value));
        Ok(())
    }
    // Consume once, before executing completion callbacks. A wrong consumer
    // type must not steal or destroy another registered dispatcher's state.
    pub fn take<T:Any+Send+Sync>(&self)->Result<Option<T>,String> {
        self.take_with_token::<T>().map(|value|value.map(|(value,_)|value))
    }
    pub fn take_with_token<T:Any+Send+Sync>(&self)->Result<Option<(T,SpawnToken)>,String> {
        let mut pending=self.pending.lock().map_err(|_|"spawn state poisoned")?;
        let Some(value)=pending.value.as_ref() else { return Ok(None) };
        if !value.is::<T>() { return Err("spawn state type mismatch".into()); }
        let token=pending.token.clone().ok_or("prepared spawn has no owner token")?;
        let value=pending.value.take().unwrap();
        drop(pending);
        value.downcast::<T>().map(|value|Some((*value,token))).map_err(|_|"spawn state type mismatch".into())
    }
    pub fn is_consumed(&self)->Result<bool,String> {
        let pending=self.pending.lock().map_err(|_|"spawn state poisoned")?;
        Ok(pending.claimed && pending.value.is_none())
    }
    pub fn owns(&self,token:&SpawnToken)->Result<bool,String> {
        let pending=self.pending.lock().map_err(|_|"spawn state poisoned")?;
        Ok(pending.token.as_ref().is_some_and(|own|Arc::ptr_eq(&own.0,&token.0)))
    }
    pub fn abort(&self)->Result<(),String> {
        let value=self.pending.lock().map_err(|_|"spawn state poisoned")?.value.take();
        // Payload destructors may re-enter state/resource services.
        drop(value);
        Ok(())
    }
}

// Only the charged-creeper table consults this scoped policy. Other damage
// properties (especially powered) must remain unchanged for normal loot tables.
thread_local! {static HEAD_ELIGIBLE:Cell<Option<bool>>=const {Cell::new(None)};}
pub fn head_drop_eligibility()->Option<bool> {HEAD_ELIGIBLE.with(Cell::get)}
pub fn with_head_drop_eligibility<R>(allowed:bool,run:impl FnOnce()->R)->R {
    struct Restore(Option<bool>);
    impl Drop for Restore {fn drop(&mut self){HEAD_ELIGIBLE.with(|slot|slot.set(self.0));}}
    let old=HEAD_ELIGIBLE.with(|slot|slot.replace(Some(allowed)));
    let _restore=Restore(old);
    run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc,Weak,atomic::{AtomicUsize,AtomicBool,Ordering}};
    struct Lease { drops:Arc<AtomicUsize>,owner:Weak<SpawnState>,unlocked:Arc<AtomicBool> }
    impl Drop for Lease {
        fn drop(&mut self) {
            self.drops.fetch_add(1,Ordering::SeqCst);
            if let Some(owner)=self.owner.upgrade() {
                self.unlocked.store(owner.pending.try_lock().is_ok(),Ordering::SeqCst);
            }
        }
    }
    #[test]
    fn head_claim_is_owner_local_once_only_and_scoped_loot_policy_restores_after_nested_panic() {
        let owner=SpawnState::default();let other=SpawnState::default();
        let before_claim=owner.clone();let identity=owner.entity_token();
        assert!(owner.owns_entity(&identity));assert!(before_claim.owns_entity(&identity));
        assert!(!other.owns_entity(&identity));assert!(!SpawnState::default().owns_entity(&identity));
        assert!(owner.head_drop_available());assert!(owner.claim_head_drop());assert!(!owner.claim_head_drop());
        assert!(!before_claim.head_drop_available(),"an older value snapshot shares the same logical head claim");
        assert!(!before_claim.claim_head_drop());
        assert!(owner.owns_entity(&identity),"spending the head claim must not change logical entity identity");
        assert!(!owner.head_drop_available());assert!(other.head_drop_available());
        assert!(!owner.clone().head_drop_available(),"Mob value copies retain the spent flag, not a pending spawn plan");
        assert!(SpawnState::default().head_drop_available(),"new/restored entities get the unsaved constructor default");
        assert_eq!(head_drop_eligibility(),None);
        with_head_drop_eligibility(true,|| {
            assert_eq!(head_drop_eligibility(),Some(true));
            let result=std::panic::catch_unwind(||with_head_drop_eligibility(false,|| {
                assert_eq!(head_drop_eligibility(),Some(false));panic!("nested loot failure");
            }));
            assert!(result.is_err());assert_eq!(head_drop_eligibility(),Some(true));
        });
        assert_eq!(head_drop_eligibility(),None);
    }
    #[test]
    fn state_survives_owned_mob_moves_but_not_clones_and_consumes_exactly_once() {
        let drops=Arc::new(AtomicUsize::new(0));
        let unlocked=Arc::new(AtomicBool::new(false));
        let state=Arc::new(SpawnState::default());
        let make=||Lease { drops:drops.clone(),owner:Arc::downgrade(&state),unlocked:unlocked.clone() };
        assert!(!state.is_consumed().unwrap());
        assert!(state.prepare(make()).is_ok());
        assert!(!state.is_consumed().unwrap());
        let copied=state.as_ref().clone();
        assert!(copied.take::<Lease>().unwrap().is_none(),"render/transport clones must not duplicate a pending spawn plan");
        assert!(state.take::<u64>().is_err(),"wrong consumer must not steal the lease");
        let rejected=state.prepare(17u32).unwrap_err();
        assert_eq!(rejected.1,17);
        let moved=state.clone(); // moving an owner/Arc, not cloning the Mob field
        let (lease,token)=moved.take_with_token::<Lease>().unwrap().unwrap();
        assert!(state.is_consumed().unwrap());
        assert!(!copied.is_consumed().unwrap());
        assert!(state.owns(&token).unwrap());
        assert!(!copied.owns(&token).unwrap(),"reused IDs and Mob clones cannot claim the original completion");
        assert!(!SpawnState::default().owns(&token).unwrap());
        assert!(state.take::<Lease>().unwrap().is_none());
        assert!(state.prepare(18u32).is_err(),"completion cannot reroll the same spawn");
        assert_eq!(drops.load(Ordering::SeqCst),0);
        drop(lease);
        assert_eq!(drops.load(Ordering::SeqCst),1);
        assert!(unlocked.load(Ordering::SeqCst));
        let cancelling=Arc::new(SpawnState::default());
        assert!(cancelling.prepare(Lease { drops:drops.clone(),owner:Arc::downgrade(&cancelling),unlocked:unlocked.clone() }).is_ok());
        cancelling.abort().unwrap(); cancelling.abort().unwrap();
        assert!(cancelling.prepare(19u32).is_err());
        assert_eq!(drops.load(Ordering::SeqCst),2);
        assert!(unlocked.load(Ordering::SeqCst),"abort must release the lock before dropping captured resources");
        let aborted=SpawnState::default();
        assert!(aborted.prepare(make()).is_ok());
        drop(aborted);
        assert_eq!(drops.load(Ordering::SeqCst),3,"aborted unpublished mobs release their captured generation");
    }
}
