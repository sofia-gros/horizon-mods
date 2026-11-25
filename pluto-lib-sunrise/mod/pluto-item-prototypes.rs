// Retain default custom data from the EXACT Arc already read by Items::load_with.
// The final items-map LOAD moves into Items; earlier receiver/reference accesses
// must not be selected. No second file read, TLS map, cache key or Items field.
fn pluto_item_prototypes(
    mut value: HashMap<Arc<str>, ItemInfo>,
    raw: &Arc<sunrise_world::item_defaults::Defaults>,
) -> HashMap<Arc<str>, ItemInfo> {
    for (id, item) in &mut value {
        if let Some(data) = raw.get(id.as_ref()).and_then(|components| components.get("minecraft:custom_data")) {
            // Keep even malformed non-compounds: the reload snapshot validator
            // must reject them rather than inventing an empty default compound.
            item.custom_components.insert("minecraft:custom_data".into(),
                horizon_client_core::scoreboard::json_to_tag(data));
        }
    }
    value
}
