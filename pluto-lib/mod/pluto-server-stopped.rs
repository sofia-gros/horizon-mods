// The independent API crate installs the dispatcher before any server starts.
use std::sync::OnceLock;

static STOPPED: OnceLock<fn(bool)> = OnceLock::new();

pub fn set_dispatcher(dispatch: fn(bool)) {
    let _ = STOPPED.set(dispatch);
}

pub fn finish(value: crate::Result<()>, dedicated: bool) -> crate::Result<()> {
    // The outer loop returns Ok only after the final save and teardown.
    if value.is_ok() && let Some(dispatch) = STOPPED.get() {
        dispatch(dedicated);
    }
    value
}
