// After the mob's own tick ran: record the climb state the tick settled, so
// the tick report built on the entity's post-tick event reads this tick's
// climbing flag and fall-flying state. A tick that did not run (the mob is
// gone) records nothing.
fn pluto_entity_climb_record(value: bool, m: &mut Mob) {
    if value {
        crate::game::pluto_entity_climb_access::record(m);
    }
}
