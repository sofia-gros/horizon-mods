// Extend only registered custom head kinds; every native codec path stays native.
fn pluto_skull_special_decode(v:&Value)->std::ops::ControlFlow<Result<SpecialModel>> {
    if !matches!(v.get("type").and_then(Value::as_str),Some("minecraft:head"|"head")) {return std::ops::ControlFlow::Continue(());}
    let Some(kind)=v.get("kind").and_then(Value::as_str) else {return std::ops::ControlFlow::Continue(())};
    if !kind.contains(':') || kind.starts_with("minecraft:") {return std::ops::ControlFlow::Continue(());}
    match crate::item_model::pluto_skull_model_access::definition(kind) {
        Ok(None)=>std::ops::ControlFlow::Continue(()),
        Err(error)=>std::ops::ControlFlow::Break(Err(err(&error))),
        Ok(Some(_))=>std::ops::ControlFlow::Break((|| {
            let m=object(v)?;
            Ok(SpecialModel {kind:SpecialKind::Head {kind:kind.to_owned(),
                texture:get(m,"texture").map(ident).transpose()?,
                animation:get(m,"animation").map_or(Ok(0.0),float)?,
            }})
        })()),
    }
}
