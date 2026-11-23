//! Spawn placement predicates (the SpawnPlacements shape): a type's own
//! check for a candidate spawn position, registered per entity type. Where
//! the game consults placements, a registered predicate's verdict replaces
//! the inherited species rules for that type; unregistered types keep them.
//!
//! A second registration form carries the original's `Operation.OR`: an
//! OR-registered predicate ADDS spawn positions the inherited species rules
//! rejected. The game's own selection never proposes those positions, so
//! the game-side permit-add pass proposes its own candidates around players
//! after the game's natural pass and spawns the ones an OR predicate
//! accepts, through the game's own preparation and finalization. The two
//! forms are independent: a type may carry either, or both (a replacing
//! verdict for the game's candidates, an OR verdict for the added ones).
use super::base::Identifier;
use std::{collections::BTreeMap,sync::{Arc,Mutex}};

/// The facts of one candidate position, read by the game at the check.
#[derive(Clone,Debug)]
pub struct SpawnPlacementContext {
    pub entity_type:Identifier,
    /// The dimension the position is in.
    pub dimension:Identifier,
    /// Block position of the candidate (the mob's feet).
    pub position:[i32;3],
    /// Backend reason: natural, spawner, structure, conversion, ...
    pub reason:String,
    /// The level's difficulty is peaceful.
    pub peaceful:bool,
    /// The block below the candidate.
    pub block_below:Identifier,
    /// The fluid at the candidate (`minecraft:empty` when none).
    pub fluid_at:Identifier,
    /// The fluid below the candidate (`minecraft:empty` when none).
    pub fluid_below:Identifier,
    /// The block below is a valid spawn block for the type's base.
    pub block_below_valid_spawn:bool,
    /// A monster would be dark enough to spawn at the candidate.
    pub dark_enough:bool,
}
type Predicate=Arc<dyn Fn(&SpawnPlacementContext)->bool+Send+Sync>;
static REGISTRY:Mutex<BTreeMap<String,Predicate>>=Mutex::new(BTreeMap::new());
static OR_REGISTRY:Mutex<BTreeMap<String,Predicate>>=Mutex::new(BTreeMap::new());
/// Per-type spawn placement registration. One predicate per type; a second
/// registration for the same type rejects.
pub struct SpawnPlacementRegistry;
impl SpawnPlacementRegistry {
    pub fn register(entity_type:Identifier,predicate:impl Fn(&SpawnPlacementContext)->bool+Send+Sync+'static)->Result<(),String> {
        let mut registry=REGISTRY.lock().map_err(|_|"spawn placement registry poisoned")?;
        let id=entity_type.to_string();
        if registry.contains_key(&id) {return Err(format!("spawn placement {id} already registered"));}
        registry.insert(id,Arc::new(predicate));Ok(())
    }
    pub fn is_registered(entity_type:&Identifier)->bool {
        REGISTRY.lock().is_ok_and(|registry|registry.contains_key(&entity_type.to_string()))
    }
    /// Registers a type's OR placement predicate (the original's `Operation.OR`): positions
    /// the inherited species rules rejected may still spawn when this predicate accepts them,
    /// through the game's permit-add pass. One OR predicate per type; a second registration
    /// for the same type rejects.
    pub fn register_or(entity_type:Identifier,predicate:impl Fn(&SpawnPlacementContext)->bool+Send+Sync+'static)->Result<(),String> {
        let mut registry=OR_REGISTRY.lock().map_err(|_|"spawn placement registry poisoned")?;
        let id=entity_type.to_string();
        if registry.contains_key(&id) {return Err(format!("spawn placement {id} already registered"));}
        registry.insert(id,Arc::new(predicate));Ok(())
    }
    pub fn is_or_registered(entity_type:&Identifier)->bool {
        OR_REGISTRY.lock().is_ok_and(|registry|registry.contains_key(&entity_type.to_string()))
    }
    pub(crate) fn has_or_registrations()->bool {
        OR_REGISTRY.lock().is_ok_and(|registry|!registry.is_empty())
    }
    /// The types carrying an OR predicate, in id order: the permit-add pass proposes
    /// candidates for exactly these.
    pub(crate) fn or_types()->Vec<Identifier> {
        OR_REGISTRY.lock().map(|registry|registry.keys().filter_map(|id|Identifier::parse(id).ok()).collect()).unwrap_or_default()
    }
    /// The registered OR predicate's verdict, or None when the type has none.
    pub(crate) fn evaluate_or(context:&SpawnPlacementContext)->Option<bool> {
        let predicate=OR_REGISTRY.lock().ok()?.get(&context.entity_type.to_string()).cloned()?;
        Some(predicate(context))
    }
    pub(crate) fn has_registrations()->bool {
        REGISTRY.lock().is_ok_and(|registry|!registry.is_empty())
    }
    /// The registered predicate's verdict, or None when the type has none
    /// and the inherited species rules decide.
    pub(crate) fn evaluate(context:&SpawnPlacementContext)->Option<bool> {
        let predicate=REGISTRY.lock().ok()?.get(&context.entity_type.to_string()).cloned()?;
        Some(predicate(context))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn context(kind:&str,below:&str)->SpawnPlacementContext {
        SpawnPlacementContext {entity_type:Identifier::parse(kind).unwrap(),
            dimension:Identifier::parse("minecraft:overworld").unwrap(),position:[0,64,0],reason:"natural".into(),
            peaceful:false,block_below:Identifier::parse(below).unwrap(),
            fluid_at:Identifier::parse("minecraft:empty").unwrap(),fluid_below:Identifier::parse("minecraft:empty").unwrap(),
            block_below_valid_spawn:true,dark_enough:true}
    }
    #[test]
    fn registered_predicates_decide_and_unregistered_types_decline() {
        let kind=Identifier::parse("placement_test:slime").unwrap();
        SpawnPlacementRegistry::register(kind.clone(),|context|!context.peaceful
            && context.block_below==Identifier::parse("placement_test:grass").unwrap()).unwrap();
        assert!(SpawnPlacementRegistry::register(kind.clone(),|_|true).is_err());
        assert!(SpawnPlacementRegistry::is_registered(&kind));
        assert_eq!(SpawnPlacementRegistry::evaluate(&context("placement_test:slime","placement_test:grass")),Some(true));
        assert_eq!(SpawnPlacementRegistry::evaluate(&context("placement_test:slime","minecraft:stone")),Some(false));
        let mut peaceful=context("placement_test:slime","placement_test:grass");
        peaceful.peaceful=true;
        assert_eq!(SpawnPlacementRegistry::evaluate(&peaceful),Some(false));
        assert_eq!(SpawnPlacementRegistry::evaluate(&context("placement_test:other","placement_test:grass")),None);
    }
    #[test]
    fn or_predicates_are_a_separate_registry_with_their_own_verdicts() {
        let kind=Identifier::parse("placement_test:or_slime").unwrap();
        assert!(!SpawnPlacementRegistry::has_or_registrations()||SpawnPlacementRegistry::is_or_registered(&kind));
        SpawnPlacementRegistry::register_or(kind.clone(),|context|!context.peaceful
            && context.block_below==Identifier::parse("placement_test:grass").unwrap()).unwrap();
        assert!(SpawnPlacementRegistry::register_or(kind.clone(),|_|true).is_err());
        assert!(SpawnPlacementRegistry::is_or_registered(&kind));
        assert!(SpawnPlacementRegistry::has_or_registrations());
        assert!(SpawnPlacementRegistry::or_types().contains(&kind));
        assert_eq!(SpawnPlacementRegistry::evaluate_or(&context("placement_test:or_slime","placement_test:grass")),Some(true));
        assert_eq!(SpawnPlacementRegistry::evaluate_or(&context("placement_test:or_slime","minecraft:stone")),Some(false));
        // The OR registration is not a replacing one, and the reverse holds too.
        assert_eq!(SpawnPlacementRegistry::evaluate(&context("placement_test:or_slime","placement_test:grass")),None);
        assert_eq!(SpawnPlacementRegistry::evaluate_or(&context("placement_test:slime","placement_test:grass")),None);
    }
}
