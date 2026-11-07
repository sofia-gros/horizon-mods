//! `Event<T>` (Fabric API: `net.fabricmc.fabric.api.event.Event` and `EventFactory`).
//!
//! An event holds callbacks, grouped into named *phases* that can be ordered; invoking the event runs the callbacks and
//! combines their results. Differences from Fabric, because this is Rust: a callback is a closure `Fn(&A) -> R`
//! (`A` is the callback's argument, a tuple for several), the combining rule is a [`Combiner`] picked when the event is
//! made instead of an invoker factory, and ordering a cycle returns an error instead of throwing.

use std::sync::{Arc, RwLock};

use super::action_result::ActionResult;
use super::identifier::Identifier;

/// A registered callback.
pub type Callback<A, R> = Arc<dyn Fn(&A) -> R + Send + Sync>;

/// How the results of an event's callbacks become one result. The iterator runs callbacks lazily, so a combiner that
/// stops early stops the remaining callbacks from running (Fabric: `UseItemCallback` stops at the first non-`PASS`).
pub trait Combiner<R>: Send + Sync {
    fn combine(&self, results: &mut dyn Iterator<Item = R>) -> R;
}

/// Run every callback; the result is `()` (Fabric: events with a `void` callback such as `ServerTickEvents`).
pub struct Unit;

impl Combiner<()> for Unit {
    fn combine(&self, results: &mut dyn Iterator<Item = ()>) {
        for () in results {}
    }
}

/// The first result that is not `ActionResult::Pass` wins, else `Pass` (Fabric: `UseItemCallback`, `UseBlockCallback`).
pub struct FirstNotPass;

impl Combiner<ActionResult> for FirstNotPass {
    fn combine(&self, results: &mut dyn Iterator<Item = ActionResult>) -> ActionResult {
        for result in results {
            if result != ActionResult::Pass {
                return result;
            }
        }
        ActionResult::Pass
    }
}

/// Every callback must return `true`; the first `false` stops the rest (Fabric: `ALLOW_*` events such as `ALLOW_DAMAGE`).
pub struct AllowAll;

impl Combiner<bool> for AllowAll {
    fn combine(&self, results: &mut dyn Iterator<Item = bool>) -> bool {
        for allowed in results {
            if !allowed {
                return false;
            }
        }
        true
    }
}

/// Two phase orderings contradict each other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhaseOrderingCycle {
    pub first: Identifier,
    pub second: Identifier,
}

impl std::fmt::Display for PhaseOrderingCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ordering phase {} before {} creates a cycle", self.first, self.second)
    }
}

impl std::error::Error for PhaseOrderingCycle {}

struct Phase<A, R> {
    id: Identifier,
    callbacks: Vec<Callback<A, R>>,
}

struct State<A, R> {
    phases: Vec<Phase<A, R>>,
    orderings: Vec<(Identifier, Identifier)>,
    /// Callbacks in invocation order; rebuilt after a registration or ordering change.
    snapshot: Option<Arc<Vec<Callback<A, R>>>>,
}

impl<A, R> State<A, R> {
    const fn new() -> Self {
        Self { phases: Vec::new(), orderings: Vec::new(), snapshot: None }
    }

    fn phase_index(&mut self, id: &Identifier) -> usize {
        if let Some(i) = self.phases.iter().position(|p| &p.id == id) {
            return i;
        }
        self.phases.push(Phase { id: id.clone(), callbacks: Vec::new() });
        self.phases.len() - 1
    }

    /// Phases in a stable topological order: earliest-created first among those with no unmet "runs before" constraint.
    fn ordered_phases(&self) -> Result<Vec<usize>, ()> {
        let n = self.phases.len();
        let index = |id: &Identifier| self.phases.iter().position(|p| &p.id == id);
        let mut waiting_on = vec![0usize; n];
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for (first, second) in &self.orderings {
            if let (Some(a), Some(b)) = (index(first), index(second)) {
                edges.push((a, b));
                waiting_on[b] += 1;
            }
        }
        let mut done = vec![false; n];
        let mut order = Vec::with_capacity(n);
        while order.len() < n {
            let next = (0..n).find(|&i| !done[i] && waiting_on[i] == 0).ok_or(())?;
            done[next] = true;
            order.push(next);
            for &(a, b) in &edges {
                if a == next {
                    waiting_on[b] -= 1;
                }
            }
        }
        Ok(order)
    }
}

/// The phase callbacks are registered in when none is named (Fabric: `Event.DEFAULT_PHASE`).
pub fn default_phase() -> Identifier {
    Identifier::new("pluto", "default").expect("valid identifier")
}

/// A set of callbacks invoked together (Fabric: `Event<T>`). Make one with the constructor for its result type
/// ([`Event::void`], [`Event::first_not_pass`], [`Event::allow_all`], or [`Event::new`] with your own [`Combiner`]); it is
/// `const`, so events can be `static`.
pub struct Event<A: 'static, R: 'static> {
    combiner: &'static dyn Combiner<R>,
    state: RwLock<State<A, R>>,
}

impl<A: 'static, R: 'static> Event<A, R> {
    pub const fn new(combiner: &'static dyn Combiner<R>) -> Self {
        Self { combiner, state: RwLock::new(State::new()) }
    }

    /// Fabric: `register(listener)`: into the default phase.
    pub fn register(&self, callback: impl Fn(&A) -> R + Send + Sync + 'static) {
        self.register_with_phase(default_phase(), callback);
    }

    /// Fabric: `register(phase, listener)`.
    pub fn register_with_phase(&self, phase: Identifier, callback: impl Fn(&A) -> R + Send + Sync + 'static) {
        let mut state = self.state.write().expect("event state poisoned");
        let i = state.phase_index(&phase);
        state.phases[i].callbacks.push(Arc::new(callback));
        state.snapshot = None;
    }

    /// Fabric: `addPhaseOrdering(first, second)`: callbacks of `first` run before those of `second`. A contradicting
    /// ordering is refused and leaves the event unchanged.
    pub fn add_phase_ordering(&self, first: Identifier, second: Identifier) -> Result<(), PhaseOrderingCycle> {
        let mut state = self.state.write().expect("event state poisoned");
        state.phase_index(&first);
        state.phase_index(&second);
        state.orderings.push((first.clone(), second.clone()));
        if state.ordered_phases().is_err() {
            state.orderings.pop();
            return Err(PhaseOrderingCycle { first, second });
        }
        state.snapshot = None;
        Ok(())
    }

    fn callbacks(&self) -> Arc<Vec<Callback<A, R>>> {
        if let Some(snapshot) = &self.state.read().expect("event state poisoned").snapshot {
            return Arc::clone(snapshot);
        }
        let mut state = self.state.write().expect("event state poisoned");
        if let Some(snapshot) = &state.snapshot {
            return Arc::clone(snapshot);
        }
        let order = state.ordered_phases().expect("orderings are checked when added");
        let flat: Vec<_> = order.into_iter().flat_map(|i| state.phases[i].callbacks.iter().cloned()).collect();
        let snapshot = Arc::new(flat);
        state.snapshot = Some(Arc::clone(&snapshot));
        snapshot
    }

    /// Fabric: `invoker().invoke(...)`. Callbacks run without any lock held, so a callback may register more callbacks
    /// (they take effect from the next invocation).
    pub fn invoke(&self, args: &A) -> R {
        let callbacks = self.callbacks();
        let mut results = callbacks.iter().map(|callback| callback(args));
        self.combiner.combine(&mut results)
    }
}

impl<A: 'static> Event<A, ()> {
    /// An event whose callbacks return nothing; all run.
    pub const fn void() -> Self {
        Self::new(&Unit)
    }
}

impl<A: 'static> Event<A, ActionResult> {
    /// An interaction event: the first callback that does not `Pass` decides.
    pub const fn first_not_pass() -> Self {
        Self::new(&FirstNotPass)
    }
}

impl<A: 'static> Event<A, bool> {
    /// An `ALLOW_*` event: false from any callback vetoes.
    pub const fn allow_all() -> Self {
        Self::new(&AllowAll)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn id(path: &str) -> Identifier {
        Identifier::new("test", path).unwrap()
    }

    fn recorder() -> (Arc<Mutex<Vec<&'static str>>>, impl Fn(&'static str) -> Box<dyn Fn(&()) + Send + Sync>) {
        let log = Arc::new(Mutex::new(Vec::new()));
        let make = {
            let log = Arc::clone(&log);
            move |name: &'static str| -> Box<dyn Fn(&()) + Send + Sync> {
                let log = Arc::clone(&log);
                Box::new(move |_| log.lock().unwrap().push(name))
            }
        };
        (log, make)
    }

    #[test]
    fn callbacks_run_in_registration_order() {
        let event: Event<(), ()> = Event::void();
        let (log, cb) = recorder();
        for name in ["a", "b", "c"] {
            event.register(cb(name));
        }
        event.invoke(&());
        assert_eq!(*log.lock().unwrap(), ["a", "b", "c"]);
    }

    #[test]
    fn phase_ordering_overrides_registration_order() {
        let event: Event<(), ()> = Event::void();
        let (log, cb) = recorder();
        event.register_with_phase(id("late"), cb("late"));
        event.register_with_phase(id("early"), cb("early"));
        event.add_phase_ordering(id("early"), id("late")).unwrap();
        event.invoke(&());
        assert_eq!(*log.lock().unwrap(), ["early", "late"]);
    }

    #[test]
    fn contradicting_ordering_is_refused_and_event_still_works() {
        let event: Event<(), ()> = Event::void();
        let (log, cb) = recorder();
        event.register_with_phase(id("a"), cb("a"));
        event.register_with_phase(id("b"), cb("b"));
        event.add_phase_ordering(id("a"), id("b")).unwrap();
        let err = event.add_phase_ordering(id("b"), id("a")).unwrap_err();
        assert_eq!((err.first, err.second), (id("b"), id("a")));
        event.invoke(&());
        assert_eq!(*log.lock().unwrap(), ["a", "b"]);
    }

    #[test]
    fn first_not_pass_stops_at_the_decision() {
        let event: Event<(), ActionResult> = Event::first_not_pass();
        let later_ran = Arc::new(Mutex::new(false));
        event.register(|_| ActionResult::Pass);
        event.register(|_| ActionResult::Success);
        let flag = Arc::clone(&later_ran);
        event.register(move |_| {
            *flag.lock().unwrap() = true;
            ActionResult::Fail
        });
        assert_eq!(event.invoke(&()), ActionResult::Success);
        assert!(!*later_ran.lock().unwrap());
    }

    #[test]
    fn first_not_pass_with_no_decision_passes() {
        let event: Event<(), ActionResult> = Event::first_not_pass();
        event.register(|_| ActionResult::Pass);
        assert_eq!(event.invoke(&()), ActionResult::Pass);
    }

    #[test]
    fn allow_all_vetoes_on_any_false_and_stops() {
        let event: Event<i32, bool> = Event::allow_all();
        let calls = Arc::new(Mutex::new(0));
        event.register(|n| *n > 0);
        event.register(|n| *n < 10);
        let counter = Arc::clone(&calls);
        event.register(move |_| {
            *counter.lock().unwrap() += 1;
            true
        });
        assert!(event.invoke(&5));
        assert_eq!(*calls.lock().unwrap(), 1);
        assert!(!event.invoke(&50));
        assert_eq!(*calls.lock().unwrap(), 1, "the veto stops the remaining callbacks");
    }

    static GLOBAL: Event<i32, ()> = Event::void();

    #[test]
    fn registering_inside_a_callback_does_not_deadlock() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let outer_seen = Arc::clone(&seen);
        GLOBAL.register(move |n| {
            outer_seen.lock().unwrap().push(*n);
            let inner_seen = Arc::clone(&outer_seen);
            GLOBAL.register(move |m| inner_seen.lock().unwrap().push(m * 100));
        });
        GLOBAL.invoke(&1);
        GLOBAL.invoke(&2);
        assert_eq!(*seen.lock().unwrap(), [1, 2, 200]);
    }
}
