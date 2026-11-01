fn tconstruct_builtin_knockback(value:(f32,bool), game:&mut crate::game::Game<'_>) -> (f32,bool) {
    let Some(stack) = game.held() else { return value; };
    let Some(max) = crate::game::tconstruct_source_transform_logic::maximum(stack) else { return value; };
    if stack.damage >= max { return value; }
    (value.0 + crate::game::tconstruct_source_melee_knockback::builtin_bonus(stack.item.as_ref()), value.1)
}
