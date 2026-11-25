fn pluto_crafting_special_synced(game: &mut Game<'_>) {
    let Some(synced) = &mut game.recipe_tables.synced else {
        return;
    };
    for id in sunrise_client::pluto_inventory::crafting::special::provider_ids() {
        if !synced.crafting.iter().any(|(name, _)| name.as_str() == id) {
            synced.crafting.push((
                id.to_owned(),
                sunrise_client::crafting::Recipe::Special(
                    sunrise_client::crafting::Special::RepairItem,
                ),
            ));
        }
    }
}
