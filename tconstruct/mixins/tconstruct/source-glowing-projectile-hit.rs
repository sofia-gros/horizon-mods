// Arrow hits retain their firing stack during flight; save/reload is not yet wired.
fn tconstruct_glowing_projectile_hit(game: &mut crate::game::Game<'_>, p: &horizon_client::projectiles::Projectile, hit: horizon_client::projectiles::Hit) {
    use horizon_client::{mobs::Who, projectiles::Hit};
    let Some(weapon) = p.arrow.as_ref().and_then(|a| a.fired_from_weapon.as_ref()) else { return; };
    // A disconnected guest must not fall back to host permissions.
    if p.guest.is_some() && p.guest != game.lan_acting { return; }
    if !crate::game::tconstruct_glowing_world::has_glowing(game, weapon) { return; }
    let (at, preferred, remove_arrow) = match hit {
        Hit::Block { pos, face, .. } if face != [0,0,0] => {
            let Some(x) = pos[0].checked_add(face[0]) else { return; };
            let Some(y) = pos[1].checked_add(face[1]) else { return; };
            let Some(z) = pos[2].checked_add(face[2]) else { return; };
            ([x,y,z], [-face[0],-face[1],-face[2]], true)
        }
        Hit::Entity { who: Who::Mob(id), .. } => {
            let Some(mob) = game.mobs.iter().find(|mob| mob.id == id) else { return; };
            ([mob.position.x.floor() as i32,mob.position.y.floor() as i32,mob.position.z.floor() as i32], [0,-1,0], false)
        }
        _ => return,
    };
    match crate::game::tconstruct_glowing_world::place(game, at, preferred) {
        Ok(true) if remove_arrow => {
            if let Some(arrow) = game.creatures.projectiles.iter_mut().find(|arrow| arrow.id == p.id).and_then(|arrow| arrow.arrow.as_mut()) {
                arrow.removed = true;
            }
        }
        Ok(_) => {},
        Err(error) => eprintln!("tconstruct Glowing projectile: {error}"),
    }
}
