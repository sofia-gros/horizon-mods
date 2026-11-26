// Mobs scale how far they see a player by PlayerView::visibility_percent (sneaking, invisibility, worn heads). The mods may scale it
// further for the kind of mob that is looking (LivingEntity.getVisibilityPercent's LivingVisibilityEvent): the native value is kept
// and multiplied by the factor, which is 1 when nobody is registered, nobody is looking or the player wears nothing that applies.
fn pluto_player_visibility(value:f64,player:&PlayerView,targeting:Option<&str>)->f64 {
    let Some(kind)=targeting else {return value};
    let Some(sink)=crate::mobs::pluto_player_visibility_access::sink() else {return value};
    let mut uuid=0_u128;
    for word in player.uuid { uuid=(uuid<<32)|u128::from(word as u32); }
    let factor=(sink.factor)(uuid,kind);
    if factor.is_finite() && factor != 1.0 { value*factor.max(0.0) } else { value }
}
