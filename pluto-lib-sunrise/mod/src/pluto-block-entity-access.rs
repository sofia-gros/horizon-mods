// Host-independent factory priority. Only explicit custom registrations may
// preempt native name/template classification; vanilla and unregistered blocks
// keep the original factory. Source wiring exists; combined plan, native
// compilation and lifecycle verification remain unverified.
use std::ops::ControlFlow;
use std::sync::OnceLock;
#[cfg(not(test))]
pub use sunrise_client_render::pluto_client_block_entity_state::set_cache_generation_provider;

#[cfg(not(test))]
mod client_ticks {
    use sunrise_world::nbt::Compound;
    use crate::pluto_block::LevelView;
    #[derive(Clone, Copy)]
    pub struct ClientTickSink {
        pub has: fn(&str) -> bool,
        pub tick: fn(&str, [i32; 3], &Compound, &mut Compound, &dyn LevelView) -> bool,
    }
    static SINK: std::sync::OnceLock<ClientTickSink> = std::sync::OnceLock::new();
    pub fn set_client_tick_sink(sink: ClientTickSink) -> Result<(), &'static str> {
        SINK.set(sink).map_err(|_| "client block entity tick sink already installed")
    }
    pub fn client_tick_sink() -> Option<ClientTickSink> { SINK.get().copied() }
}
#[cfg(not(test))]
pub use client_ticks::{ClientTickSink, set_client_tick_sink, client_tick_sink};

pub type TypePredicate = fn(&str) -> Result<bool, String>;
static PREDICATE: OnceLock<TypePredicate> = OnceLock::new();
pub fn set_predicate(predicate: TypePredicate) -> Result<(), String> {
    PREDICATE.set(predicate).map_err(|_| "block entity type predicate already installed".into())
}
pub fn registered(block: &str) -> Result<bool, String> {
    PREDICATE.get().map_or(Ok(false), |predicate| predicate(block))
}

/// Continue means original native classification, with no factory invocation
/// here. Break means a claimed registration or a registry error: NEVER fall back
/// after a failed claimed factory, and never call it a second time.
pub fn route_factory<T>(
    block: &str,
    registered: impl FnOnce(&str) -> Result<bool, String>,
    create: impl FnOnce() -> Option<T>,
) -> ControlFlow<Result<Option<T>, String>> {
    let Some((namespace, path)) = block.split_once(':') else { return ControlFlow::Continue(()) };
    if namespace.is_empty() || namespace == "minecraft" || path.is_empty() || path.contains(':') {
        return ControlFlow::Continue(());
    }
    match registered(block) {
        Ok(false) => ControlFlow::Continue(()),
        Ok(true) => ControlFlow::Break(Ok(create())),
        Err(error) => ControlFlow::Break(Err(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn vanilla_and_unregistered_custom_names_preserve_original_factory() {
        for name in ["minecraft:piglin_head", "minecraft:chest", "missing_namespace", ":head", "example:", "example:bad:id"] {
            assert_eq!(route_factory::<String>(name, |_| panic!("must not consult registry"), || panic!("must not create")), ControlFlow::Continue(()));
        }
        assert_eq!(route_factory::<String>("example:custom_head", |_| Ok(false), || panic!("unregistered")), ControlFlow::Continue(()));
        // Native-recognized names need no per-head special case: an explicitly
        // registered custom sign/head/other name goes through the same route.
        for name in ["example:custom_head", "example:custom_sign", "example:machine"] {
            let calls = Cell::new(0);
            let result = route_factory(name, |_| Ok(true), || { calls.set(calls.get() + 1); Some(String::from("example:owned_type")) });
            assert_eq!(result, ControlFlow::Break(Ok(Some("example:owned_type".into()))));
            assert_eq!(calls.get(), 1);
        }
    }
    #[test]
    fn claimed_failure_and_registry_error_never_retry_or_delegate_to_native_head() {
        let calls = Cell::new(0);
        let result = route_factory::<String>("example:custom_head", |_| Ok(true), || { calls.set(calls.get() + 1); None });
        assert_eq!(result, ControlFlow::Break(Ok(None)));
        assert_eq!(calls.get(), 1);
        let error = route_factory::<String>("example:custom_head", |_| Err("registry unavailable".into()), || panic!("registry failed"));
        assert_eq!(error, ControlFlow::Break(Err("registry unavailable".into())));
    }
}
