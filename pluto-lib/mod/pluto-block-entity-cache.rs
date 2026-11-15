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
}
