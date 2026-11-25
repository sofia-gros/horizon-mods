// Every guest menu operation loads the guest's inventory into the menu
// first: a content mod menu type's registered equipment joins the menu
// here, before the operation runs, so the operation's slots are the
// layout's. A menu already carrying its equipment is left alone.
fn pluto_menu_equipment_guest_load(menu: &mut Menu) {
    sunrise_client_lib::menu::pluto_menu_equipment::insert_equipment(menu);
}
