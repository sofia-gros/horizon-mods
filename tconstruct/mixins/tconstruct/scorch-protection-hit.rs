fn tconstruct_scorch_protection_hit(
    game: &mut crate::game::Game<'_>, amount: f32, from: Option<[f64;3]>, damage_type: &str,
    known: Option<Option<u32>>,
    original: impl FnOnce(&mut crate::game::Game<'_>,f32,Option<[f64;3]>,&str,Option<Option<u32>>) -> bool,
) -> bool {
    crate::game::tconstruct_scorch_hit_world::wrap(game,amount,from,damage_type,known,original)
}
