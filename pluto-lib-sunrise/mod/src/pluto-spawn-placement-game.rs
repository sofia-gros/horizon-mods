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
//
// The OR (permit-add) side answers the asymmetry from the other end:
// after the game's own natural pass, `permit_add_spawns` proposes its
// own candidates for the OR-registered types — around the level's
// players, on the surface, outside the no-spawn radius — and spawns the
// ones an OR predicate accepts, through the same preparation and
// finalization the natural path uses. A type's OR predicate therefore
// adds the positions its inherited species rules reject, as the
// original's `Operation.OR` registration does; the candidate stream is
// the pass's own (a bounded number of attempts per type per pass), not
// the selection's, which is the part the weaver's limit costs.
use super::Game;
use sunrise_client::{entry::pluto_spawn_placement as access,mobs::{Mob,Species}};
use sunrise_client_lib::mobs::spawn_rules::SpawnLevel;
use sunrise_core::geometry::Vec3;
use sunrise_core::random::RandomSource;

/// The facts of one candidate position, read once from the game's own level view.
fn candidate_facts(game:&Game<'_>,level:&impl SpawnLevel,entity:&str,species:Species,x:i32,y:i32,z:i32)->access::Facts {
    let facts=level.facts();
    let below=facts.get(level.state(x,y-1,z));
    let at=facts.get(level.state(x,y,z));
    let fluid_name=|fluid:&sunrise_items::blocks::Fluid| {
        if matches!(fluid,sunrise_items::blocks::Fluid::Water) { "minecraft:water" }
        else if matches!(fluid,sunrise_items::blocks::Fluid::Lava) { "minecraft:lava" }
        else { "minecraft:empty" }
    };
    // The darkness fact draws from a clone of the world stream, like the
    // spawn adapters' RNG handling; the canonical stream keeps its place.
    let mut rng=game.world.random.clone();
    access::Facts {
        entity_type:entity.to_string(),dimension:game.dim.id().into(),position:[x,y,z],reason:"natural".into(),
        peaceful:level.peaceful(),block_below:below.name.to_string(),
        fluid_at:fluid_name(&at.fluid).into(),fluid_below:fluid_name(&below.fluid).into(),
        block_below_valid_spawn:sunrise_client_lib::mobs::spawn_rules::is_valid_spawn(species,level,x,y-1,z),
        dark_enough:sunrise_client_lib::mobs::spawn_rules::is_dark_enough(level,&mut rng,x,y,z),
    }
}

pub(crate) fn natural_allowed(game:&Game<'_>,entity:&str,species:Species,position:[f64;3])->bool {
    if !access::active() { return true; }
    let level=game.natural_level();
    let [x,y,z]=[position[0].floor() as i32,position[1].floor() as i32,position[2].floor() as i32];
    let verdict=access::evaluate(&candidate_facts(game,&level,entity,species,x,y,z));
    verdict.unwrap_or(true)
}

/// The permit-add pass's attempts per OR-registered type in one natural pass.
const PERMIT_ATTEMPTS:i32=4;

/// Proposes candidates for the OR-registered types and returns the mobs whose
/// OR predicate accepted, prepared and finalized the natural path's way; the
/// caller (the spawn_natural return helper, in the module that owns the add)
/// adds them. Candidates stand on the surface within 48 blocks of a randomly
/// drawn non-spectator player and never inside the 24-block no-spawn radius.
pub(crate) fn permit_add_spawns(game:&Game<'_>)->Vec<Mob> {
    if !access::active_or() || game.world_debug { return Vec::new(); }
    let players:Vec<_>=game.level_players().into_iter().filter(|player|!player.spectator).collect();
    if players.is_empty() { return Vec::new(); }
    let level=game.natural_level();
    let mut rng=game.world.random.clone();
    let mut spawned=Vec::new();
    for entity in access::or_types() {
        let Some(kind)=game.res.mob_types().get(&entity).cloned() else { continue };
        for _ in 0..PERMIT_ATTEMPTS {
            let anchor=&players[(rng.next_int_bound(players.len() as i32).unwrap_or(0)) as usize];
            let x=anchor.position.x.floor() as i32+rng.next_int_bound(97).unwrap_or(0)-48;
            let z=anchor.position.z.floor() as i32+rng.next_int_bound(97).unwrap_or(0)-48;
            let y=level.world_surface(x,z);
            if level.player_near(x as f64+0.5,y as f64,z as f64+0.5,24.0) { continue; }
            let position=[x as f64+0.5,y as f64,z as f64+0.5];
            let yaw=(rng.next_int_bound(360).unwrap_or(0)) as f32;
            let mut mob=Mob::new(0,kind.clone(),Vec3::new(position[0],position[1],position[2]),yaw);
            let facts=candidate_facts(game,&level,&entity,mob.species(),x,y,z);
            if access::evaluate_or(&facts)!=Some(true) { continue; }
            if let Err(error)=crate::game::pluto_entity_spawn_game::prepare(game,&mut mob,"natural") {
                log::error!("PlutoLib permit-add spawn preparation failed: {error}");
                continue;
            }
            let biome=game.biome_facts(x,y,z);
            let difficulty=game.local_difficulty(position);
            sunrise_client_lib::mobs::spawning::finalize(&mut mob,&biome,&difficulty);
            sunrise_client_lib::mobs::raider::finalize(&mut mob,sunrise_client_lib::mobs::raider::Reason::Natural);
            spawned.push(mob);
        }
    }
    spawned
}
