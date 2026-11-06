// EXCLUDED source-only extension. Include after the frozen retained_book.rs in
// the SAME private native module; do not create a second store or ID allocator.
// This is local storage capability, not actor, locale, packet or UI authority.

/// Minted only while this service explicitly installs a new owned value.
/// No Clone, Copy, deserialization or conversion from an observed Stamp.
pub(super) struct OwnedOpening {
    attachment: u64,
    opening: u64,
}

pub(super) struct Installed<T> {
    pub(super) owner: OwnedOpening,
    /// Returned, not dropped, after the new storage state is fully committed.
    pub(super) previous: Option<T>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PrepareError {
    NoCurrentOpening,
    WrongOwner,
    StaleObservation,
    RevisionExhausted,
}

/// Exclusive access from final storage revalidation through replacement.
/// Dropping this uncommitted guard has no effect. It accepts no user callback.
pub(super) struct PreparedReplacement<'a, T> {
    slot: &'a mut RetainedBook<T>,
    next_revision: u64,
}

impl<T> RetainedBook<T> {
    /// Only this explicit installation can create an OwnedOpening. Ordinary
    /// native set/replace/restore remains unowned and cannot be claimed later.
    /// Failure returns the exact candidate and leaves all old fields untouched.
    pub(super) fn try_install_owned(&mut self, value: T) -> Result<Installed<T>, T> {
        self.try_install_owned_using(value, &NEXT)
    }

    fn try_install_owned_using(&mut self, value: T, counter: &AtomicU64) -> Result<Installed<T>, T> {
        let Some(attachment) = self.attachment else { return Err(value); };
        let Some(opening) = fresh(counter) else { return Err(value); };
        // No destructor or caller code executes during these field operations.
        let previous = self.value.replace(value);
        self.opening = Some(opening);
        self.revision = 0;
        Ok(Installed { owner: OwnedOpening { attachment, opening }, previous })
    }

    /// `expected` is a fresh commit-time observation, not the request-time page
    /// revision. Native page turns may advance revision without revoking owner.
    /// This validates only the private slot. A future Game adapter must also
    /// validate actor, locale/resource inputs and pending reply independently.
    pub(super) fn prepare_owned_replace(
        &mut self,
        owner: &OwnedOpening,
        expected: Stamp,
    ) -> Result<PreparedReplacement<'_, T>, PrepareError> {
        let current = self.observed_stamp().ok_or(PrepareError::NoCurrentOpening)?;
        if current.attachment != owner.attachment || current.opening != owner.opening {
            return Err(PrepareError::WrongOwner);
        }
        if current != expected { return Err(PrepareError::StaleObservation); }
        let next_revision = self.revision.checked_add(1).ok_or(PrepareError::RevisionExhausted)?;
        Ok(PreparedReplacement { slot: self, next_revision })
    }
}

impl<T> PreparedReplacement<'_, T> {
    /// Infallible under this private guard's invariant: a Some value was
    /// validated and the unique mutable borrow prevents intervening slot edits.
    /// Returns the old value so its Drop cannot run inside the commit operation.
    /// Callers must release their enclosing Game/RefCell borrow before dropping
    /// it; later destructor effects/panics do not imply rollback or safe retry.
    pub(super) fn commit(self, value: T) -> T {
        let previous = self.slot.value.replace(value);
        self.slot.revision = self.next_revision;
        previous.expect("prepared replacement retains an existing value")
    }
}
