extern crate self as horizon_client;
pub mod text {
    #[derive(Clone, Debug, PartialEq)] pub enum ClickEvent { ChangePage(i32), RunCommand(String) }
    #[derive(Clone, Default)] pub struct Style { pub click: Option<ClickEvent>, pub insertion: Option<String>, pub color: Option<u32>, pub bold: Option<bool>, pub italic: Option<bool>, pub underlined: Option<bool>, pub strikethrough: Option<bool>, pub obfuscated: Option<bool> }
    #[derive(Clone)] pub struct Span { pub text: String, pub style: Style }
    #[derive(Clone)] pub struct Text { pub spans: Vec<Span> }
    impl Text { pub fn styled(s: impl Into<String>, style: Style) -> Self { Self { spans: vec![Span { text: s.into(), style }] } } pub fn literal(s: impl Into<String>) -> Self { Self::styled(s, Style::default()) } pub fn empty() -> Self { Self { spans: Vec::new() } } pub fn push(&mut self, other: Text) { self.spans.extend(other.spans); } }
}
#[path = "../../../mixins/tconstruct/source-materials-book-wire.rs"] mod wire;
