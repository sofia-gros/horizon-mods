//! Bounded read-only guide content. Content, layout, ownership, sessions and
//! transport authorization belong to separately validated adapters/consumers.
pub mod document;

/// Synchronous stored-book facts and conditional native page-turn contract.
pub mod current_book;

/// Explicit in-process correlation, not native session or screen ownership.
pub mod requests;

// Short paths to the same contracts, not alternative types or state owners.
pub use document::{Document, Page, Span, Style};
pub use requests::RequestTracker;

#[cfg(test)]
mod entry_tests {
    use super::*;

    #[test]
    fn short_document_paths_preserve_types_and_validation() {
        let pages: Vec<Page> = vec![
            vec![Span::plain("Example guide"), Span::styled("Next", Style {
                page: Some(2), ..Style::default()
            })],
            vec![Span::plain(String::from("Second page"))],
        ];
        let document: document::Document = Document::new(pages, |_| false).unwrap();
        assert_eq!(document.pages()[0][0], document::Span {
            text: "Example guide".into(), style: document::Style::default()
        });
        assert_eq!(document::decode(&document::encode(&document).unwrap(), |_| false).unwrap(), document);
        let bad_link = Span::styled("Missing", Style { page: Some(2), ..Style::default() });
        assert_eq!(Document::new(vec![vec![bad_link]], |_| true), Err(document::Error::Link));
        let metadata = Span::styled("Opaque", Style { metadata: Some("example:data".into()), ..Style::default() });
        assert_eq!(Document::new(vec![vec![metadata]], |_| false), Err(document::Error::Metadata));
    }

    #[test]
    fn short_request_path_does_not_activate_or_own_a_screen() {
        let tracker: requests::RequestTracker = RequestTracker::new("example:guide", "example:reply").unwrap();
        assert!(!tracker.is_active());
        assert!(!tracker.has_pending());
    }
}
