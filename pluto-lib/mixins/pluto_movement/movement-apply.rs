// Block movement factors for mod blocks (A34 lantern speedFactor).
// The host's block declarations carry no speed-factor key: horizon-mods'
// BlockProperties denies unknown fields, so a "speed_factor" key would be
// rejected at pack load. The factors ride in PlutoLib's movement registry
// instead, and this module applies them to the movement table the host loads
// from its version data: the table already holds one entry per block (the mod
// blocks inherit their template's factors there), and only the registered
// blocks' speed and jump factors are replaced. Everything else about an entry
// (friction, bounce, fall reduction, fence gate) is left alone, and a
// registration for a block the table does not know is skipped, so a missing
// declaration cannot break the load. This module is a child of
// movement_properties, so it may read that module's private definitions map.
pub fn apply(value: std::io::Result<crate::movement_properties::MovementProperties<'_>>) -> std::io::Result<crate::movement_properties::MovementProperties<'_>> {
    let mut value = value;
    if let Ok(properties) = &mut value {
        for (block, factors) in pluto_movement_api::registered() {
            if let Some(coefficients) = properties.definitions.get_mut(&block) {
                coefficients.speed_factor = factors.speed_factor;
                coefficients.jump_factor = factors.jump_factor;
            }
        }
    }
    value
}
