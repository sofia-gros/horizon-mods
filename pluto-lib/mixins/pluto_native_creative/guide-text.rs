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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pluto_guide_text_preserves_empty_spans_unicode_and_tristate_style() {
        let mut text = Text::empty();
        text.spans = vec![
            Span {
                text: String::new(),
                style: Style {
                    insertion: Some("example:panel".into()),
                    bold: Some(false),
                    underlined: None,
                    click: Some(ClickEvent::ChangePage(1)),
                    ..Default::default()
                },
            },
            Span {
                text: "日本語😀\0§a".into(),
                style: Style {
                    color: Some(u32::MAX),
                    italic: Some(true),
                    ..Default::default()
                },
            },
        ];
        let document = from_native(&[text.clone()], |s| s.starts_with("example:")).unwrap();
        let wire = api::encode(&document).unwrap();
        let decoded = api::decode(&wire, |s| s.starts_with("example:")).unwrap();
        assert_eq!(to_native(&decoded), vec![text]);
    }
    #[test]
    fn pluto_guide_text_rejects_actions_and_unsupported_styles_without_loss() {
        for click in [
            ClickEvent::RunCommand("/say x".into()),
            ClickEvent::OpenUrl("https://example.com".into()),
            ClickEvent::CopyToClipboard("x".into()),
        ] {
            let page = Text::styled(
                "x",
                Style {
                    click: Some(click),
                    ..Default::default()
                },
            );
            assert_eq!(
                from_native(&[page], |_| false),
                Err(api::Error::UnsupportedStyle)
            );
        }
        for style in [
            Style {
                font: Some("minecraft:default".into()),
                ..Default::default()
            },
            Style {
                shadow_color: Some(0),
                ..Default::default()
            },
            Style {
                hover: Some(Box::new(Text::literal("hover"))),
                ..Default::default()
            },
        ] {
            assert_eq!(
                from_native(&[Text::styled("x", style)], |_| false),
                Err(api::Error::UnsupportedStyle)
            );
        }
    }
    #[test]
    fn pluto_guide_text_metadata_admission_and_page_links_are_explicit() {
        let page = Text::styled(
            "x",
            Style {
                insertion: Some("other:data".into()),
                ..Default::default()
            },
        );
        assert_eq!(from_native(&[page], |_| false), Err(api::Error::Metadata));
        for target in [-1, 0, 2] {
            let page = Text::styled(
                "x",
                Style {
                    click: Some(ClickEvent::ChangePage(target)),
                    ..Default::default()
                },
            );
            assert_eq!(from_native(&[page], |_| true), Err(api::Error::Link));
        }
    }
}
