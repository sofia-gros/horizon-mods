fn tconstruct_bonking_knockback(entity: i32, canceled: &mut bool) {
    if crate::game::tconstruct_bonking_world::suppress_knockback(entity) {
        *canceled = true;
    }
}
