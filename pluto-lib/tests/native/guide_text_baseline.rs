// Frozen converter oracle from horizon-mods 38b07177, before helper extraction.
// Only provenance comments, test-module omission and final blank-line trim differ.
// Restricted semantic native Text conversion. Not a full text-component codec,
// screen opener, packet handler or permission to interpret consumer metadata.
use crate::{
    pluto_inventory::guide::document as api,
    text::{ClickEvent, Span, Style, Text},
};

pub fn from_native(
    pages: &[Text],
    admit: impl FnMut(&str) -> bool,
) -> Result<api::Document, api::Error> {
    if pages.is_empty() || pages.len() > api::MAX_PAGES {
        return Err(api::Error::Pages);
    }
    let mut raw = 0usize;
    let mut result = Vec::with_capacity(pages.len());
    for page in pages {
        if page.spans.len() > api::MAX_SPANS {
            return Err(api::Error::Spans);
        }
        let mut spans = Vec::with_capacity(page.spans.len());
        for span in &page.spans {
            let style = &span.style;
            if style.shadow_color.is_some()
                || style.hover.is_some()
                || style.hover_item.is_some()
                || style.hover_entity.is_some()
                || style.font.is_some()
                || style.inline_object.is_some()
            {
                return Err(api::Error::UnsupportedStyle);
            }
            let link = match &style.click {
                None => None,
                Some(ClickEvent::ChangePage(n)) => {
                    Some(u16::try_from(*n).map_err(|_| api::Error::Link)?)
                }
                _ => return Err(api::Error::UnsupportedStyle),
            };
            if span.text.len() > api::MAX_TEXT_BYTES {
                return Err(api::Error::Text);
            }
            if style
                .insertion
                .as_ref()
                .is_some_and(|s| s.len() > api::MAX_METADATA_BYTES)
            {
                return Err(api::Error::Metadata);
            }
            raw = raw.checked_add(span.text.len()).ok_or(api::Error::Bytes)?;
            raw = raw
                .checked_add(style.insertion.as_ref().map_or(0, String::len))
                .ok_or(api::Error::Bytes)?;
            if raw > api::MAX_BYTES {
                return Err(api::Error::Bytes);
            }
            spans.push(api::Span {
                text: span.text.clone(),
                style: api::Style {
                    color: style.color,
                    bold: style.bold,
                    italic: style.italic,
                    underlined: style.underlined,
                    strikethrough: style.strikethrough,
                    obfuscated: style.obfuscated,
                    page: link,
                    metadata: style.insertion.clone(),
                },
            });
        }
        result.push(spans);
    }
    api::Document::new(result, admit)
}

pub fn to_native(document: &api::Document) -> Vec<Text> {
    document
        .pages()
        .iter()
        .map(|page| {
            // Text::literal/styled discard empty strings; direct semantic spans keep
            // an empty span's style/metadata. The native source-JSON cache stays None.
            let mut text = Text::empty();
            text.spans = page
                .iter()
                .map(|span| {
                    let style = &span.style;
                    Span {
                        text: span.text.clone(),
                        style: Style {
                            color: style.color,
                            bold: style.bold,
                            italic: style.italic,
                            underlined: style.underlined,
                            strikethrough: style.strikethrough,
                            obfuscated: style.obfuscated,
                            click: style.page.map(|n| ClickEvent::ChangePage(i32::from(n))),
                            insertion: style.metadata.clone(),
                            ..Default::default()
                        },
                    }
                })
                .collect();
            text
        })
        .collect()
}
