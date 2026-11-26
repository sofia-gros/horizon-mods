fn pluto_station_ingredient(
    value: &serde_json::Value,
    items: &Items,
) -> std::ops::ControlFlow<Ingredient> {
    crate::stations::pluto_ingredient::parse(value, items)
}
