// Concrete native lookup guard; retain the original key/revision checks and
// update of last-use time. Real function/site validation remains pending.
fn pluto_block_entity_cache_get(cache: &mut KeptGeometry, pos: [i32; 3]) {
    use crate::game::pluto_block_entity_cache::{begin_lookup, reusable};
    let generation = begin_lookup(sunrise_client_render::pluto_client_block_entity_state::cache_generation());
    if generation.is_none() {
        cache.entries.clear();
    } else if cache.entries.get(&pos).is_some_and(|entry|
        !reusable(*entry.__horizon_mixins_pluto_lib_pluto_block_entity_cache_generation(), generation)) {
        cache.entries.remove(&pos);
    }
}
