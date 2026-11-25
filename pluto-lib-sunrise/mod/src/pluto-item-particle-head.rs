fn pluto_item_particle_factory(game:&mut Game<'_>,options:&ParticleOptions,at:[f64;3],velocity:[f64;3])->std::ops::ControlFlow<bool> {
    crate::game::pluto_item_particles::apply(game,options,at,velocity)
}
