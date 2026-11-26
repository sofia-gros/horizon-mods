fn pluto_menu_equipment_slot(menu: &str, slot: i32) -> core::ops::ControlFlow<Option<usize>> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some((_, kinds)) = crate::pluto_menu_equipment::equipment_for(menu) else {
        return Continue(());
    };
    let Some(t) = sunrise_client_render::containers::mod_menu_type(menu) else { return Continue(()); };
    let n = t.slots;
    let eq = kinds.len();
    let Ok(s) = usize::try_from(slot) else { return Continue(()); };
    // The menu's own mapping, with the equipment slots in it: the container slots, then the
    // equipment, then the inventory and the hotbar (the original maps the inventory over the
    // equipment's places, so a click on one never reached a menu slot).
    Break(match s {
        _ if s < n + eq => Some(300 + s),
        _ if s < n + eq + 27 => Some(9 + s - n - eq),
        _ if s < n + eq + 36 => Some(s - n - eq - 27),
        _ => None,
    })
}
