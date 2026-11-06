// TConstruct compatibility adapter: use the existing bounded Pluto queries
// where admissible, otherwise retain the original native read-only mechanics.
// No screen/session handle, action execution, wire codec or persistent cache.
use horizon_client::{
    font::Font,
    pluto_guide_layout::{self as layout, Wrapped},
    pluto_guide_text,
    pluto_inventory::guide::document::Document,
    text::{ClickEvent, Text},
};
use std::cell::OnceCell;

/// Borrow only for one existing draw/click handler. A full document preserves
/// cross-page link numbers; a one-page projection would reject valid targets.
pub struct Book<'a> {
    pages: &'a [Text],
    document: OnceCell<Option<Document>>,
}
impl<'a> Book<'a> {
    pub fn new(pages: &'a [Text]) -> Self {
        Self { pages, document: OnceCell::new() }
    }

    pub fn page(&self, font: &Font, index: usize, width: f32) -> Option<Lines> {
        let text = self.pages.get(index)?;
        let document = self.document.get_or_init(|| {
            // Metadata remains opaque and is not executed here. Do not impose
            // a new prefix policy on existing native guide content.
            pluto_guide_text::from_native(self.pages, |_| true).ok()
        });
        if let Some(wrapped) = document.as_ref()
            .and_then(|doc| layout::wrap(font, doc, index, width).ok())
        {
            return Some(Lines::Shared(wrapped));
        }
        // Strict API rejection is not permission to delete content or change
        // native action/style semantics. Preserve the old splitter verbatim.
        Some(Lines::Legacy(font.split(text, width)))
    }
}

pub enum Lines {
    Shared(Wrapped),
    Legacy(Vec<Text>),
}
impl Lines {
    pub fn as_slice(&self) -> &[Text] {
        match self {
            Self::Shared(wrapped) => wrapped.native_lines(),
            Self::Legacy(lines) => lines,
        }
    }

    /// Caller already admitted its own row/geometry. The row is the original
    /// native cast, including its existing nonfinite-coordinate behavior.
    pub fn click(&self, font: &Font, x: f32, row: usize) -> Option<ClickEvent> {
        if let Self::Shared(wrapped) = self
            && x >= 0.0
        {
            if let Ok(hit) = wrapped.hit(x, 0.0, 1.0, row, 1) {
                return hit.and_then(|hit| hit.style.page)
                    .map(|page| ClickEvent::ChangePage(i32::from(page)));
            }
        }
        // This is the old logical traversal, not visual bidi hit testing.
        let line = self.as_slice().get(row)?.resolve_legacy();
        let mut x = x;
        for (character, style) in line.chars() {
            x -= font.advance(character, style);
            if x < 0.0 { return style.click.clone(); }
        }
        None
    }
}
