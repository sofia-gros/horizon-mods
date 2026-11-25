fn pluto_skull_extents(renderer:&ItemRenderer,model:&SpecialModel)->std::ops::ControlFlow<Vec<Vec3>> {
    let SpecialKind::Head {kind,animation,..}=&model.kind else {return std::ops::ControlFlow::Continue(())};
    if !kind.contains(':') || kind.starts_with("minecraft:") {return std::ops::ControlFlow::Continue(());}
    let definition=match sunrise_assets::item_model::pluto_skull_model_access::definition(kind) {
        Ok(None)=>return std::ops::ControlFlow::Continue(()),
        Ok(Some(definition))=>definition,
        Err(error)=>{eprintln!("PlutoLib: custom skull bounds unavailable: {error}");return std::ops::ControlFlow::Break(Vec::new());},
    };
    let mut out=Vec::new();
    if let Some(part)=renderer.part(&definition.item) {
        let style=sunrise_assets::item_model::pluto_skull_model_access::animation_kind(definition.animation);
        extents_of(part,"root",Mat4::IDENTITY,&skull_pose(style,*animation),&mut out);
    }
    std::ops::ControlFlow::Break(out)
}
