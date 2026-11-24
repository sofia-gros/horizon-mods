fn pluto_movement_table(value: std::io::Result<crate::movement_properties::MovementProperties<'_>>) -> std::io::Result<crate::movement_properties::MovementProperties<'_>> {
    crate::movement_properties::pluto_movement::apply(value)
}
