// Read-only native font queries. These facts are neither an admitted input nor
// a presentation capability. No Game, renderer, registry, action or transport.
use crate::{
    font::Font,
    pluto_guide_text,
    pluto_inventory::guide::document,
    text::{Span, Style, Text},
};

pub const MAX_GLYPHS: usize = 4096;
pub const MAX_LINES: usize = MAX_GLYPHS + 1;
/// Conservative logical work charge, not an allocator/Font-cache memory limit.
pub const MAX_WORK_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Page,
    Width,
    Point,
    LineHeight,
    Window,
    Budget,
    Metric,
    Style(document::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    /// Absolute index in this wrapped result, not a document page index.
    pub line: usize,
    /// Unicode scalar index after this line's native legacy resolution.
    /// This is logical traversal, not a byte/UTF-16 or visual bidi index.
    pub scalar: usize,
    pub character: char,
    /// A fact only. A page link/metadata does not authorize an action.
    pub style: document::Style,
}

struct Line {
    resolved: Text,
    advances: Vec<f32>,
    width: f32,
}

/// A measured snapshot. It does not follow resource reloads, current screen
/// content or cursor movement. Raw split output and resolved hit facts differ.
pub struct Wrapped {
    raw: Vec<Text>,
    lines: Vec<Line>,
}

impl Wrapped {
    /// Exact Font::split output, retaining its raw legacy codes and styles.
    pub fn native_lines(&self) -> &[Text] {
        &self.raw
    }

    /// Native width_text (legacy resolution and native bidi measurement),
    /// captured at construction. It is not the hit traversal's x extent.
    pub fn width(&self, line: usize) -> Option<f32> {
        self.lines.get(line).map(|line| line.width)
    }

    /// Caller supplies local text coordinates and visible row geometry. Native
    /// book-style hit traversal: resolve each raw line's legacy codes, subtract
    /// its logical characters' advances, choose the first negative remainder.
    /// Exact right edges move to the next character; zero advances are skipped.
    /// No clipping to wrap width or visual bidi/shaping correction is invented.
    pub fn hit(
        &self,
        x: f32,
        y: f32,
        line_height: f32,
        first_line: usize,
        visible_lines: usize,
    ) -> Result<Option<Hit>, Error> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::Point);
        }
        if !line_height.is_finite() || line_height <= 0.0 {
            return Err(Error::LineHeight);
        }
        if visible_lines > MAX_LINES {
            return Err(Error::Window);
        }
        if x < 0.0 || y < 0.0 {
            return Ok(None);
        }
        let row = (y / line_height).floor();
        if !row.is_finite() || row >= visible_lines as f32 {
            return Ok(None);
        }
        let Some(index) = first_line.checked_add(row as usize) else {
            return Err(Error::Window);
        };
        let Some(line) = self.lines.get(index) else {
            return Ok(None);
        };
        let mut remaining = x;
        for (scalar, ((character, style), advance)) in
            line.resolved.chars().zip(&line.advances).enumerate()
        {
            remaining -= advance;
            if !remaining.is_finite() {
                return Err(Error::Metric);
            }
            if remaining < 0.0 {
                return Ok(Some(Hit {
                    line: index,
                    scalar,
                    character,
                    style: pluto_guide_text::from_native_style(style).map_err(Error::Style)?,
                }));
            }
        }
        Ok(None)
    }
}

fn admit(page: &document::Page) -> Result<(), Error> {
    let mut glyphs = 0usize;
    let mut charge = 0usize;
    for span in page {
        let count = span.text.chars().count();
        glyphs = glyphs.checked_add(count).ok_or(Error::Budget)?;
        // Font::split clones a native Style for every raw scalar. Reserve four
        // passes for that, split output, resolved output and measurement, plus
        // the original spans. Charge before native conversion/splitting. Opaque
        // metadata is the only dynamic native style field our codec admits.
        let per = std::mem::size_of::<Style>()
            .checked_add(std::mem::size_of::<Span>())
            .and_then(|n| n.checked_add(4))
            .and_then(|n| n.checked_add(span.style.metadata.as_ref().map_or(0, String::len)))
            .ok_or(Error::Budget)?;
        let next = count
            .checked_add(1)
            .and_then(|n| n.checked_mul(per))
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(span.text.len()))
            .ok_or(Error::Budget)?;
        charge = charge.checked_add(next).ok_or(Error::Budget)?;
        if glyphs > MAX_GLYPHS || charge > MAX_WORK_BYTES {
            return Err(Error::Budget);
        }
    }
    Ok(())
}

fn advances(font: &Font, text: &Text) -> Result<Vec<f32>, Error> {
    let mut magnitude = 0.0f32;
    text.chars()
        .map(|(character, style)| {
            let value = font.advance(character, style);
            magnitude += value.abs();
            if !value.is_finite() || !magnitude.is_finite() {
                Err(Error::Metric)
            } else {
                Ok(value)
            }
        })
        .collect()
}

/// Query one validated document page with the caller's current native Font.
/// Uses exact raw Font::split, not split_formatted/wrap_components. It does not
/// normalize legacy codes before wrapping, matching the audited guide callers.
pub fn wrap(
    font: &Font,
    document: &document::Document,
    page: usize,
    width: f32,
) -> Result<Wrapped, Error> {
    if !width.is_finite() || width <= 0.0 {
        return Err(Error::Width);
    }
    let page = document.pages().get(page).ok_or(Error::Page)?;
    admit(page)?;
    let native = pluto_guide_text::to_native_page(page);
    // Reject nonfinite metrics before the native splitter's comparisons.
    advances(font, &native)?;
    let raw = font.split(&native, width);
    if raw.len() > MAX_LINES {
        return Err(Error::Budget);
    }
    let mut lines = Vec::with_capacity(raw.len());
    let mut output_glyphs = 0usize;
    for text in &raw {
        output_glyphs = output_glyphs
            .checked_add(text.chars().count())
            .ok_or(Error::Budget)?;
        if output_glyphs > MAX_GLYPHS {
            return Err(Error::Budget);
        }
        let width = font.width_text(text);
        if !width.is_finite() {
            return Err(Error::Metric);
        }
        let resolved = text.resolve_legacy();
        let advances = advances(font, &resolved)?;
        lines.push(Line {
            resolved,
            advances,
            width,
        });
    }
    Ok(Wrapped { raw, lines })
}
