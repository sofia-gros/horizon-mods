fn pluto_skull_worn_kind(item:&str)->std::ops::ControlFlow<Option<&'static str>> {
    if !item.contains(':') || item.starts_with("minecraft:") {return std::ops::ControlFlow::Continue(());}
    match sunrise_client_lib::item_render::pluto_skull_model_access::worn_kind(item) {
        Ok(Some(kind))=>std::ops::ControlFlow::Break(Some(kind)),
        Ok(None)=>std::ops::ControlFlow::Continue(()),
        Err(error)=>{eprintln!("PlutoLib: custom worn skull unavailable: {error}");std::ops::ControlFlow::Break(None)},
    }
}
