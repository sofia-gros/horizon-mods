// Proposed private compatibility bridge, not an opening/actor authority API.
// Install only after full plan/order/source/type and consumer oracle gates.
use super::{Game, books::BookScreen};
use horizon_client::{pluto_inventory::client_payload::Payload, text::Text};
use horizon_protocol::packets::play::Clientbound;
use std::sync::RwLock;

pub(super) type Context = for<'g> fn(&Game<'g>) -> u64;
pub(super) type Update = for<'v, 'p> fn(&mut RetainedView<'v>, u64, Payload<'p>);

// Only a static descriptor and function pointers are retained. No Game/packet
// reference, consumer document, pending request or sequence is stored here.
pub(super) struct Handler {
    pub(super) id: &'static str,
    pub(super) channel: &'static str,
    pub(super) max_bytes: usize,
    pub(super) context: Context,
    pub(super) update: Update,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RegistrationError { InvalidKey, Conflict, Full, Poisoned }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DispatchError { Poisoned }
struct Registry(RwLock<Vec<&'static Handler>>);
impl Registry {
    const fn new() -> Self { Self(RwLock::new(Vec::new())) }
    fn register(&self, handler: &'static Handler) -> Result<(), RegistrationError> {
        // Trusted same-crate descriptor; keys are bounded selectors, not proof
        // that the caller owns the channel or a retained book.
        let valid = |s: &str| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control);
        if !valid(handler.id) || !valid(handler.channel) { return Err(RegistrationError::InvalidKey); }
        let mut entries = self.0.write().map_err(|_| RegistrationError::Poisoned)?;
        if entries.iter().any(|old| std::ptr::eq(*old, handler)) { return Ok(()); }
        if entries.iter().any(|old| old.id == handler.id || old.channel == handler.channel) {
            return Err(RegistrationError::Conflict);
        }
        if entries.len() == 64 { return Err(RegistrationError::Full); }
        entries.push(handler);
        Ok(())
    }
    fn snapshot<'p>(&self, packet: &'p Clientbound) -> Result<Option<Snapshot<'p>>, DispatchError> {
        let Clientbound::CustomPayload(message) = packet else { return Ok(None); };
        let selected = self.0.read().map_err(|_| DispatchError::Poisoned)?
            .iter().find(|h| h.channel == message.channel && message.data.len() <= h.max_bytes).copied();
        // The lock is gone before the native/read-only observer can call out.
        Ok(selected.map(|handler| Snapshot { handler, payload: Payload { channel: &message.channel, bytes: &message.data } }))
    }
}
static REGISTRY: Registry = Registry::new();
pub(super) fn register(handler: &'static Handler) -> Result<(), RegistrationError> { REGISTRY.register(handler) }
pub(super) fn snapshot(packet: &Clientbound) -> Result<Option<Snapshot<'_>>, DispatchError> { REGISTRY.snapshot(packet) }

// Non-Clone, borrowed call-local selection. Registering while observers execute
// affects only a later snapshot. One exact channel has at most one updater.
pub(super) struct Snapshot<'p> { handler: &'static Handler, payload: Payload<'p> }
impl Snapshot<'_> {
    pub(super) fn dispatch(self, game: &mut Game<'_>) {
        // Run AFTER the native and read-only observers. The context callback is
        // read-only; the current View is borrowed only after it returns. It can
        // supply consumer state, but cannot install or mutate an opening here.
        let context = (self.handler.context)(game);
        let Some(BookScreen::View { pages, page, lectern: None }) = game.book.as_mut() else { return; };
        let mut view = RetainedView { pages, page, attempted: false };
        (self.handler.update)(&mut view, context, self.payload);
    }
}

pub(super) struct RetainedView<'a> { pages: &'a mut Vec<Text>, page: &'a mut usize, attempted: bool }
pub(super) enum ReplaceError { Empty(Vec<Text>), AlreadyAttempted(Vec<Text>) }
impl RetainedView<'_> {
    pub(super) fn pages(&self) -> &[Text] { self.pages }
    // This is a synchronous current-value update, not durable ownership. No
    // input-eligibility gate: the old response also updates beneath overlays.
    pub(super) fn replace_pages(&mut self, candidate: Vec<Text>) -> Result<(), ReplaceError> {
        if self.attempted { return Err(ReplaceError::AlreadyAttempted(candidate)); }
        if candidate.is_empty() { return Err(ReplaceError::Empty(candidate)); }
        self.attempted = true;
        if *self.page != usize::MAX { *self.page = (*self.page).min(candidate.len() - 1); }
        // Preserve the old statement order and concrete Vec<Text> assignment.
        // Panic after this attempt is not rollback or permission to retry.
        *self.pages = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context(_: &Game<'_>) -> u64 { 0 }
    fn update(_: &mut RetainedView<'_>, _: u64, _: Payload<'_>) {}
    static FIRST: Handler = Handler { id: "demo:first", channel: "demo:pages", max_bytes: 262_152, context, update };
    static SAME_ID: Handler = Handler { id: "demo:first", channel: "demo:other", max_bytes: 262_152, context, update };
    static SAME_CHANNEL: Handler = Handler { id: "demo:second", channel: "demo:pages", max_bytes: 262_152, context, update };
    static OTHER: Handler = Handler { id: "demo:other", channel: "demo:other", max_bytes: 0, context, update };
    fn packet(channel: &str, size: usize) -> Clientbound {
        Clientbound::CustomPayload(horizon_protocol::packets::configuration::CustomPayload {
            channel: channel.into(), data: vec![0; size],
        })
    }
    #[test]
    fn descriptor_idempotence_and_conflicts() {
        let registry = Registry::new();
        assert_eq!(registry.register(&FIRST), Ok(()));
        assert_eq!(registry.register(&FIRST), Ok(()));
        assert_eq!(registry.register(&SAME_ID), Err(RegistrationError::Conflict));
        assert_eq!(registry.register(&SAME_CHANNEL), Err(RegistrationError::Conflict));
    }
    #[test]
    fn exact_channel_and_whole_envelope_limit() {
        let registry = Registry::new();
        registry.register(&FIRST).unwrap();
        let at_limit = packet("demo:pages", 262_152);
        assert_eq!(registry.snapshot(&at_limit).unwrap().unwrap().payload.bytes.len(), 262_152);
        assert!(registry.snapshot(&packet("demo:pages", 262_153)).unwrap().is_none());
        assert!(registry.snapshot(&packet("demo:unknown", 0)).unwrap().is_none());
    }
    #[test]
    fn snapshot_does_not_hold_registry_lock() {
        let registry = Registry::new();
        registry.register(&FIRST).unwrap();
        let incoming = packet("demo:pages", 1);
        let selected = registry.snapshot(&incoming).unwrap().unwrap();
        registry.register(&OTHER).unwrap();
        assert!(std::ptr::eq(selected.handler, &FIRST));
    }
    #[test]
    fn native_page_replacement_preserves_cover_and_clamps() {
        for initial in [usize::MAX, 0, 1, 20] {
            let mut pages = vec![Text::literal("old")];
            let mut page = initial;
            let mut view = RetainedView { pages: &mut pages, page: &mut page, attempted: false };
            assert!(view.replace_pages(vec![Text::literal("new"), Text::literal("last")]).is_ok());
            assert_eq!(page, if initial == usize::MAX { usize::MAX } else { initial.min(1) });
            assert_eq!(pages[0].plain(), "new");
        }
    }
    #[test]
    fn invalid_or_second_candidate_is_returned_without_another_update() {
        let mut pages = vec![Text::literal("old")];
        let mut page = 10;
        let mut view = RetainedView { pages: &mut pages, page: &mut page, attempted: false };
        assert!(matches!(view.replace_pages(vec![]), Err(ReplaceError::Empty(value)) if value.is_empty()));
        assert!(!view.attempted);
        assert!(view.replace_pages(vec![Text::literal("new")]).is_ok());
        assert!(matches!(view.replace_pages(vec![Text::literal("discarded")]), Err(ReplaceError::AlreadyAttempted(value)) if value[0].plain() == "discarded"));
        assert_eq!(view.pages()[0].plain(), "new");
    }
    #[test]
    fn later_same_payload_gets_a_new_snapshot() {
        let registry = Registry::new();
        registry.register(&FIRST).unwrap();
        let incoming = packet("demo:pages", 8);
        assert!(registry.snapshot(&incoming).unwrap().is_some());
        assert!(registry.snapshot(&incoming).unwrap().is_some());
    }
}
