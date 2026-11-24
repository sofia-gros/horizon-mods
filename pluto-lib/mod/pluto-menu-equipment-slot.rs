fn pluto_menu_equipment_slot(value: Option<usize>, menu: &str, slot: i32) -> Option<usize> {
    let Some((_, kinds)) = horizon_client_lib::menu::pluto_menu_equipment::equipment_for(menu) else {
        return value;
    };
    let Some(t) = horizon_client::containers::mod_menu_type(menu) else { return value; };
    let n = t.slots;
    let eq = kinds.len();
    let Ok(s) = usize::try_from(slot) else { return value; };
    // The menu's own mapping, with the equipment slots in it: the container slots, then the
    // equipment, then the inventory and the hotbar (the original maps the inventory over the
    // equipment's places, so a click on one never reached a menu slot).
    match s {
        _ if s < n + eq => Some(300 + s),
        _ if s < n + eq + 27 => Some(9 + s - n - eq),
        _ if s < n + eq + 36 => Some(s - n - eq - 27),
        _ => None,
    }
}
