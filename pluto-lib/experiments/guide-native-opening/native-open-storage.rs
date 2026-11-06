// EXCLUDED private implementation for native assignment compatibility.
// Append in the existing retained_book module. No second store/ID allocator.
// The generic function is private; only the concrete BookScreen adapter uses it.

fn install_native_owned_value<T>(
    slot: &mut RetainedBook<T>,
    value: T,
    counter: &AtomicU64,
) -> Result<OwnedOpening, T> {
    let Some(attachment) = slot.attachment else { return Err(value); };
    let Some(opening) = fresh(counter) else { return Err(value); };
    // Match the existing native set's revocation and Option assignment order.
    // Unlike the generic try_install_owned API, the old concrete value is
    // dropped here, before the new opening is published and before Game's tail.
    // A destructor panic returns no owner and leaves opening unavailable.
    slot.opening = None;
    slot.revision = 0;
    slot.value = Some(value);
    slot.opening = Some(opening);
    Ok(OwnedOpening { attachment, opening })
}

fn native_owner_is_current<T>(slot: &RetainedBook<T>, owner: &OwnedOpening) -> bool {
    slot.value.is_some()
        && slot.attachment == Some(owner.attachment)
        && slot.opening == Some(owner.opening)
}
