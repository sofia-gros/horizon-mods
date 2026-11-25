fn pluto_skull_submit(renderer:&ItemRenderer,model:&SpecialModel,pose:Mat4,
    sprites:&HashMap<String,Rect>,sink:&mut dyn ItemSink)->std::ops::ControlFlow<()> {
    let SpecialKind::Head {kind,texture,animation}=&model.kind else {return std::ops::ControlFlow::Continue(())};
    if !kind.contains(':') || kind.starts_with("minecraft:") {return std::ops::ControlFlow::Continue(());}
    let definition=match sunrise_assets::item_model::pluto_skull_model_access::definition(kind) {
        Ok(None)=>return std::ops::ControlFlow::Continue(()),
        Ok(Some(definition))=>definition,
        Err(error)=>{eprintln!("PlutoLib: custom skull draw unavailable: {error}");return std::ops::ControlFlow::Break(());},
    };
    let skin=sunrise_assets::item_model::pluto_skull_model_access::sprite(definition,texture.as_deref());
    if let (Some(part),Some(rect))=(renderer.part(&definition.item),sprites.get(&skin).copied()) {
        let style=sunrise_assets::item_model::pluto_skull_model_access::animation_kind(definition.animation);
        draw(part,"root",pose,&skull_pose(style,*animation),rect,argb(-1),Foil::None,sink);
    }
    std::ops::ControlFlow::Break(())
}
