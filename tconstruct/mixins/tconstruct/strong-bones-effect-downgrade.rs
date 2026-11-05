fn tconstruct_strong_bones_effect_downgrade(effect: &mut crate::effects::EffectInstance) {
    if effect.duration == 0
        && let Some(hidden) = effect.hidden.as_deref()
    {
        let item = hidden
            .__horizon_mixins_tconstruct_strong_bones_cure_item()
            .clone();
        *effect.__horizon_mixins_tconstruct_strong_bones_cure_item_mut() = item;
    }
}
