// Owned by this Game's private CombatTracker: current-wave head claims plus
// spent live creepers carried into that wave. Dead-source lookups additionally
// require membership in native charged_exploders. No global/spawn-ID registry.
// Reset at the native wave-clear seam, before the next tick's entity events.
use std::{collections::{BTreeMap,BTreeSet},sync::{Arc,Mutex}};
#[derive(Default,Debug)]
pub struct HeadDeaths {claimed:Mutex<BTreeSet<u32>>}
impl HeadDeaths {
    pub fn available(&self,id:u32)->Result<bool,String> {
        Ok(!self.claimed.lock().map_err(|_|"explosion head state poisoned")?.contains(&id))
    }
    pub fn claim(&self,id:u32)->Result<bool,String> {
        Ok(self.claimed.lock().map_err(|_|"explosion head state poisoned")?.insert(id))
    }
    pub fn begin_wave(&self,spent_live:impl IntoIterator<Item=u32>)->Result<(),String> {
        let next=spent_live.into_iter().collect();
        *self.claimed.lock().map_err(|_|"explosion head state poisoned")?=next;Ok(())
    }
    pub fn clear(&self)->Result<(),String> {self.begin_wave(std::iter::empty())}
}
// The Sunrise host carries no woven field for this state, so the module
// owns the store: one HeadDeaths per live CombatTracker, keyed by the
// tracker's address, which is the `key` every caller passes
// (std::ptr::from_ref(&game.combat) as usize). CombatTracker is the
// game.combat field of the long-lived Game value and is never
// value-swapped between worlds, and every method on HeadDeaths takes
// &self, so consumers share one Arc<HeadDeaths> per tracker. Should a
// tracker address ever be reused by a later Game, begin_wave replaces
// the claimed set wholesale at each wave boundary before any drop
// consults it, so a stale entry cannot leak claims into the later
// tracker.
static TRACKERS:Mutex<BTreeMap<usize,Arc<HeadDeaths>>>=Mutex::new(BTreeMap::new());
/// The drop state owned by the tracker at `key`, created empty on
/// first use.
pub fn state(key:usize)->Arc<HeadDeaths> {
    let mut store=TRACKERS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    Arc::clone(store.entry(key).or_insert_with(||Arc::new(HeadDeaths::default())))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn claims_stay_in_their_own_combat_owner_and_reset_only_at_native_wave_boundary() {
        let first=HeadDeaths::default();let other=HeadDeaths::default();
        assert!(first.available(7).unwrap());assert!(first.claim(7).unwrap());
        assert!(!first.claim(7).unwrap());assert!(!first.available(7).unwrap());
        assert!(other.available(7).unwrap(),"another Game's identical numeric ID is independent");
        assert!(first.available(8).unwrap());assert!(first.claim(8).unwrap());
        first.begin_wave([7]).unwrap();
        assert!(!first.available(7).unwrap(),"a spent live creeper can explode in a later wave without regaining a head");
        assert!(first.available(8).unwrap(),"the previous dead explosion is no longer active");
        first.clear().unwrap();assert!(first.claim(7).unwrap(),"a later native wave can reuse an ID after the explicit clear");
    }
    #[test]
    fn store_gives_each_tracker_its_own_state() {
        let a=state(0x7EA11);let b=state(0x7EA22);
        assert!(a.claim(7).unwrap());assert!(b.claim(7).unwrap(),"another tracker's identical numeric ID is independent");
        assert!(std::sync::Arc::ptr_eq(&a,&state(0x7EA11)),"the same tracker key resolves to the same state");
    }
}
