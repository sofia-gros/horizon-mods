// Native get -> draw -> keep frame generation. The pinned native loop holds
// its kept-geometry RefCell borrow across this serial pipeline; a nested cache
// pass on that same cache is already forbidden by the native borrow contract.
// Do not recapture generation when keep runs after consumer callbacks.
use std::cell::Cell;
thread_local! { static FRAME: Cell<Option<u64>> = const { Cell::new(None) }; }

pub(crate) fn begin_lookup(generation: Option<u64>) -> Option<u64> {
    FRAME.with(|frame| frame.set(generation));
    generation
}
pub(crate) fn frame_generation() -> u64 {
    // Unavailable provider is handled by clearing cache lookup, not by treating
    // this constructor placeholder as a usable registration generation.
    FRAME.with(|frame| frame.get()).unwrap_or(0)
}
pub(crate) fn reusable(entry: u64, generation: Option<u64>) -> bool {
    generation == Some(entry)
}

// The Sunrise host carries no woven field for an entry's registration
// generation, so the module owns the stamps: keyed by (cache address,
// position) on this frame thread, mirroring the serial pipeline above.
// A get that leaves no reusable entry behind (a miss, or an eviction)
// records a pending stamp at the lookup generation; the entry the
// frame's keep goes on to store belongs to that same generation
// (keep runs inside the lookup's frame), so the next get that finds
// the entry commits the pending stamp as its own.
// A position looked up again at a different generation overwrites the
// pending stamp, which can only cost a spurious eviction, never a
// wrong reuse. A cache cleared for an unavailable generation drops
// its stamps with its entries (see clear_cache's caller).
thread_local! {
    static STAMPS: std::cell::RefCell<std::collections::BTreeMap<(usize, [i32; 3]), u64>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
    static PENDING: std::cell::RefCell<std::collections::BTreeMap<(usize, [i32; 3]), u64>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

/// The committed registration generation of the entry at `pos` in the
/// cache identified by `cache_key`, committing any pending stamp the
/// entry's frame left behind. None when no stamp is known; callers
/// treat that as not reusable — the conservative direction.
pub(crate) fn entry_generation(cache_key: usize, pos: [i32; 3]) -> Option<u64> {
    STAMPS.with(|stamps| {
        if let Some(generation) = stamps.borrow().get(&(cache_key, pos)) {
            return Some(*generation);
        }
        PENDING.with(|pending| {
            let generation = pending.borrow_mut().remove(&(cache_key, pos))?;
            stamps.borrow_mut().insert((cache_key, pos), generation);
            Some(generation)
        })
    })
}

/// Record that the lookup at `generation` left no reusable entry at
/// `pos` (a miss, or an eviction): the entry the frame goes on to
/// keep belongs to this generation. Any older committed stamp for
/// the position is stale and dropped.
pub(crate) fn note_entry_missing(cache_key: usize, pos: [i32; 3], generation: u64) {
    STAMPS.with(|stamps| {
        stamps.borrow_mut().remove(&(cache_key, pos));
    });
    PENDING.with(|pending| {
        pending.borrow_mut().insert((cache_key, pos), generation);
    });
}

/// Drop every stamp belonging to the cache identified by `cache_key`.
pub(crate) fn clear_cache(cache_key: usize) {
    STAMPS.with(|stamps| {
        stamps.borrow_mut().retain(|(owner, _), _| *owner != cache_key);
    });
    PENDING.with(|pending| {
        pending.borrow_mut().retain(|(owner, _), _| *owner != cache_key);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_entries_keep_the_pre_callback_generation_and_missing_provider_never_reuses() {
        let before = begin_lookup(Some(11));
        assert!(reusable(11, before));
        assert_eq!(frame_generation(), 11);
        // Registration changed during the callback: keep must still stamp 11.
        let current = Some(12);
        assert_eq!(frame_generation(), 11);
        assert!(!reusable(frame_generation(), current));
        begin_lookup(current);
        assert_eq!(frame_generation(), 12);
        assert!(reusable(12, current));
        begin_lookup(None);
        assert!(!reusable(0, None));
        assert!(!reusable(12, None));
    }

    #[test]
    fn stamps_commit_from_the_missing_lookup_and_clear_with_their_cache() {
        let key = 0xCA11usize;
        // No stamp is known for an entry nobody looked up.
        assert_eq!(entry_generation(key, [1, 2, 3]), None);
        // A lookup that misses leaves the pending stamp the keep commits.
        note_entry_missing(key, [1, 2, 3], 11);
        assert_eq!(entry_generation(key, [1, 2, 3]), Some(11));
        assert!(reusable(entry_generation(key, [1, 2, 3]).unwrap(), Some(11)));
        assert!(!reusable(entry_generation(key, [1, 2, 3]).unwrap(), Some(12)));
        // A later miss at a new generation retires the committed stamp.
        note_entry_missing(key, [1, 2, 3], 12);
        assert_eq!(entry_generation(key, [1, 2, 3]), Some(12));
        // Positions and caches are independent.
        note_entry_missing(key, [4, 5, 6], 12);
        note_entry_missing(0xCA22, [1, 2, 3], 7);
        assert_eq!(entry_generation(0xCA22, [1, 2, 3]), Some(7));
        clear_cache(key);
        assert_eq!(entry_generation(key, [1, 2, 3]), None);
        assert_eq!(entry_generation(key, [4, 5, 6]), None);
        assert_eq!(entry_generation(0xCA22, [1, 2, 3]), Some(7));
        clear_cache(0xCA22);
    }
}
