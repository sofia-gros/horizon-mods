// Unapplied official-host storage prototype authored in horizon-mods.
// Observed identities are NOT owner handles or permission to apply a reply.
// No sender, packet callback, registry, presentation policy or UI action here.
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

fn fresh(counter: &AtomicU64) -> Option<u64> {
    counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1)).ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    attachment: u64,
    opening: u64,
    revision: u64,
}

/// Private host storage, parameterized here so its mechanics can be tested
/// without constructing an actual Client/renderer. The proposed Game field
/// instantiates this with the real books::BookScreen, not a replacement enum.
pub(super) struct RetainedBook<T> {
    value: Option<T>,
    attachment: Option<u64>,
    opening: Option<u64>,
    revision: u64,
}

impl<T> Default for RetainedBook<T> {
    fn default() -> Self {
        Self { value: None, attachment: fresh(&NEXT), opening: None, revision: 0 }
    }
}

impl<T> RetainedBook<T> {
    pub(super) fn is_some(&self) -> bool { self.value.is_some() }
    pub(super) fn is_none(&self) -> bool { self.value.is_none() }
    pub(super) fn as_ref(&self) -> Option<&T> { self.value.as_ref() }

    /// Compatibility borrow for the pinned native field-mutation call sites.
    /// A borrow advances revision even if that caller ultimately does no write.
    /// It does NOT establish exact mutation events or prevent arbitrary trusted
    /// native code replacing T through this reference. Such a new call site
    /// needs review; this prototype is not a public untrusted mutation API.
    pub(super) fn as_mut(&mut self) -> Option<&mut T> {
        if self.value.is_some() {
            match self.revision.checked_add(1) {
                Some(revision) => self.revision = revision,
                None => self.opening = None,
            }
        }
        self.value.as_mut()
    }

    pub(super) fn observed_stamp(&self) -> Option<Stamp> {
        self.value.as_ref()?;
        Some(Stamp { attachment: self.attachment?, opening: self.opening?, revision: self.revision })
    }

    /// Keep ordinary assignment's destructor ordering. Revocation happens
    /// before dropping the old T; if Drop panics, identity remains unavailable.
    /// Identity exhaustion never prevents native local contents being installed.
    pub(super) fn set(&mut self, value: Option<T>) {
        self.set_using(value, &NEXT);
    }
    fn set_using(&mut self, value: Option<T>, counter: &AtomicU64) {
        self.opening = None;
        self.revision = 0;
        self.value = value;
        self.opening = self.value.as_ref().and_then(|_| fresh(counter));
    }

    /// Keep mem::replace's ownership/drop behavior for guest save/restore.
    pub(super) fn replace(&mut self, value: Option<T>) -> Option<T> {
        self.opening = None;
        self.revision = 0;
        let old = std::mem::replace(&mut self.value, value);
        self.opening = self.value.as_ref().and_then(|_| fresh(&NEXT));
        old
    }
    pub(super) fn take(&mut self) -> Option<T> { self.replace(None) }

    /// Called at a real player/connection transition in the proposed patch,
    /// not on generic overlay appearance or a polling screen-name comparison.
    pub(super) fn invalidate_attachment(&mut self) {
        self.attachment = fresh(&NEXT);
    }
}

#[cfg(test)]
mod storage_tests {
    use super::*;
    #[test]
    fn actual_option_shape_remains_available_to_native_readers() {
        enum View { Edit, Read { page: usize } }
        let mut slot = RetainedBook::default();
        assert!(slot.is_none()); assert!(slot.as_ref().is_none());
        slot.set(Some(View::Read { page: usize::MAX }));
        assert!(matches!(slot.as_ref(), Some(View::Read { .. })));
        let Some(View::Read { page }) = slot.as_mut() else { panic!() };
        assert_eq!(*page, usize::MAX); *page = 2;
        assert!(matches!(slot.as_ref(), Some(View::Read { page: 2 })));
        slot.set(Some(View::Edit)); assert!(slot.is_some());
    }
    #[test]
    fn equal_replacement_and_restoration_never_reuse_opening() {
        let mut slot = RetainedBook::default(); slot.set(Some("same"));
        let first = slot.observed_stamp().unwrap(); slot.set(Some("same"));
        assert_ne!(slot.observed_stamp().unwrap().opening, first.opening);
        let saved = slot.take(); assert!(slot.observed_stamp().is_none());
        slot.set(saved); assert_ne!(slot.observed_stamp().unwrap().opening, first.opening);
    }
    #[test]
    fn borrowed_mutation_advances_revision_without_replacing_opening() {
        let mut slot = RetainedBook::default(); slot.set(Some(1)); let before = slot.observed_stamp().unwrap();
        *slot.as_mut().unwrap() = 2; let after = slot.observed_stamp().unwrap();
        assert_eq!(before.opening, after.opening); assert_eq!(after.revision, before.revision + 1);
        let _ = slot.as_mut(); assert_eq!(slot.observed_stamp().unwrap().revision, after.revision + 1);
    }
    #[test]
    fn overlay_does_not_mutate_slot_and_attachment_reset_preserves_contents() {
        let mut slot = RetainedBook::default(); slot.set(Some("retained")); let before = slot.observed_stamp();
        let mut unrelated_dialog = Some("overlay"); assert_eq!(slot.observed_stamp(), before);
        unrelated_dialog.take(); assert_eq!(slot.observed_stamp(), before);
        slot.invalidate_attachment(); assert_ne!(slot.observed_stamp(), before);
        assert_eq!(slot.as_ref(), Some(&"retained"));
    }
    #[test]
    fn replace_returns_old_contents_without_dropping_them() {
        use std::sync::{Arc, atomic::AtomicUsize};
        struct Value(Arc<AtomicUsize>);
        impl Drop for Value { fn drop(&mut self) { self.0.fetch_add(1, Ordering::SeqCst); } }
        let count = Arc::new(AtomicUsize::new(0)); let mut slot = RetainedBook::default();
        slot.set(Some(Value(count.clone()))); let old = slot.replace(Some(Value(count.clone())));
        assert_eq!(count.load(Ordering::SeqCst), 0); drop(old);
        assert_eq!(count.load(Ordering::SeqCst), 1); drop(slot);
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn id_and_revision_exhaustion_do_not_disable_native_content_changes() {
        let mut slot = RetainedBook::default(); let exhausted = AtomicU64::new(u64::MAX);
        slot.set_using(Some(4), &exhausted); assert_eq!(slot.as_ref(), Some(&4)); assert!(slot.observed_stamp().is_none());
        slot.set(Some(5)); slot.revision = u64::MAX; *slot.as_mut().unwrap() = 6;
        assert_eq!(slot.as_ref(), Some(&6)); assert!(slot.observed_stamp().is_none());
        assert_eq!(fresh(&exhausted), None); assert_eq!(exhausted.load(Ordering::Relaxed), u64::MAX);
    }
    #[test]
    fn assignment_destructor_panic_cannot_leave_a_valid_old_stamp() {
        use std::sync::{Arc, atomic::AtomicBool};
        struct Value(Arc<AtomicBool>);
        impl Drop for Value { fn drop(&mut self) { if self.0.swap(false, Ordering::SeqCst) { panic!("old value drop") } } }
        let mut slot = RetainedBook::default(); slot.set(Some(Value(Arc::new(AtomicBool::new(true)))));
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| slot.set(None)));
        assert!(outcome.is_err()); assert!(slot.observed_stamp().is_none());
    }
}
