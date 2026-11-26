//Read an already-materialized, strictly transient attachment through a shared
//world borrow. No lazy decoding, insertion, persistence or sync is permitted.
//The owning game phase initializes the value through the normal mutable API.
use sunrise_mod_runtime::attachments::{AttachmentError, AttachmentType, Attachments, Entry};

pub fn peek<'a, T: Clone + Send + Sync + 'static>(
    attachments: &'a Attachments,
    ty: &AttachmentType<T>,
) -> Result<Option<&'a T>, AttachmentError> {
    ty.validate_policy()?;
    // A shared read must never materialize saved data or bypass a holder's
    // copy/sync policy. This bridge is only for native transient services.
    if ty.codec.is_some() || ty.copy_on_death || ty.sync {
        return Err(AttachmentError::InvalidCopyPolicy);
    }
    match attachments.entries.get(&(ty.namespace.into(), ty.name.into())) {
        Some(Entry::Decoded(value)) => value.as_any().downcast_ref::<T>()
            .map(Some).ok_or(AttachmentError::TypeMismatch),
        Some(Entry::Raw(_)) => Err(AttachmentError::Decode),
        None => Ok(None),
    }
}
