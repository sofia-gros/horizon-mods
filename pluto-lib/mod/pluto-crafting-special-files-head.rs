fn pluto_crafting_special_files(files: &mut Vec<(String, Vec<u8>)>) {
    for id in horizon_client::pluto_inventory::crafting::special::provider_ids() {
        files.push((
            format!("recipe/{id}"),
            br#"{"type":"minecraft:crafting_special_repairitem"}"#.to_vec(),
        ));
    }
}
