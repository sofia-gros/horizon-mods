//! Standalone startup/content example. No Game, packet sending or UI mutation.
use pluto_inventory_api::{
    client_payload::{self, Payload, RegistrationError},
    guide::{Document, Span, Style, document},
};

// An ordinary consumer calls this once from its own explicit startup path.
// There is no automatic init/ready event or new native hook in this example.
fn init() -> Result<(), RegistrationError> {
    client_payload::OBSERVED
        .channel("example:guide_document", 4096)
        .register("example:guide_logger", log_document)
}

fn log_document(payload: Payload<'_>) {
    // This example chooses a Pluto document body for its own channel. Bytes
    // remain untrusted; receipt grants no reply, identity or screen authority.
    let Ok(document) = document::decode(payload.bytes, |_| false) else { return };
    println!("Observed {} document pages", document.pages().len());
}

fn content() -> Result<Document, document::Error> {
    Document::new(vec![
        vec![Span::plain("Example guide"), Span::styled("Next page", Style {
            page: Some(2), ..Style::default()
        })],
        vec![Span::plain("Any mod can supply its own content")],
    ], |_| false)
}

fn main() {
    init().expect("unique valid startup subscription");
    let document = content().expect("bounded example document");
    assert_eq!(document.pages().len(), 2);
    // Cleanup is explicit. A channel settings value is not an RAII handle.
    assert!(client_payload::OBSERVED.remove("example:guide_logger").unwrap());
}

#[test]
fn public_startup_and_content_example() {
    main();
}
