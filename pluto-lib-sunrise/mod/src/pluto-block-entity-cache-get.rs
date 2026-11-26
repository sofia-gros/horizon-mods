// Concrete native lookup guard; retain the original key/revision checks and
// update of last-use time. Real function/site validation remains pending.
fn pluto_block_entity_cache_get(cache: &mut KeptGeometry, pos: [i32; 3]) {
    use crate::game::pluto_block_entity_cache::{begin_lookup, clear_cache, entry_generation, note_entry_missing, reusable};
    let generation = begin_lookup(crate::pluto_client_block_entity_state::cache_generation());
    let cache_key = std::ptr::from_ref(cache) as usize;
    if generation.is_none() {
        cache.entries.clear();
        clear_cache(cache_key);
    } else if cache.entries.get(&pos).is_some() {
        // An entry with no committed stamp is not reusable either: its
        // frame never recorded the generation it was kept under.
        let reusable_entry = entry_generation(cache_key, pos)
            .is_some_and(|entry| reusable(entry, generation));
        if !reusable_entry {
            cache.entries.remove(&pos);
            // The frame goes on to draw and keep a replacement under
            // this lookup's generation; record it as pending.
            if let Some(generation) = generation {
                note_entry_missing(cache_key, pos, generation);
            }
        }
    } else if let Some(generation) = generation {
        note_entry_missing(cache_key, pos, generation);
    }
}
