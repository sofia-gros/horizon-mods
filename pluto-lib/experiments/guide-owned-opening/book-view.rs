// EXCLUDED actual-type caller candidate, NOT included by the std-only fixture.
// Proposed placement: append after owned.rs inside native game::retained_book.
// No Game entrypoint, packet callback, actor/locale policy or installed hook.
use super::books::BookScreen;
use horizon_client::text::Text;

pub(super) enum ViewRejection {
    EmptyPages,
    NotReadOnlyView,
    Storage(PrepareError),
}
pub(super) struct RejectedPages {
    pub(super) reason: ViewRejection,
    pub(super) pages: Vec<Text>,
}

pub(super) struct PreparedView<'a> {
    storage: PreparedReplacement<'a, BookScreen>,
    replacement: BookScreen,
}
impl PreparedView<'_> {
    /// A future adapter can consume its already-validated pending work while
    /// this guard holds the slot exclusively, then perform this one commit.
    /// Dropping this guard instead releases the slot without changing it.
    pub(super) fn commit(self) -> BookScreen { self.storage.commit(self.replacement) }
}

/// Conditional retained-view replacement, preserving the legacy cover/clamp.
/// It intentionally does NOT require input eligibility: overlays may retain a
/// view. This is insufficient to admit a Game actor or authoritative LAN reply.
/// The old BookScreen is returned; no old page destructor runs in this function.
pub(super) fn prepare_owned_view<'a>(
    slot: &'a mut RetainedBook<BookScreen>,
    owner: &OwnedOpening,
    expected: Stamp,
    pages: Vec<Text>,
) -> Result<PreparedView<'a>, RejectedPages> {
    if pages.is_empty() { return Err(RejectedPages { reason: ViewRejection::EmptyPages, pages }); }
    let page = match slot.as_ref() {
        Some(BookScreen::View { page, lectern: None, .. }) => {
            if *page == usize::MAX { usize::MAX } else { (*page).min(pages.len() - 1) }
        }
        _ => return Err(RejectedPages { reason: ViewRejection::NotReadOnlyView, pages }),
    };
    let prepared = match slot.prepare_owned_replace(owner, expected) {
        Ok(prepared) => prepared,
        Err(reason) => return Err(RejectedPages { reason: ViewRejection::Storage(reason), pages }),
    };
    Ok(PreparedView { storage: prepared, replacement: BookScreen::View { pages, page, lectern: None } })
}
