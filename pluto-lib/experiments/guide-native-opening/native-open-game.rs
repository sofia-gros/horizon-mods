// EXCLUDED private Game-side candidate, appended in game::retained_book.
// Requires the proposed storage field and the unchanged owned/view fragments.
// This does not authorize a caller to open a screen or register a public action.

pub(super) enum NativeOpenRejection {
    NotReadOnlyView,
    EmptyPages,
    IdentityUnavailable,
}

pub(super) enum NativeOpenOutcome {
    /// No installation/tail occurred. The exact candidate remains caller-owned.
    Rejected { reason: NativeOpenRejection, candidate: BookScreen },
    /// Current at return only. Every later mutation needs fresh revalidation.
    Current(OwnedOpening),
    /// Installation/tail occurred, but that opening no longer survives.
    /// Do not take/restore a different current screen or retry automatically.
    Superseded,
}

/// Trusted private call only. Its future caller must separately establish client,
/// actor and opening permission. No existing consumer calls this candidate.
/// No input-only gate is substituted for that still-missing admission boundary.
pub(super) fn open_native_owned_view(
    game: &mut super::Game<'_>,
    candidate: BookScreen,
) -> NativeOpenOutcome {
    match &candidate {
        BookScreen::View { pages, lectern: None, .. } if !pages.is_empty() => {},
        BookScreen::View { pages, lectern: None, .. } if pages.is_empty() => {
            return NativeOpenOutcome::Rejected { reason: NativeOpenRejection::EmptyPages, candidate };
        },
        _ => return NativeOpenOutcome::Rejected { reason: NativeOpenRejection::NotReadOnlyView, candidate },
    }
    let owner = match install_native_owned_value(&mut game.book, candidate, &NEXT) {
        Ok(owner) => owner,
        Err(candidate) => return NativeOpenOutcome::Rejected {
            reason: NativeOpenRejection::IdentityUnavailable, candidate,
        },
    };
    // Exact original open_book tail, once and only after native assignment/drop.
    game.keys.clear();
    game.right = false;
    game.capture(false);
    // Do not compare request-time revision: a page-only change is not a reopen.
    if native_owner_is_current(&game.book, &owner) {
        NativeOpenOutcome::Current(owner)
    } else {
        NativeOpenOutcome::Superseded
    }
}
