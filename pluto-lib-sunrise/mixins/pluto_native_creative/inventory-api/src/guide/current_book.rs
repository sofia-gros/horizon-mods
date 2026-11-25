//! Conditional operations on a current stored read-only book, not a session.
//! No snapshot proves input ownership, object identity or network authority.
use super::document::{Document, Error as DocumentError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    document: Document,
    page: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    Context,
    NoBook,
    NotReadOnlyView,
    Lectern,
    InvalidPage,
    Document(DocumentError),
}
impl Snapshot {
    pub fn new(document: Document, page: usize) -> Result<Self, Unavailable> {
        if page >= document.pages().len() {
            return Err(Unavailable::InvalidPage);
        }
        Ok(Self { document, page })
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn page(&self) -> usize {
        self.page
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Back,
    Forward,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TurnOutcome {
    pub before: usize,
    pub after: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnError {
    AlreadyInvoked,
    ExpectedDoesNotMatchEntry,
    CurrentChanged,
    Unavailable(Unavailable),
    /// Native turn ran; subsequent facts could not be read. No rollback claim.
    OutcomeUnavailable(Unavailable),
    /// Native turn ran, but its observed result differs. No rollback claim.
    OutcomeChanged,
}
/// A borrowed capability valid only for its adapter's synchronous invocation.
/// A successful call means the native turn ran once, even if an edge kept the
/// page unchanged (native page-turn audio is still part of that operation).
/// It cannot open, replace, close, edit, sign, send or accept a remote response.
pub trait PageTurn {
    fn turn(&mut self, expected: &Snapshot, direction: Direction)
    -> Result<TurnOutcome, TurnError>;
}

/// Pure adapter precondition/proposal; does not authorize or apply an operation.
/// Equality is supported presentation semantics plus page, not object identity.
pub fn check_turn(
    entry: &Snapshot,
    current: &Snapshot,
    expected: &Snapshot,
    direction: Direction,
) -> Result<usize, TurnError> {
    if expected != entry {
        return Err(TurnError::ExpectedDoesNotMatchEntry);
    }
    if current != expected {
        return Err(TurnError::CurrentChanged);
    }
    Ok(match direction {
        Direction::Back => current.page.saturating_sub(1),
        Direction::Forward => (current.page + 1).min(current.document.pages().len() - 1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guide::document::{Span, Style};
    fn snapshot(page: usize) -> Snapshot {
        Snapshot::new(
            Document::new(
                vec![
                    vec![Span {
                        text: "first".into(),
                        style: Style::default(),
                    }],
                    vec![],
                ],
                |_| true,
            )
            .unwrap(),
            page,
        )
        .unwrap()
    }
    #[test]
    fn changed_current_page_or_content_never_proposes_a_turn() {
        let entry = snapshot(0);
        let changed = snapshot(1);
        assert_eq!(
            check_turn(&entry, &changed, &entry, Direction::Forward),
            Err(TurnError::CurrentChanged)
        );
        let mut pages = entry.document.pages().to_vec();
        pages[0][0].style.metadata = Some("different".into());
        let changed = Snapshot::new(Document::new(pages, |_| true).unwrap(), 0).unwrap();
        assert_eq!(
            check_turn(&entry, &changed, &entry, Direction::Forward),
            Err(TurnError::CurrentChanged)
        );
    }
    #[test]
    fn expected_value_must_match_this_invocation_entry() {
        let entry = snapshot(0);
        let other = snapshot(1);
        assert_eq!(
            check_turn(&entry, &other, &other, Direction::Back),
            Err(TurnError::ExpectedDoesNotMatchEntry)
        );
    }
    #[test]
    fn edge_turns_remain_valid_native_invocations_without_wrap() {
        for (page, direction, after) in [
            (0, Direction::Back, 0),
            (0, Direction::Forward, 1),
            (1, Direction::Forward, 1),
            (1, Direction::Back, 0),
        ] {
            let entry = snapshot(page);
            assert_eq!(check_turn(&entry, &entry, &entry, direction), Ok(after));
        }
        let single = Snapshot::new(Document::new(vec![vec![]], |_| true).unwrap(), 0).unwrap();
        assert_eq!(
            check_turn(&single, &single, &single, Direction::Forward),
            Ok(0)
        );
    }
    #[test]
    fn invalid_and_cover_sentinel_pages_are_not_normalized() {
        let document = snapshot(0).document;
        for page in [2, usize::MAX] {
            assert_eq!(
                Snapshot::new(document.clone(), page),
                Err(Unavailable::InvalidPage)
            );
        }
    }
}
