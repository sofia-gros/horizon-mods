fn tconstruct_rugged_health(
    value: std::io::Result<bool>,
    environment: &mut crate::health::Environment<'_>,
) -> std::io::Result<bool> {
    // Preserve earlier native/mod cancellations and errors. This hook still
    // precedes shields, armor wear, hurt cooldown, knockback and statistics.
    let canceled = value?;
    if canceled { return Ok(true); }
    let blocked = environment.status.__horizon_mixins_tconstruct_rugged_blocked_types()
        .iter().any(|kind|kind == &environment.source.definition().name);
    if blocked { environment.canceled = true; }
    Ok(blocked)
}
