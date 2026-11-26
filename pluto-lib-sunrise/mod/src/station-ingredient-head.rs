fn pluto_station_ingredient(
    value: &serde_json::Value,
    items: &Items,
) -> std::ops::ControlFlow<Ingredient> {
    sunrise_client_lib::pluto_ingredient::parse(value, items)
}
