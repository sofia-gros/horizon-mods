// Block movement factors for mod blocks (A34 lantern speedFactor).
// The host's block declarations carry no speed-factor key: horizon-mods'
// BlockProperties denies unknown fields, so a "speed_factor" key would be
// rejected at pack load. The factors ride in PlutoLib's movement registry
// instead, and this module applies them when the host binds its movement
// table for use: binding bakes one movement entry per block state from the
// table's definitions (the mod blocks inherit their template's factors
// there), and only the registered blocks' speed and jump factors are
// replaced in the baked entries. Everything else about an entry (friction,
// bounce, fall reduction, fence gate) is left alone, and a registration for
// a block the table does not know is skipped, so a missing declaration
// cannot break the load. This module is a child of movement_properties, so
// it may rewrite the bound table's private states.
pub fn apply_bound(
    value: std::io::Result<crate::movement_properties::BoundMovementProperties>,
    properties: &crate::movement_properties::MovementProperties<'_>,
) -> std::io::Result<crate::movement_properties::BoundMovementProperties> {
    let mut value = value;
    if let Ok(bound) = &mut value {
        let registered: std::collections::BTreeMap<String, pluto_movement_api::MovementFactors> =
            pluto_movement_api::registered().into_iter().collect();
        if !registered.is_empty() {
            let registry = properties.registry();
            for (index, state) in bound.states.iter_mut().enumerate() {
                let name = &registry
                    .state(crate::registry::BlockStateId(index as u32))
                    .unwrap()
                    .name;
                if let Some(factors) = registered.get(name) {
                    state.coefficients.speed_factor = factors.speed_factor;
                    state.coefficients.jump_factor = factors.jump_factor;
                }
            }
        }
    }
    value
}
