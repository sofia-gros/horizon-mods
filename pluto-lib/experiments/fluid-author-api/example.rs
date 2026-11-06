//! Consumer-side example for the existing PlutoLib fluid API, not an installed mod.
//! Signatures match the verified 8715b123 source; this example has not been compiled.
//! Call initialization from the consumer's approved startup wiring, before resources.
use pluto_api::fluid::{self, contact, Definition, Position, Sample};

pub const RAINWATER: &str = "meadow:rainwater";

/// Registration is atomic for this batch, not for installing assets or other services.
pub fn initialize_fluids() -> Result<(), String> {
    fluid::register_all([Definition {
        id: RAINWATER.into(),
        block: "meadow:rainwater_block".into(),
        bucket: "meadow:rainwater_bucket".into(),
        texture: "meadow:block/rainwater_still".into(),
        tick_delay: 5,
        drop_off: 1,
        light: 0,
    }])
}

/// A read query, not a registered event. The caller supplies a valid read scope
/// and exact adjusted bounds. Never translate unavailable cells into Air.
pub fn read_contacts(
    bounds: contact::Bounds,
    sample: impl FnMut(Position) -> Sample,
) -> Result<contact::Contacts, contact::QueryError> {
    contact::query_box_with(bounds, contact::Limits { max_cells: 4096 }, sample)
}

/// Consumer policy over returned facts. Unknown stays unknown, not false.
/// This does not apply an effect, classify a native actor, or authorize mutation.
pub fn completely_known_rainwater_contact(contacts: &contact::Contacts) -> Option<bool> {
    if !contacts.is_complete() {
        return None;
    }
    Some(contacts.fluids.contains_key(RAINWATER))
}
