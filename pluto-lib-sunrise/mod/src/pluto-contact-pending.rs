//Bounded transient effects with separate player/world clocks. Drains move
//requests out before the caller invokes sound delivery or consumer callbacks.
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock { Player(i32), World(i64) }

struct State<T> {
    world_tick: Option<i64>,
    requests: BTreeMap<i32, Vec<(Clock, T)>>,
}

pub struct Pending<T>(Mutex<State<T>>);
impl<T> Default for Pending<T> {
    fn default() -> Self {
        Self(Mutex::new(State { world_tick: None, requests: BTreeMap::new() }))
    }
}
// Attachments and worlds can be cloned for snapshots. A transient effect is
// owned by the original simulation phase, never duplicated into its clone.
impl<T> Clone for Pending<T> {
    fn clone(&self) -> Self { Self::default() }
}
impl<T> Pending<T> {
    pub fn begin_world(&self, tick: i64) -> bool {
        let Ok(mut state) = self.0.lock() else { return false };
        // A nested operation borrows the outer phase; it must neither erase
        // pending requests nor claim ownership of that phase's flush.
        if state.world_tick == Some(tick) { return false; }
        // A skipped flush may not replay into a later world tick.
        state.requests.retain(|_, rows| {
            rows.retain(|(clock, _)| matches!(clock, Clock::Player(_)));
            !rows.is_empty()
        });
        state.world_tick = Some(tick);
        true
    }
    pub fn world_active(&self, tick: i64) -> bool {
        self.0.lock().is_ok_and(|state| state.world_tick == Some(tick))
    }
    pub fn push(&self, entity: i32, clock: Clock, value: T) -> bool {
        let Ok(mut state) = self.0.lock() else { return false };
        if let Clock::World(tick) = clock {
            if state.world_tick != Some(tick) { return false; }
        }
        if !state.requests.contains_key(&entity) && state.requests.len() >= 256 { return false; }
        let rows = state.requests.entry(entity).or_default();
        if rows.len() >= 256 { return false; }
        rows.push((clock, value));
        true
    }
    pub fn drain_player(&self, entity: i32, tick: i32) -> Vec<T> {
        let Ok(mut state) = self.0.lock() else { return Vec::new() };
        let Some(rows) = state.requests.remove(&entity) else { return Vec::new() };
        let (mut ready, mut keep) = (Vec::new(), Vec::new());
        for (clock, value) in rows {
            match clock {
                Clock::Player(stamp) if stamp == tick => ready.push(value),
                Clock::World(_) => keep.push((clock, value)),
                Clock::Player(_) => {}, // stale player requests are discarded
            }
        }
        if !keep.is_empty() { state.requests.insert(entity, keep); }
        ready
    }
    pub fn drain_world(&self, tick: i64) -> Vec<(i32, T)> {
        let Ok(mut state) = self.0.lock() else { return Vec::new() };
        let active = state.world_tick.take() == Some(tick);
        let rows = std::mem::take(&mut state.requests);
        let mut ready = Vec::new();
        for (entity, rows) in rows {
            let mut keep = Vec::new();
            for (clock, value) in rows {
                match clock {
                    Clock::World(stamp) if active && stamp == tick => ready.push((entity, value)),
                    Clock::Player(_) => keep.push((clock, value)),
                    Clock::World(_) => {},
                }
            }
            if !keep.is_empty() { state.requests.insert(entity, keep); }
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use sunrise_world::*;
    #[test]
    fn clocks_worlds_clones_and_skipped_phases_do_not_replay_effects() {
        let queue = Pending::default();
        assert!(!queue.push(1, Clock::World(10), "outside phase"));
        assert!(queue.begin_world(10));
        assert!(queue.push(1, Clock::Player(30), "player"));
        assert!(queue.push(1, Clock::World(10), "mob"));
        assert!(!queue.begin_world(10)); // nested scope keeps the outer requests
        assert!(!queue.push(1, Clock::World(11), "wrong tick"));
        assert!(queue.clone().drain_world(10).is_empty());
        assert!(queue.clone().drain_player(1, 30).is_empty());
        assert_eq!(queue.drain_world(10), vec![(1, "mob")]);
        assert!(!queue.world_active(10));
        assert_eq!(queue.drain_player(1, 30), vec!["player"]);
        assert!(queue.begin_world(11));
        assert!(queue.push(2, Clock::World(11), "skipped"));
        assert!(queue.begin_world(12));
        assert!(queue.drain_world(12).is_empty());
        assert!(queue.push(2, Clock::Player(1), "stale"));
        assert!(queue.drain_player(2, 2).is_empty());
        assert!(queue.drain_player(2, 1).is_empty());
        assert!(Pending::<()>::default().drain_world(10).is_empty());
    }
    #[test]
    fn per_entity_and_total_entity_caps_reject_without_evicting_accepted_requests() {
        let queue = Pending::default();
        for n in 0..256 { assert!(queue.push(1, Clock::Player(4), n)); }
        assert!(!queue.push(1, Clock::Player(4), 999));
        assert_eq!(queue.drain_player(1, 4), (0..256).collect::<Vec<_>>());
        for entity in 0..256 { assert!(queue.push(entity, Clock::Player(4), entity)); }
        assert!(!queue.push(256, Clock::Player(4), 256));
        assert_eq!(queue.drain_player(0, 4), vec![0]);
        assert!(queue.push(256, Clock::Player(4), 256));
    }
}
