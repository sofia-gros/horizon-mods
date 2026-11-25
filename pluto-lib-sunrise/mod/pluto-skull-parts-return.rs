fn pluto_skull_parts_return(mut value:HashMap<String,ModelPart>,root:&Path)->HashMap<String,ModelPart> {
    if let Err(error)=crate::item_render::pluto_skull_item_models::extend(&mut value,root) {
        eprintln!("PlutoLib: custom skull geometry unavailable: {error}");
    }
    value
}
