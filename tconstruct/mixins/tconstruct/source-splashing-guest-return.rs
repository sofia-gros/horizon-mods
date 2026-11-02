fn tconstruct_splashing_guest_return(value: (), game: &mut crate::game::Game<'_>, conn: u32) -> () {
    game.world.chunks.__horizon_mixins_tconstruct_splashing_guest_scope_mut().remove(&conn);
    value
}
