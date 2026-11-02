//! Experimental mapping for the official use hook; this is not a runtime adapter.
use pluto_item_api::UseOutcome;

#[derive(Debug, PartialEq, Eq)]
pub struct UseDecision {
    pub claim: bool,
    pub commit_data: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FailureFeedbackUnavailable;

/// The official `claimed` flag swings the hand; it cannot express the old
/// Cancel outcome's failed, non-swinging response.
pub fn decision(outcome: UseOutcome) -> Result<UseDecision, FailureFeedbackUnavailable> {
    match outcome {
        UseOutcome::Pass => Ok(UseDecision { claim: false, commit_data: false }),
        UseOutcome::Handled => Ok(UseDecision { claim: true, commit_data: true }),
        UseOutcome::Cancel => Err(FailureFeedbackUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_the_three_existing_outcomes() {
        assert_eq!(decision(UseOutcome::Pass), Ok(UseDecision { claim: false, commit_data: false }));
        assert_eq!(decision(UseOutcome::Handled), Ok(UseDecision { claim: true, commit_data: true }));
        assert_eq!(decision(UseOutcome::Cancel), Err(FailureFeedbackUnavailable));
    }
}
