// Compile this against the installed client-library facade. It does not open
// or refresh any screen, send a packet, or register a locale/event callback.
use horizon_client::pluto_language::{Key, Text, key};

const TITLE: Key<'static> = key("gui.example.guide.title").with_fallback("Field Guide");
const COUNT: Key<'static> = key("gui.example.guide.count").with_fallback("Pages: %s");

fn labels(page_count: usize) -> (Text, Text) {
    (
        TITLE.resolve(&[]),
        COUNT.resolve(&[Text::literal(page_count.to_string())]),
    )
}

fn main() {
    // A consumer calls labels again in its own admitted draw/rebuild path.
    // These values are snapshots; retaining them does not track language changes.
    let (title, count) = labels(3);
    println!("{}: {}", title.plain(), count.plain());
}
