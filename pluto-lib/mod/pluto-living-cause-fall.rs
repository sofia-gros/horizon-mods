// A class causeFallDamage override, consulted before the inherited
// application. A decided outcome replaces the application: the block's own
// fallOn still runs through mod_block_fall with zero damage, exactly as the
// native living-fall cancellation branch shapes it, and a bounce additionally
// rewrites the motion and clears the landing state. When no class decides,
// the inherited body runs untouched.
#[allow(clippy::too_many_arguments)]
fn pluto_living_cause_fall(
    mob:&mut Mob,
    distance:f64,
    multiplier:f32,
    events:&mut Vec<MobEvent>,
    contact:Option<(&MobWorld,[i32;3])>,
) -> std::ops::ControlFlow<bool> {
    let Some(sink)=super::pluto_living_entity_access::sink() else { return std::ops::ControlFlow::Continue(()) };
    if !(sink.has_cause_fall)(&mob.kind.id) { return std::ops::ControlFlow::Continue(()); }
    let velocity=[mob.velocity.x,mob.velocity.y,mob.velocity.z];
    // Mobs never suppress a bounce; only a sneaking player does, and players
    // do not land through this Mob path.
    match (sink.cause_fall)(mob.id,&mob.kind.id,distance,multiplier,velocity,false) {
        None=>std::ops::ControlFlow::Continue(()),
        Some(super::pluto_living_entity_access::FallOutcome::Cancel)=>{
            mob.mod_block_fall(contact,distance,0.0);
            std::ops::ControlFlow::Break(false)
        }
        Some(super::pluto_living_entity_access::FallOutcome::Bounce { velocity,sound })=>{
            mob.mod_block_fall(contact,distance,0.0);
            mob.velocity=Vec3::new(velocity[0],velocity[1],velocity[2]);
            mob.fall_distance=0.0;
            mob.on_ground=false;
            if let Some(sound)=sound {
                events.push(MobEvent::Sound(sound,[mob.position.x,mob.position.y,mob.position.z],1.0,1.0));
            }
            std::ops::ControlFlow::Break(false)
        }
    }
}
