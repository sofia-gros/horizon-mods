// Transport for mod-resolved advancement function rewards: the submitting
// library installs a resolver, and the reward-function hook grants what it
// returns (see pluto_advancement_function_reward). No actor or progress
// state lives here.
use std::sync::OnceLock;

static FUNCTION_RESOLVER: OnceLock<fn(&str) -> Vec<String>> = OnceLock::new();

/// Install the process-lifetime resolver of advancement function rewards.
/// The resolver answers a function id with the advancement ids the
/// function grants the rewarded player; an empty answer (or no resolver)
/// leaves the function to the game's data pack store. Installing this
/// native package alone resolves nothing.
pub fn set_function_resolver(resolver: fn(&str) -> Vec<String>) -> Result<(), String> {
    FUNCTION_RESOLVER
        .set(resolver)
        .map_err(|_| "advancement function resolver already installed".into())
}

/// The advancements the installed resolver grants for a function id.
pub(crate) fn resolve(id: &str) -> Vec<String> {
    FUNCTION_RESOLVER.get().map_or_else(Vec::new, |resolver| resolver(id))
}
