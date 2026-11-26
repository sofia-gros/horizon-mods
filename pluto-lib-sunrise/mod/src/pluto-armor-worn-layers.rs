// The worn stack's own layers (a registered renderer), replacing the equipment asset's list for that piece only.
fn pluto_armor_worn_layers(game:&Game<'_>,stack:Option<&ItemStack>,kind:&str)->Option<Vec<(String,Option<u32>)>> {
    sunrise_client_lib::pluto_armor_layers::layers(&game.pack.item_renderer,stack?,kind)
}
