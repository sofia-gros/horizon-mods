// TConstruct compatibility adapter: use the existing bounded Pluto queries
// where admissible, otherwise retain the original native read-only mechanics.
// No screen/session handle, action execution, wire codec or persistent cache.
use horizon_client::{
    font::Font,
    pluto_guide_layout::{self as layout, RawLines},
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
            .and_then(|doc| layout::split_raw(font, doc, index, width).ok())
        {
            return Some(Lines::Shared(wrapped));
        }
        // Strict API rejection is not permission to delete content or change
        // native action/style semantics. Preserve the old splitter verbatim.
        Some(Lines::Legacy(font.split(text, width)))
    }
}

pub enum Lines {
    Shared(RawLines),
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
        if let Self::Shared(wrapped) = self {
            return wrapped.hit_native(font, x, row).and_then(|hit| hit.style.click);
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

/// Source-page generation retains its original legacy resolution and row policy.
/// This is not a screen operation or a new wire/document acceptance policy.
/// Resolve once before admission, then query the native splitter exactly once.
pub fn paginate(font: &Font, text: &Text, width: f32, rows: usize) -> Vec<Text> {
    let resolved = text.resolve_legacy();
    let lines = generation_lines(font, &resolved, width);
    lines.as_slice().chunks(rows.max(1)).map(|chunk| {
        let mut page = Text::empty();
        for (i, line) in chunk.iter().enumerate() {
            if i > 0 { page.push("\n"); }
            page.push(line.clone());
        }
        page
    }).collect()
}

fn generation_lines(font: &Font, resolved: &Text, width: f32) -> Lines {
    // This is a temporary one-page projection, never an emitted document.
    // Existing cross-page actions or unsupported content use the old splitter;
    // conversion/admission failure occurs before any glyph/cache query.
    if let Some(lines) = pluto_guide_text::from_native(std::slice::from_ref(resolved), |_| true)
        .ok().and_then(|doc| layout::split_raw(font, &doc, 0, width).ok())
    {
        return Lines::Shared(lines);
    }
    Lines::Legacy(font.split(resolved, width))
}
