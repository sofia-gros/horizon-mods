// One provider can alter only the native policy entries under its registration.
// Numeric validation/composition has one owner: the native protection Policy.
use crate::pluto_inventory::protection::Contribution;
use horizon_world::combat_rules::pluto_protection::Policy;

pub(super) struct Scoped<'a> {
    id: &'a str,
    policy: &'a mut Policy,
}
impl<'a> Scoped<'a> {
    pub(super) fn new(id: &'a str, policy: &'a mut Policy) -> Self {
        Self { id, policy }
    }
}
impl Contribution for Scoped<'_> {
    fn set_lower(&mut self, lower: f32) -> bool {
        self.policy.set_lower(self.id, lower)
    }
    fn set_upper(&mut self, upper: f32) -> bool {
        self.policy.set_upper(self.id, upper)
    }
    fn set_additive(&mut self, points: f32) -> bool {
        self.policy.set_additive(self.id, points)
    }
    fn clear(&mut self) {
        self.policy.remove(self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pluto_provider_capability_cannot_overwrite_another_registration() {
        let mut policy = Policy::default();
        policy.set_lower("other:a", -8.0);
        policy.set_upper("other:a", 22.0);
        policy.set_additive("other:a", 4.0);
        let mut scoped = Scoped::new("test:b", &mut policy);
        assert!(scoped.set_lower(-2.0));
        assert!(scoped.set_upper(24.0));
        assert!(scoped.set_additive(-3.0));
        assert!(!scoped.set_additive(f32::NAN));
        assert!(!scoped.set_upper(26.0));
        assert!(!scoped.set_lower(1.0));
        drop(scoped);
        assert_eq!(policy.bounds().lower, -8.0);
        assert_eq!(policy.bounds().upper, 24.0);
        assert_eq!(policy.additive("test:b"), Some(-3.0));
        Scoped::new("test:b", &mut policy).clear();
        assert_eq!(policy.bounds().lower, -8.0);
        assert_eq!(policy.bounds().upper, 22.0);
        assert_eq!(policy.additive("other:a"), Some(4.0));
        assert_eq!(policy.additive("test:b"), None);
    }
}
