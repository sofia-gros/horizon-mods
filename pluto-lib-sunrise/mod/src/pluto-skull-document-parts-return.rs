// The shared startup document must not fall back to a second filesystem read.
fn pluto_skull_document_parts_return(mut value:HashMap<String,ModelPart>,document:&EntityModelDocument)->HashMap<String,ModelPart> {
    if let Err(error)=crate::item_render::pluto_skull_item_models::extend_document(&mut value,document.value()) {
        eprintln!("PlutoLib: custom skull geometry unavailable: {error}");
    }
    value
}
