// The source sword/cleaver sweep is a separate, reduced-damage path: it is
// not the circle attack and does not apply a separate tool wear per target.
fn tconstruct_melee_sweep(game:&mut crate::game::Game<'_>, index:usize, _scale:f32) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break,Continue};
    use crate::game::tconstruct_source_melee_shape::{self as shape,Attack};
    let Some(stack)=game.held() else{return Continue(());};
    let Some(kind @ Attack::Sweep(range))=shape::attack(stack.item.as_ref()) else{return Continue(());};
    if shape::intact(stack).is_none() {return Break(());}
    let Some(player)=game.player.as_ref() else{return Break(());};
    let from=[player.position.x,player.position.y,player.position.z];
    let Some(primary)=game.mobs.get(index) else{return Break(());};
    let p=primary.position;let target=[p.x,p.y,p.z];
    let bounds=primary.bounds().inflate(horizon_core::geometry::Vec3::new(range,0.25,range));
    let allies=game.player_allies();
    let uuid=game.level.player_uuid.or(Some([0,0,0,1]));
    let candidates:Vec<u32>=game.mobs.iter().enumerate().filter(|(i,mob)| {
        *i!=index && mob.alive() && !mob.removed && !allies.contains(&mob.id)
            && !crate::game::mace::ally_of_player(mob,uuid) && mob.bounds().intersects(bounds)
            && shape::in_distance(kind,target,from,[mob.position.x,mob.position.y,mob.position.z])
    }).map(|(_,mob)|mob.id).collect();
    let (xd,zd)=game.extra_knockback_direction();
    for id in candidates {
        let Some(i)=game.mobs.iter().position(|mob|mob.id==id && mob.alive() && !mob.removed) else {continue;};
        game.mobs[i].knockback(0.4,xd,zd);
        let source=game.attack_source(from);
        if game.hurt_by_player(i,1.0,source) {game.last_hurt_by_player.insert(id,game.lan_acting);}
    }
    // Source sweep against other players is not connected yet; do not treat the
    // vanilla fixed-radius sweep as equivalent to Cleaver's larger radius.
    Break(())
}
