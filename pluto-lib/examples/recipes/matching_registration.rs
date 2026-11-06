// Actual existing API. Calling this function installs a process-wide provider;
// merely compiling this file does not register it. Source-only example.
use pluto_inventory_api::crafting::matching::{self, Decision, Registration, RegistrationError};

/// Example mod policy: deny native matching for fixed results with components.
/// This intentionally broad example is NOT a recommendation for TConstruct's
/// material recipes and is not sufficient to block automatic book placement.
pub fn register_example_policy() -> Result<(), RegistrationError> {
    matching::register(Registration {
        id: "example:component_result_policy",
        interest: |recipe| recipe.template.is_some_and(|template| template.has_components),
        callback: |_request| Decision::Deny,
    })
}

// Registration belongs to the consumer's supported startup path, once before
// the first relevant event. Duplicate IDs return Err, never replace silently.
fn main() {}
