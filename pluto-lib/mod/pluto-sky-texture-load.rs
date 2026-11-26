// A pack's custom sky is loaded: when mods have registered textured sky layers of their own,
// the pack's layers are wrapped with them, so the game's one sky object carries both — the
// pack's textures first (its draw indices do not move), then the mods', drawn after.
//
// The reach is a cancellable HEAD, not a RETURN: the body's own answer for a pack without a
// sky is built inside a closure (unwrap_or_else), and the weaver cannot re-emit a body that
// names a closure, so a return reach on this function can never weave. A HEAD reach leaves
// the body untouched: with no sink, or no layer registered, it continues and the game loads
// the pack's sky itself, exactly as it does unmodded. Only when a layer answers does the
// helper break, returning the same layers the body would have built (asked of the pack
// formats directly) wrapped by the access module's composite. The result type is spelled
// exactly as the host writes it (Arc<dyn SkyLayers>, resolved by the target file's own
// imports): the weaver compares the spelling, and a fully qualified path is refused.
fn pluto_sky_texture_load(root: &Path) -> core::ops::ControlFlow<Arc<dyn SkyLayers>> {
    let Some(sink) = crate::custom_sky::pluto_sky_texture_access::sink() else {
        return core::ops::ControlFlow::Continue(());
    };
    if !(sink.any)() {
        return core::ops::ControlFlow::Continue(());
    }
    let pack = crate::pack_hooks::formats().custom_sky(root);
    core::ops::ControlFlow::Break(crate::custom_sky::pluto_sky_texture_access::wrap(pack, *sink))
}
