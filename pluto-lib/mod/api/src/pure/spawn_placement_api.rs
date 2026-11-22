//! Spawn placement predicates (the SpawnPlacements shape): a type's own
//! check for a candidate spawn position, registered per entity type. Where
//! the game consults placements, a registered predicate's verdict replaces
//! the inherited species rules for that type; unregistered types keep them.
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
}
