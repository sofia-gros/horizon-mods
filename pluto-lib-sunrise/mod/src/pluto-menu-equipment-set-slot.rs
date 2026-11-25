// A menu's slots are first written by its session's copy-in: a content mod
// menu type's registered equipment joins the menu here, before that first
// write, so every slot the session sets, clicks and copies back is the
// layout's. A menu already carrying its equipment is left alone.
fn pluto_menu_equipment_set_slot(menu: &mut Menu) {
    pluto_menu_equipment::insert_equipment(menu);
}
