// Candidate support module in game::books. All legacy policy stays here.
use super::Game;
use crate::game::pluto_client_payload_updates::{Handler, RetainedView, register};
use horizon_client::pluto_inventory::client_payload::Payload;
use std::sync::{OnceLock, atomic::{AtomicBool, Ordering}};

static REGISTERED: OnceLock<Result<(), crate::game::pluto_client_payload_updates::RegistrationError>> = OnceLock::new();
static FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);

static RESPONSE: Handler = Handler {
    id: "tconstruct:materials_book_reply",
    channel: "tconstruct:materials_book",
    // Eight-byte sequence prefix plus the unchanged legacy body maximum.
    max_bytes: 262_152,
    context: current_sequence,
    update: update,
};
fn current_sequence(game: &Game<'_>) -> u64 {
    *game.world.chunks.__horizon_mixins_tconstruct_materials_book_request_sequence()
}
fn update(view: &mut RetainedView<'_>, sequence: u64, payload: Payload<'_>) {
    let Some(decoded) = super::tconstruct_materials_book_wire::decode_reply(sequence, view.pages(), payload.bytes) else { return; };
    // Decoder requires nonempty pages; this handler attempts exactly one write.
    if view.replace_pages(decoded).is_err() {
        log::error!("TConstruct: guide reply rejected by retained-view update contract");
    }
}
pub(super) fn ensure_registered(game: &mut Game<'_>) -> bool {
    // No unregister exists: success/conflict/invalid/full/poisoned is stable for
    // this process. No callback or notification runs inside get_or_init.
    match *REGISTERED.get_or_init(|| register(&RESPONSE)) {
        Ok(()) => true,
        Err(error) => {
            // Do not open/send a request that can never be handled silently.
            // Native message method/type still requires the actual host gate.
            // Claim before calling native chat, so repeated/reentrant packets
            // cannot turn a permanent conflict into unbounded notifications.
            if !FAILURE_REPORTED.swap(true, Ordering::Relaxed) {
                log::error!("TConstruct: guide reply registration failed: {error:?}");
                game.system_message(horizon_client::text::Text::literal(
                    "TConstruct guide replies are unavailable: handler registration failed. Check the mod compatibility log.",
                ));
            }
            false
        }
    }
}
