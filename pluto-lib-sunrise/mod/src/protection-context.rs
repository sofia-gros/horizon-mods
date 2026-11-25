// Scoped bridge for the official player's post-resistance protection stage.
// No equipment, modifier IDs, or consuming-mod formulas belong here.
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    marker::PhantomData,
    rc::Rc,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub lower: f32,
    pub upper: f32,
}
impl Default for Bounds {
    fn default() -> Self {
        Self {
            lower: 0.0,
            upper: 20.0,
        }
    }
}
impl Bounds {
    pub fn valid(self) -> bool {
        self.lower.is_finite()
            && self.lower <= 0.0
            && self.upper.is_finite()
            && (20.0..=25.0).contains(&self.upper)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Policy {
    bounds: BTreeMap<String, Bounds>,
    additive: BTreeMap<String, f32>,
}
impl Policy {
    pub fn set_lower(&mut self, provider: &str, lower: f32) -> bool {
        let mut bounds = self.bounds.get(provider).copied().unwrap_or_default();
        bounds.lower = lower;
        self.set(provider, bounds)
    }
    pub fn set_upper(&mut self, provider: &str, upper: f32) -> bool {
        let mut bounds = self.bounds.get(provider).copied().unwrap_or_default();
        bounds.upper = upper;
        self.set(provider, bounds)
    }
    pub fn set(&mut self, provider: &str, bounds: Bounds) -> bool {
        if provider.is_empty() || !bounds.valid() {
            return false;
        }
        if bounds == Bounds::default() {
            self.bounds.remove(provider);
        } else {
            self.bounds.insert(provider.to_owned(), bounds);
        }
        true
    }
    pub fn remove(&mut self, provider: &str) {
        self.bounds.remove(provider);
        self.additive.remove(provider);
    }
    /// Replace this provider's additive raw protection points. Signed finite
    /// values are accepted; zero removes only the additive contribution.
    /// Bounds are independent. Invalid writes preserve the prior value.
    pub fn set_additive(&mut self, provider: &str, points: f32) -> bool {
        if provider.is_empty() || !points.is_finite() {
            return false;
        }
        if points == 0.0 {
            self.additive.remove(provider);
        } else {
            self.additive.insert(provider.to_owned(), points);
        }
        true
    }
    pub fn additive(&self, provider: &str) -> Option<f32> {
        self.additive.get(provider).copied()
    }
    // Native raw first, then stable provider-ID order, using f32 additions.
    // A nonfinite intermediate rejects the whole additive batch for this hit;
    // it never keeps a partial prefix or silently changes independent bounds.
    fn apply_additive(&self, raw: f32) -> f32 {
        self.additive
            .values()
            .try_fold(raw, |total, points| {
                let next = total + points;
                next.is_finite().then_some(next)
            })
            .unwrap_or(raw)
    }
    pub fn bounds(&self) -> Bounds {
        self.bounds.values().fold(Bounds::default(), |a, b| Bounds {
            lower: a.lower.min(b.lower),
            upper: a.upper.max(b.upper),
        })
    }
}

// Only the immediately armed, borrowed native Environment status may project a
// marker. No pointer is dereferenced or retained beyond the synchronous scope.
const MARKER: f32 = f32::MAX;
struct Frame {
    id: u64,
    policy: Policy,
    reentry: bool,
    status: Option<usize>,
    pending: Option<(f32, f32)>,
}
thread_local! {
    static NEXT: Cell<u64> = const { Cell::new(0) };
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
}
/// Synchronous, thread-bound hit scope. Drop restores nested/error paths.
pub struct Context {
    id: u64,
    _thread: PhantomData<Rc<()>>,
}
impl Context {
    pub fn enter(policy: Policy) -> Self {
        let id = NEXT.with(|next| {
            let id = next
                .get()
                .checked_add(1)
                .expect("protection scope counter exhausted");
            next.set(id);
            id
        });
        FRAMES.with(|frames| {
            frames.borrow_mut().push(Frame {
                id,
                policy,
                reentry: false,
                status: None,
                pending: None,
            })
        });
        Self {
            id,
            _thread: PhantomData,
        }
    }
    /// Permit exactly the immediate call back into the preserved native method.
    pub fn allow_reentry(&self) {
        FRAMES.with(|frames| {
            if let Some(frame) = frames.borrow_mut().last_mut().filter(|f| f.id == self.id) {
                frame.reentry = true;
            }
        });
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        FRAMES.with(|frames| frames.borrow_mut().retain(|f| f.id != self.id));
    }
}
pub fn take_reentry() -> bool {
    FRAMES.with(|frames| {
        frames
            .borrow_mut()
            .last_mut()
            .is_some_and(|f| std::mem::take(&mut f.reentry))
    })
}
pub fn set_current(policy: Policy) -> bool {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        let Some(frame) = frames.last_mut() else {
            return false;
        };
        frame.policy = policy;
        true
    })
}
pub fn set_current_lower(provider: &str, lower: f32) -> bool {
    FRAMES.with(|frames| {
        frames
            .borrow_mut()
            .last_mut()
            .is_some_and(|f| f.policy.set_lower(provider, lower))
    })
}
pub fn set_current_upper(provider: &str, upper: f32) -> bool {
    FRAMES.with(|frames| {
        frames
            .borrow_mut()
            .last_mut()
            .is_some_and(|f| f.policy.set_upper(provider, upper))
    })
}
/// Update additive raw protection only in the current explicit hit scope.
/// Call before the native protection read, just like the bound setters.
pub fn set_current_additive(provider: &str, points: f32) -> bool {
    FRAMES.with(|frames| {
        frames
            .borrow_mut()
            .last_mut()
            .is_some_and(|f| f.policy.set_additive(provider, points))
    })
}
/// Private adapter protocol: arm at the native environment's server-level read.
pub fn arm(status: usize) {
    FRAMES.with(|frames| {
        if let Some(f) = frames.borrow_mut().last_mut() {
            f.status = Some(status);
            f.pending = None;
        }
    });
}
/// Private adapter protocol: the matching status read consumes the arm once.
pub fn project(raw: f32, status: usize) -> f32 {
    FRAMES.with(|frames| {
        let mut frames = frames.borrow_mut();
        let Some(f) = frames.last_mut() else {
            return raw;
        };
        if f.status != Some(status) {
            return raw;
        }
        f.status = None;
        f.pending = None;
        if !raw.is_finite() {
            return raw;
        }
        let bounds = f.policy.bounds();
        let combined = f.policy.apply_additive(raw);
        let value = combined.clamp(bounds.lower, bounds.upper);
        if !(0.0..=20.0).contains(&value) {
            f.pending = Some((value, raw));
            MARKER
        } else if combined == raw {
            // No effective additive change: retain native raw/gate semantics.
            raw
        } else {
            // The native positive-protection gate and scalar stage can handle
            // this value directly. A nonpositive result needs no marker.
            value
        }
    })
}
/// Private adapter protocol: preserve the original post-resistance stage.
pub fn after_magic(damage: f32, protection: f32) -> Option<f32> {
    FRAMES.with(|frames| {
        let pending = frames
            .borrow_mut()
            .last_mut()
            .and_then(|f| f.pending.take());
        if protection == MARKER {
            pending.map(|(points, raw)| {
                let extended = damage * (1.0 - points / 25.0);
                if extended.is_finite() {
                    extended
                }
                // A finite policy can still overflow f32 multiplication. Fail
                // closed to the native raw-EPF path, including nonfinite damage.
                else if raw > 0.0 {
                    damage * (1.0 - raw.clamp(0.0, 20.0) / 25.0)
                } else {
                    damage
                }
            })
        } else {
            None
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn policy(lower: f32, upper: f32) -> Policy {
        let mut p = Policy::default();
        assert!(p.set("example", Bounds { lower, upper }));
        p
    }
    #[test]
    fn native_scope_and_bounds() {
        let _c = Context::enter(policy(-20.0, 23.75));
        arm(10);
        assert_eq!(after_magic(10.0, project(-30.0, 10)), Some(18.0));
        arm(10);
        assert!((after_magic(10.0, project(40.0, 10)).unwrap() - 0.5).abs() < 0.000001);
        assert_eq!(after_magic(10.0, MARKER), None);
    }
    #[test]
    fn ordinary_queries_and_default_policy_are_unchanged() {
        assert_eq!(project(-4.0, 10), -4.0);
        let _c = Context::enter(policy(-20.0, 23.75));
        arm(10);
        assert_eq!(project(-5.0, 11), -5.0);
        assert_eq!(project(-5.0, 10), MARKER);
        assert_eq!(project(-5.0, 10), -5.0);
        assert_eq!(after_magic(10.0, MARKER), Some(12.0));
        set_current(Policy::default());
        arm(10);
        assert_eq!(project(-5.0, 10), -5.0);
    }
    #[test]
    fn nested_scopes_errors_and_unwind_restore_outer() {
        let outer = Context::enter(policy(-20.0, 23.75));
        arm(10);
        let marker = project(-5.0, 10);
        {
            let _inner = Context::enter(policy(-20.0, 20.0));
            arm(20);
            assert_eq!(after_magic(10.0, project(-10.0, 20)), Some(14.0));
        }
        let _ = std::panic::catch_unwind(|| {
            let _inner = Context::enter(Policy::default());
            panic!("test");
        });
        assert_eq!(after_magic(10.0, marker), Some(12.0));
        arm(10);
        project(-5.0, 10);
        drop(outer);
        assert_eq!(after_magic(10.0, MARKER), None);
    }
    #[test]
    fn reentry_is_one_shot_and_nested_hits_are_new_scopes() {
        let outer = Context::enter(Policy::default());
        assert!(!take_reentry());
        outer.allow_reentry();
        assert!(take_reentry());
        assert!(!take_reentry());
        {
            let inner = Context::enter(Policy::default());
            inner.allow_reentry();
            assert!(take_reentry());
        }
        assert!(!take_reentry());
    }
    #[test]
    fn arithmetic_overflow_uses_native_raw_protection_fallback() {
        let _scope = Context::enter(policy(-f32::MAX, 25.0));
        arm(10);
        assert_eq!(
            after_magic(f32::MAX, project(-f32::MAX, 10)),
            Some(f32::MAX)
        );
        arm(10);
        assert_eq!(
            after_magic(f32::INFINITY, project(30.0, 10)),
            Some(f32::INFINITY)
        );
    }
    #[test]
    fn threads_and_invalid_numbers() {
        let _c = Context::enter(policy(-20.0, 23.75));
        arm(10);
        let marker = project(-5.0, 10);
        std::thread::spawn(move || assert_eq!(after_magic(10.0, marker), None))
            .join()
            .unwrap();
        assert_eq!(after_magic(10.0, marker), Some(12.0));
        arm(10);
        assert!(project(f32::NAN, 10).is_nan());
        assert_eq!(after_magic(1.0, MARKER), None);
        let mut p = policy(-10.0, 21.0);
        assert!(!p.set_upper("other", f32::NAN));
        assert!(p.set_lower("other", -20.0));
        assert_eq!(p.bounds().lower, -20.0);
        p.remove("other");
        assert_eq!(p.bounds().lower, -10.0);
    }
    #[test]
    fn additive_writes_replace_reject_and_remove_independently() {
        let mut p = policy(-10.0, 23.0);
        assert!(p.set_additive("example", 3.0));
        assert!(p.set_additive("example", -4.0));
        assert!(!p.set_additive("example", f32::NAN));
        assert!(!p.set_additive("example", f32::INFINITY));
        assert!(!p.set_additive("", 1.0));
        assert_eq!(p.additive("example"), Some(-4.0));
        assert!(p.set("example", Bounds::default()));
        assert_eq!(p.additive("example"), Some(-4.0));
        assert_eq!(p.bounds(), Bounds::default());
        assert!(p.set_lower("example", -5.0));
        assert!(p.set_additive("example", -0.0));
        assert_eq!(p.additive("example"), None);
        assert_eq!(p.bounds().lower, -5.0);
        assert!(p.set_additive("other", 2.0));
        assert!(p.set_additive("example", 6.0));
        p.remove("example");
        assert_eq!(p.additive("other"), Some(2.0));
        assert_eq!(p.additive("example"), None);
        assert_eq!(p.bounds(), Bounds::default());
    }
    #[test]
    fn additive_order_is_stable_and_uses_uncapped_native_base() {
        let mut first = Policy::default();
        first.set_additive("z:last", 3.0);
        first.set_additive("a:first", -1.0e20);
        let mut second = Policy::default();
        second.set_additive("a:first", -1.0e20);
        second.set_additive("z:last", 3.0);
        assert_eq!(first.apply_additive(1.0e20), 3.0);
        assert_eq!(second.apply_additive(1.0e20), 3.0);
        let mut p = Policy::default();
        p.set_additive("example", -15.0);
        let _scope = Context::enter(p);
        arm(7);
        assert_eq!(project(30.0, 7), 15.0, "sum precedes native/policy cap");
    }
    #[test]
    fn negative_addition_does_not_implicitly_lower_the_cap_floor() {
        let mut p = Policy::default();
        p.set_additive("example", -15.0);
        let _scope = Context::enter(p);
        arm(7);
        assert_eq!(project(10.0, 7), 0.0);
        assert!(set_current_lower("bounds-owner", -20.0));
        arm(7);
        assert_eq!(after_magic(10.0, project(10.0, 7)), Some(12.0));
    }
    #[test]
    fn additive_overflow_discards_the_entire_batch_but_preserves_bounds() {
        let mut p = Policy::default();
        p.set_additive("a", 1.0);
        p.set_additive("b", f32::MAX);
        p.set_additive("c", f32::MAX);
        assert_eq!(p.apply_additive(1.0), 1.0, "no partial prefix survives");
        p.set_upper("bounds-owner", 25.0);
        let _scope = Context::enter(p);
        arm(7);
        assert_eq!(after_magic(10.0, project(30.0, 7)), Some(0.0));
        arm(7);
        assert_eq!(project(f32::INFINITY, 7), f32::INFINITY);
        arm(7);
        assert!(project(f32::NAN, 7).is_nan());
    }
    #[test]
    fn additive_updates_are_scoped_and_do_not_change_unrelated_reads() {
        assert!(!set_current_additive("example", 5.0));
        let scope = Context::enter(Policy::default());
        assert!(set_current_additive("example", 5.0));
        arm(7);
        assert_eq!(project(0.0, 8), 0.0);
        assert_eq!(project(0.0, 7), 5.0);
        assert_eq!(project(0.0, 7), 0.0);
        {
            let _nested = Context::enter(Policy::default());
            arm(7);
            assert_eq!(project(0.0, 7), 0.0);
        }
        arm(7);
        assert_eq!(project(0.0, 7), 5.0);
        drop(scope);
        assert!(!set_current_additive("example", 5.0));
        assert_eq!(project(0.0, 7), 0.0);
    }
}
