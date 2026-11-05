fn tconstruct_strong_bones_effect_end(
    value: bool,
    effect: &mut crate::effects::EffectInstance,
) -> bool {
    if let Some(item) = effect
        .__horizon_mixins_tconstruct_strong_bones_pending_cure_mut()
        .take()
    {
        *effect.__horizon_mixins_tconstruct_strong_bones_cure_item_mut() = item;
    }
    // Native update can clone the old active instance into the hidden chain.
    // Pending bookkeeping is not part of that instance's lasting metadata.
    let mut hidden = effect.hidden.as_deref_mut();
    while let Some(entry) = hidden {
        *entry.__horizon_mixins_tconstruct_strong_bones_pending_cure_mut() = None;
        hidden = entry.hidden.as_deref_mut();
    }
    value
}
