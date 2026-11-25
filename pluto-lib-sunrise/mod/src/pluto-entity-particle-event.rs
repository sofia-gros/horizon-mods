// A custom namespaced kind is ignored by the subsequent native enum consumer.
// Borrow/forward the actual event once before that match; leave all others alone.
fn pluto_entity_particle_event(value:MobEvent,game:&mut Game<'_>)->MobEvent {
    crate::game::pluto_item_particles::consume(game,&value);
    value
}
