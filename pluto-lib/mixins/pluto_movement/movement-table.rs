fn pluto_movement_table(value: io::Result<BoundMovementProperties>, properties: &MovementProperties<'_>) -> io::Result<BoundMovementProperties> {
    crate::movement_properties::pluto_movement::apply_bound(value, properties)
}
