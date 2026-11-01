// Source extra attacks apply tool damage, but do not call the primary-hit
// ItemStack.hurtEnemy hook or award the primary item-used statistic again.
fn tconstruct_melee_extra_hit(game:&mut crate::game::Game<'_>, _index:usize) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break,Continue};
    if !game.world.chunks.__horizon_mixins_tconstruct_melee_active().contains(&game.lan_acting) {return Continue(());}
    if let Some(stack)=game.held().cloned() {game.weapon_wear(&stack);}
    Break(())
}
