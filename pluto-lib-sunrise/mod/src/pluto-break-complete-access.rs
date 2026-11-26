// The break the game's break-completion seam reports (pluto_break_complete): the mod-facing host has no
// call that runs when a break finishes, so the woven return of the game's own break reports it through
// this sink, and the bridge hears the break's AFTER inside the break instead of at the player's next tick.
use std::sync::OnceLock;

/// A break that went through: the block at `pos` in `dimension` is gone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreakCompleted {
    pub dimension: String,
    pub pos: [i32; 3],
}

/// What the bridge does with a completed break: runs its AFTER for the breaking player.
#[derive(Clone, Copy)]
pub struct BreakCompleteSink {
    pub completed: fn(&mut dyn mod_api::Host, BreakCompleted),
}

static SINK: OnceLock<BreakCompleteSink> = OnceLock::new();

/// Installs the sink the bridge hands the seam; false when one is already installed.
pub fn set_break_complete_sink(sink: BreakCompleteSink) -> Result<(), &'static str> {
    SINK.set(sink).map_err(|_| "a break completion sink is already installed")
}

/// The installed sink, if any.
pub fn break_complete_sink() -> Option<BreakCompleteSink> {
    SINK.get().copied()
}
