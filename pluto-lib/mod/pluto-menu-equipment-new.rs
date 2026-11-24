fn pluto_menu_equipment_new(menu: Menu) -> Menu {
    let mut menu = menu;
    pluto_menu_equipment::insert_equipment(&mut menu);
    menu
}
