//! `ActionResult` (Fabric/Minecraft): what an interaction callback decided.

/// The outcome of an interaction callback such as `UseItemCallback`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionResult {
    /// The action happened; swing the hand.
    Success,
    /// The action happened; do not swing the hand.
    Consume,
    /// The action partly happened.
    ConsumePartial,
    /// Not handled here; let the next callback or the game decide.
    Pass,
    /// Cancel the action.
    Fail,
}

impl ActionResult {
    /// Minecraft: `isAccepted`. True for every result except `Pass` and `Fail`.
    pub fn is_accepted(self) -> bool {
        matches!(self, Self::Success | Self::Consume | Self::ConsumePartial)
    }

    /// Minecraft: `shouldSwingHand`.
    pub fn should_swing_hand(self) -> bool {
        self == Self::Success
    }
}
