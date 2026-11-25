// A menu's slot count is its layout's count: a content mod menu type's
// registered equipment is part of the layout, so a menu counted before its
// pieces join it (a session counts a menu it built only for the count)
// already counts them, and a menu carrying them counts itself.
fn pluto_menu_equipment_slot_count(value: usize, menu: &Menu) -> usize {
    value + pluto_menu_equipment::missing_equipment(menu)
}
