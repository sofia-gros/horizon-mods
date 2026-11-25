// The one effect add the API side recorded an icon flag for (see
// pluto-effect-apply-access) has the flag put on the instance as the host
// builds it, so an effect given with its icon hidden is given hidden, as
// vanilla's addEffect gives the instance it is handed; an add with no
// record keeps the flag the instance was built with.
fn pluto_effect_apply_head(value: horizon_client::effects::EffectInstance) -> horizon_client::effects::EffectInstance {
    let mut value = value;
    if let Some(show_icon) = crate::entry::pluto_effect_apply_access::take(value.id) {
        value.show_icon = show_icon;
    }
    value
}
