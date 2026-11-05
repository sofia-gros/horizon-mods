fn tconstruct_strong_bones_effect_begin(
    effect: &mut crate::effects::EffectInstance,
    incoming: &crate::effects::EffectInstance,
) {
    let marked = effect
        .__horizon_mixins_tconstruct_strong_bones_cure_item()
        .is_some()
        || incoming
            .__horizon_mixins_tconstruct_strong_bones_cure_item()
            .is_some();
    let replaces = crate::effects::tconstruct_strong_bones_rules::replaces_active(
        effect.duration,
        effect.amplifier,
        incoming.duration,
        incoming.amplifier,
    );
    *effect.__horizon_mixins_tconstruct_strong_bones_pending_cure_mut() = (marked && replaces)
        .then(|| {
            incoming
                .__horizon_mixins_tconstruct_strong_bones_cure_item()
                .clone()
        });
}
