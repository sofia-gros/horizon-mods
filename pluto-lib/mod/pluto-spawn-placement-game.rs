// Natural-spawn placement consultation: the facts of the candidate the
// selection accepted, read once from the game's own level view, and the
// registered predicate's verdict. Unregistered types are never consulted
// (the provider answers None) and the spawn proceeds untouched.
//
// The consultation sits at finalize time, after the inherited species
// rules passed: a registered predicate can only turn a spawn away here,
// never resurrect one the species rules rejected. That asymmetry is the
// current weaver's limit (the selection gate itself is generic over the
// level type, so no helper can read its facts); the registry and context
// are the general surface, ready for a selection-level consultation.
use super::Game;
use horizon_client::{entry::pluto_spawn_placement as access,mobs::Species};
use horizon_client::mobs::spawn_rules::SpawnLevel;

pub(crate) fn natural_allowed(game:&Game<'_>,entity:&str,species:Species,position:[f64;3])->bool {
    if !access::active() { return true; }
    let level=game.natural_level();
    let [x,y,z]=[position[0].floor() as i32,position[1].floor() as i32,position[2].floor() as i32];
    let facts=level.facts();
    let below=facts.get(level.state(x,y-1,z));
    let at=facts.get(level.state(x,y,z));
    let fluid_name=|fluid:&horizon_client::blocks::Fluid| {
        if matches!(fluid,horizon_client::blocks::Fluid::Water) { "minecraft:water" }
        else if matches!(fluid,horizon_client::blocks::Fluid::Lava) { "minecraft:lava" }
        else { "minecraft:empty" }
    };
    // The darkness fact draws from a clone of the world stream, like the
    // spawn adapters' RNG handling; the canonical stream keeps its place.
    let mut rng=game.world.random.clone();
    let verdict=access::evaluate(&access::Facts {
        entity_type:entity.to_string(),dimension:game.dim.id().into(),position:[x,y,z],reason:"natural".into(),
        peaceful:level.peaceful(),block_below:below.name.to_string(),
        fluid_at:fluid_name(&at.fluid).into(),fluid_below:fluid_name(&below.fluid).into(),
        block_below_valid_spawn:horizon_client::mobs::spawn_rules::is_valid_spawn(species,&level,x,y-1,z),
        dark_enough:horizon_client::mobs::spawn_rules::is_dark_enough(&level,&mut rng,x,y,z),
    });
    verdict.unwrap_or(true)
}
