fn patch_smeltery_controller_tank(raw: &horizon_world::nbt::Tag, state: &Smeltery) -> Option<horizon_world::nbt::Tag> {
    let horizon_world::nbt::Tag::Compound(mut fields) = raw.clone() else { return None; };
    fields.insert("tank".into(), state.tank.to_nbt());
    Some(horizon_world::nbt::Tag::Compound(fields))
}
