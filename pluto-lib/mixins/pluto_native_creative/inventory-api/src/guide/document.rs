//! A restricted semantic text format, not a full native component codec.
//! Commands, URLs, dialogs, files, clipboard actions, hover and fonts are absent.
use serde_json::{Value, json};

pub const MAX_BYTES: usize = 262_144;
pub const MAX_PAGES: usize = 512;
pub const MAX_SPANS: usize = 32;
pub const MAX_TEXT_BYTES: usize = 4096;
pub const MAX_METADATA_BYTES: usize = 8192;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub color: Option<u32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underlined: Option<bool>,
    pub strikethrough: Option<bool>,
    pub obfuscated: Option<bool>,
    /// One-based page link only, validated against this document's page count.
    pub page: Option<u16>,
    /// Opaque consumer-admitted metadata. No namespace or meaning is assumed.
    pub metadata: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: Style,
}
impl Span {
    /// Plain text with inherited/default styling. This does not validate limits;
    /// the containing Document validates content and metadata when constructed.
    pub fn plain(text: impl Into<String>) -> Self {
        Self::styled(text, Style::default())
    }

    /// Text and explicit semantic style. No action is executed or admitted here.
    /// Link bounds and metadata policy remain the containing Document's checks.
    pub fn styled(text: impl Into<String>, style: Style) -> Self {
        Self { text: text.into(), style }
    }
}
pub type Page = Vec<Span>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pages: Vec<Page>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Bytes,
    Pages,
    Spans,
    Text,
    Metadata,
    Link,
    Wire,
    UnsupportedStyle,
}
impl Document {
    /// Metadata admission is explicit consumer policy. Rejecting metadata rejects
    /// the whole document rather than silently changing it. Empty spans survive.
    pub fn new(pages: Vec<Page>, mut admit: impl FnMut(&str) -> bool) -> Result<Self, Error> {
        if pages.is_empty() || pages.len() > MAX_PAGES {
            return Err(Error::Pages);
        }
        let mut raw = 0usize;
        for page in &pages {
            if page.len() > MAX_SPANS {
                return Err(Error::Spans);
            }
            for span in page {
                if span.text.len() > MAX_TEXT_BYTES {
                    return Err(Error::Text);
                }
                raw = raw.checked_add(span.text.len()).ok_or(Error::Bytes)?;
                if let Some(metadata) = &span.style.metadata {
                    if metadata.len() > MAX_METADATA_BYTES || !admit(metadata) {
                        return Err(Error::Metadata);
                    }
                    raw = raw.checked_add(metadata.len()).ok_or(Error::Bytes)?;
                }
                if raw > MAX_BYTES {
                    return Err(Error::Bytes);
                }
                if span
                    .style
                    .page
                    .is_some_and(|n| n == 0 || usize::from(n) > pages.len())
                {
                    return Err(Error::Link);
                }
            }
        }
        let document = Self { pages };
        // Includes JSON structure/escaping overhead, not just raw string bytes.
        encode(&document)?;
        Ok(document)
    }
    pub fn pages(&self) -> &[Page] {
        &self.pages
    }
}

/// Versioned Pluto document body. It is deliberately not a Tinkers legacy reply
/// envelope and contains no owner/session, title identity or action permissions.
pub fn encode(document: &Document) -> Result<Vec<u8>, Error> {
    let pages: Vec<Vec<Value>> = document
        .pages
        .iter()
        .map(|page| {
            page.iter()
                .map(|span| {
                    let style = &span.style;
                    json!({"text":span.text,"color":style.color,"bold":style.bold,
            "italic":style.italic,"underlined":style.underlined,
            "strikethrough":style.strikethrough,"obfuscated":style.obfuscated,
            "page":style.page,"metadata":style.metadata})
                })
                .collect()
        })
        .collect();
    let bytes = serde_json::to_vec(&json!({"version":1,"pages":pages})).map_err(|_| Error::Wire)?;
    if bytes.len() > MAX_BYTES {
        return Err(Error::Bytes);
    }
    Ok(bytes)
}
fn optional_bool(value: Option<&Value>) -> Result<Option<bool>, Error> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(v)) => Ok(Some(*v)),
        _ => Err(Error::Wire),
    }
}
fn optional_u32(value: Option<&Value>) -> Result<Option<u32>, Error> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .and_then(|n| n.try_into().ok())
            .map(Some)
            .ok_or(Error::Wire),
    }
}
pub fn decode(bytes: &[u8], admit: impl FnMut(&str) -> bool) -> Result<Document, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::Bytes);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| Error::Wire)?;
    let root = value.as_object().ok_or(Error::Wire)?;
    if root
        .keys()
        .any(|key| !matches!(key.as_str(), "version" | "pages"))
        || root.get("version").and_then(Value::as_u64) != Some(1)
    {
        return Err(Error::Wire);
    }
    let wire = root
        .get("pages")
        .and_then(Value::as_array)
        .ok_or(Error::Wire)?;
    if wire.is_empty() || wire.len() > MAX_PAGES {
        return Err(Error::Pages);
    }
    let mut pages = Vec::with_capacity(wire.len());
    for page in wire {
        let spans = page.as_array().ok_or(Error::Wire)?;
        if spans.len() > MAX_SPANS {
            return Err(Error::Spans);
        }
        let mut result = Vec::with_capacity(spans.len());
        for span in spans {
            let object = span.as_object().ok_or(Error::Wire)?;
            if object.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "text"
                        | "color"
                        | "bold"
                        | "italic"
                        | "underlined"
                        | "strikethrough"
                        | "obfuscated"
                        | "page"
                        | "metadata"
                )
            }) {
                return Err(Error::Wire);
            }
            let text = object
                .get("text")
                .and_then(Value::as_str)
                .ok_or(Error::Wire)?;
            if text.len() > MAX_TEXT_BYTES {
                return Err(Error::Text);
            }
            let metadata = match object.get("metadata") {
                None | Some(Value::Null) => None,
                Some(Value::String(v)) if v.len() <= MAX_METADATA_BYTES => Some(v.clone()),
                _ => return Err(Error::Metadata),
            };
            let page = optional_u32(object.get("page"))?
                .map(u16::try_from)
                .transpose()
                .map_err(|_| Error::Link)?;
            result.push(Span {
                text: text.to_owned(),
                style: Style {
                    color: optional_u32(object.get("color"))?,
                    bold: optional_bool(object.get("bold"))?,
                    italic: optional_bool(object.get("italic"))?,
                    underlined: optional_bool(object.get("underlined"))?,
                    strikethrough: optional_bool(object.get("strikethrough"))?,
                    obfuscated: optional_bool(object.get("obfuscated"))?,
                    page,
                    metadata,
                },
            });
        }
        pages.push(result);
    }
    Document::new(pages, admit)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn span(text: &str) -> Span {
        Span {
            text: text.into(),
            style: Style::default(),
        }
    }
    fn admitted(_: &str) -> bool {
        true
    }
    #[test]
    fn guide_document_round_trips_unicode_empty_spans_and_inherited_flags() {
        let mut empty = span("");
        empty.style = Style {
            color: Some(u32::MAX),
            bold: Some(false),
            italic: Some(true),
            underlined: None,
            page: Some(1),
            metadata: Some("example:panel".into()),
            ..Default::default()
        };
        let document = Document::new(vec![vec![empty, span("日本語😀\0§a")]], admitted).unwrap();
        assert_eq!(
            decode(&encode(&document).unwrap(), admitted).unwrap(),
            document
        );
        assert_eq!(document.pages()[0][0].style.underlined, None);
    }
    #[test]
    fn guide_document_rejects_actions_unknown_fields_and_bad_schema() {
        for extra in [
            "click",
            "url",
            "command",
            "hover",
            "font",
            "insertion",
            "shadow_color",
        ] {
            let mut value = json!({"version":1,"pages":[[{"text":"x"}]]});
            value["pages"][0][0][extra] = Value::String("payload".into());
            assert_eq!(
                decode(&serde_json::to_vec(&value).unwrap(), admitted),
                Err(Error::Wire)
            );
        }
        for bytes in [
            br#"{"version":2,"pages":[[]]}"#.as_slice(),
            br#"{"version":1,"pages":[[{"text":"x","bold":1}]]}"#,
            br#"{"version":1,"pages":[[{"text":"\ud800"}]]}"#,
        ] {
            assert_eq!(decode(bytes, admitted), Err(Error::Wire));
        }
    }
    #[test]
    fn guide_document_metadata_is_bounded_and_admitted_by_consumer() {
        let mut value = span("x");
        value.style.metadata = Some("foreign:data".into());
        assert_eq!(
            Document::new(vec![vec![value.clone()]], |_| false),
            Err(Error::Metadata)
        );
        value.style.metadata = Some("x".repeat(MAX_METADATA_BYTES));
        assert!(Document::new(vec![vec![value.clone()]], admitted).is_ok());
        value.style.metadata.as_mut().unwrap().push('x');
        assert_eq!(
            Document::new(vec![vec![value]], admitted),
            Err(Error::Metadata)
        );
    }
    #[test]
    fn guide_document_enforces_page_span_text_and_link_bounds() {
        assert_eq!(Document::new(vec![], admitted), Err(Error::Pages));
        assert!(Document::new(vec![vec![]; MAX_PAGES], admitted).is_ok());
        assert_eq!(
            Document::new(vec![vec![]; MAX_PAGES + 1], admitted),
            Err(Error::Pages)
        );
        assert!(Document::new(vec![vec![span(""); MAX_SPANS]], admitted).is_ok());
        assert_eq!(
            Document::new(vec![vec![span(""); MAX_SPANS + 1]], admitted),
            Err(Error::Spans)
        );
        assert!(Document::new(vec![vec![span(&"x".repeat(MAX_TEXT_BYTES))]], admitted).is_ok());
        assert_eq!(
            Document::new(vec![vec![span(&"x".repeat(MAX_TEXT_BYTES + 1))]], admitted),
            Err(Error::Text)
        );
        for target in [0, 2, u16::MAX] {
            let mut value = span("x");
            value.style.page = Some(target);
            assert_eq!(Document::new(vec![vec![value]], admitted), Err(Error::Link));
        }
    }
    #[test]
    fn guide_document_enforces_actual_encoded_bytes_including_escapes() {
        let mut pages = vec![vec![span(&"x".repeat(MAX_TEXT_BYTES))]; 55];
        let base = Document::new(pages.clone(), admitted).unwrap();
        assert!(encode(&base).unwrap().len() < MAX_BYTES);
        // A bounded raw string expands sixfold on the wire; don't count raw only.
        for page in &mut pages {
            page[0].text = "\0".repeat(MAX_TEXT_BYTES);
        }
        assert_eq!(Document::new(pages, admitted), Err(Error::Bytes));
        assert_eq!(
            decode(&vec![b' '; MAX_BYTES + 1], admitted),
            Err(Error::Bytes)
        );
    }
    #[test]
    fn guide_document_decode_rejects_out_of_range_and_wrong_type_links() {
        for page in [
            json!(-1),
            json!(0),
            json!(2),
            json!(true),
            json!("1"),
            json!(65536),
        ] {
            let wire =
                serde_json::to_vec(&json!({"version":1,"pages":[[{"text":"x","page":page}]]}))
                    .unwrap();
            assert!(decode(&wire, admitted).is_err());
        }
        let wire = serde_json::to_vec(
            &json!({"version":1,"pages":[[{"text":"x","metadata":"foreign:data"}]]}),
        )
        .unwrap();
        assert_eq!(decode(&wire, |_| false), Err(Error::Metadata));
    }
    #[test]
    fn guide_document_accepts_exact_wire_limit_and_rejects_next_byte() {
        let mut pages = Vec::new();
        loop {
            pages.push(vec![span(&"x".repeat(4000))]);
            if Document::new(pages.clone(), admitted).is_err() {
                pages.pop();
                break;
            }
        }
        pages.push(vec![span("")]);
        if Document::new(pages.clone(), admitted).is_err() {
            pages.pop();
            pages.last_mut().unwrap()[0].text.clear();
        }
        let (mut low, mut high) = (0, MAX_TEXT_BYTES);
        while low < high {
            let middle = (low + high + 1) / 2;
            pages.last_mut().unwrap()[0].text = "x".repeat(middle);
            if Document::new(pages.clone(), admitted).is_ok() {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        pages.last_mut().unwrap()[0].text = "x".repeat(low);
        let document = Document::new(pages.clone(), admitted).unwrap();
        assert_eq!(encode(&document).unwrap().len(), MAX_BYTES);
        assert!(decode(&encode(&document).unwrap(), admitted).is_ok());
        pages.last_mut().unwrap()[0].text.push('x');
        assert_eq!(Document::new(pages, admitted), Err(Error::Bytes));
    }
}
