// Keep cube landing's loop, positions and two original RNG draws per crumb.
fn pluto_entity_particle_kind(value:&'static str,m:&Mob)->&'static str {
    sunrise_client_lib::pluto_entity_particles::particle(&m.kind.id,value)
}
