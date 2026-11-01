fn tconstruct_melee_circle(game:&mut crate::game::Game<'_>, index:usize) {
    use crate::game::tconstruct_source_melee_shape::{self as shape,Attack};
    if game.remote.is_some() {return;}
    let actor=game.lan_acting;
    if game.world.chunks.__horizon_mixins_tconstruct_melee_active().contains(&actor) {return;}
    let Some((target,ticks))=game.world.chunks.__horizon_mixins_tconstruct_melee_context().get(&actor).copied() else {return;};
    if game.mobs.get(index).is_none_or(|mob|mob.id!=target) {return;}
    let Some(stack)=game.held().cloned() else{return;};
    let Some(kind @ Attack::Circle(range))=shape::attack(stack.item.as_ref()) else{return;};
    let Some(_max)=shape::intact(&stack) else{return;};
    let primary=game.mobs[index].position;
    let bounds=game.mobs[index].bounds().inflate(horizon_core::geometry::Vec3::new(range,0.25,range));
    let primary_pos=[primary.x,primary.y,primary.z];
    let allies=game.player_allies();
    let uuid=game.level.player_uuid.or(Some([0,0,0,1]));
    let candidates:Vec<u32>=game.mobs.iter().enumerate().filter(|(i,mob)| {
        *i!=index && mob.alive() && !mob.removed && !allies.contains(&mob.id)
            && !crate::game::mace::ally_of_player(mob,uuid)
            && mob.bounds().intersects(bounds)
            && shape::in_distance(kind,primary_pos,primary_pos,[mob.position.x,mob.position.y,mob.position.z])
    }).map(|(_,mob)|mob.id).collect();
    let (xd,zd)=game.extra_knockback_direction();
    game.world.chunks.__horizon_mixins_tconstruct_melee_active_mut().insert(actor);
    for id in candidates {
        if !game.held().is_some_and(|held|held.item==stack.item && shape::intact(held).is_some()) {break;}
        let Some(next)=game.mobs.iter().position(|mob|mob.id==id && mob.alive() && !mob.removed) else {continue;};
        // Source circle applies a sweep push before its separate full hit.
        game.mobs[next].knockback(0.4,xd,zd);
        game.attack_ticks=ticks;
        game.attack_mob(next);
    }
    game.attack_ticks=0;
    game.world.chunks.__horizon_mixins_tconstruct_melee_active_mut().remove(&actor);
}
