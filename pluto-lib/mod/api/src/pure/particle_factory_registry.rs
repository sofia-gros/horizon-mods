//! Fabric-shaped particle factory registration, beginning with the standard
//! BreakingItemParticle constructor. Native request attachment is unvalidated.
use std::{collections::BTreeMap,sync::RwLock};
use super::{base::Identifier,player_api::StackData};
struct Registry {
    // Native MobEvent carries static names. Intern each DECLARED ID once for
    // this process-lifetime registry; replacing a factory never interns again.
    items:BTreeMap<&'static str,StackData>,
    entity_overrides:BTreeMap<(String,String),&'static str>,
}
static ITEMS:RwLock<Registry>=RwLock::new(Registry {items:BTreeMap::new(),entity_overrides:BTreeMap::new()});

/// Registers particle factories independently of any one mod's particle names.
/// Item factories delegate simulation, UV selection and RNG to the host's native
/// BreakingItemParticle; this API does not define custom particle physics.
pub struct ParticleFactoryRegistry;
impl ParticleFactoryRegistry {
    /// Register an item-fragment factory with an owned stack snapshot. Repeated
    /// registration replaces the factory. Rejects invalid IDs, empty counts,
    /// counts outside Java int and nonpositive durability overrides atomically.
    pub fn register_item(particle:Identifier,stack:StackData)->Result<(),String> {
        Identifier::parse(&stack.item).map_err(|e|e.to_string())?;
        if stack.count==0 || stack.count>i32::MAX as u32 || stack.max_damage.is_some_and(|n|n<=0) {
            return Err("particle factory requires a nonempty valid item stack".into());
        }
        let name=particle.to_string();
        let old={
            let mut registry=ITEMS.write().map_err(|_|"particle factory registry poisoned")?;
            if let Some(previous)=registry.items.get_mut(name.as_str()) {
                Some(std::mem::replace(previous,stack))
            } else {
                let name:&'static str=Box::leak(name.into_boxed_str());
                registry.items.insert(name,stack)
            }
        };
        drop(old);
        Ok(())
    }
    /// Replace an entity type's inherited particle with a registered item
    /// factory. The replacement must have a non-Minecraft namespace, so native
    /// enum-name consumers cannot also emit it. Source caller coverage is cube
    /// landing events only; other emitters remain a documented native stub.
    pub fn register_entity_override(entity:Identifier,original:Identifier,replacement:Identifier)->Result<(),String> {
        if replacement.namespace()=="minecraft" {return Err("entity particle replacement must use a custom namespace".into());}
        let mut registry=ITEMS.write().map_err(|_|"particle factory registry poisoned")?;
        let name=*registry.items.get_key_value(replacement.to_string().as_str())
            .ok_or("replacement particle factory is not registered")?.0;
        registry.entity_overrides.insert((entity.to_string(),original.to_string()),name);
        Ok(())
    }
    /// An independent copy of an item factory's stack, or None when unregistered.
    /// Direct lookup is functional; native request/model attachment remains
    /// source-declared and unvalidated, not a claim that a particle was drawn.
    pub fn item(particle:&Identifier)->Option<StackData> { item_from_host(&particle.to_string()) }
}
pub(crate) fn item_from_host(particle:&str)->Option<StackData> {
    ITEMS.read().ok()?.items.get(particle).cloned()
}
pub(crate) fn entity_particle_from_host(entity:&str,original:&str)->Option<&'static str> {
    let original=if original.contains(':') {original.to_owned()} else {format!("minecraft:{original}")};
    ITEMS.read().ok()?.entity_overrides.get(&(entity.to_owned(),original)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::base::Nbt;
    #[test]
    fn item_factories_copy_all_stack_data_and_invalid_replacements_leave_prior_factory_intact() {
        let id=Identifier::parse("example:crumbs").unwrap();
        let original=StackData {item:"example:painted_item".into(),count:2,damage:4,max_damage:Some(30),
            custom_data:[("test:colour".into(),Nbt::Int(0x123456))].into_iter().collect(),custom_model_data:None};
        ParticleFactoryRegistry::register_item(id.clone(),original.clone()).unwrap();
        let mut copy=ParticleFactoryRegistry::item(&id).unwrap();
        assert_eq!(copy,original); copy.count=99; copy.custom_data.clear();
        assert_eq!(ParticleFactoryRegistry::item(&id),Some(original.clone()));
        let mut bad=original.clone();bad.count=0;
        assert!(ParticleFactoryRegistry::register_item(id.clone(),bad).is_err());
        let mut bad=original.clone();bad.item="bad item".into();
        assert!(ParticleFactoryRegistry::register_item(id.clone(),bad).is_err());
        let mut bad=original.clone();bad.count=u32::MAX;
        assert!(ParticleFactoryRegistry::register_item(id.clone(),bad).is_err());
        let mut bad=original.clone();bad.max_damage=Some(0);
        assert!(ParticleFactoryRegistry::register_item(id.clone(),bad).is_err());
        assert_eq!(ParticleFactoryRegistry::item(&id),Some(original));
        ParticleFactoryRegistry::register_item(id.clone(),copy.clone()).unwrap();
        assert_eq!(ParticleFactoryRegistry::item(&id),Some(copy.clone()));
        assert!(ParticleFactoryRegistry::item(&Identifier::parse("example:missing_crumbs").unwrap()).is_none());
        let entity=Identifier::parse("example:cube").unwrap();
        let inherited=Identifier::parse("minecraft:item_slime").unwrap();
        ParticleFactoryRegistry::register_entity_override(entity.clone(),inherited.clone(),id.clone()).unwrap();
        let name=entity_particle_from_host("example:cube","item_slime").unwrap();
        assert_eq!(name,"example:crumbs");
        assert_eq!(entity_particle_from_host("example:cube","minecraft:item_slime"),Some(name));
        assert_eq!(entity_particle_from_host("minecraft:slime","item_slime"),None);
        assert_eq!(entity_particle_from_host("example:cube","flame"),None);
        assert!(ParticleFactoryRegistry::register_entity_override(entity.clone(),inherited.clone(),
            Identifier::parse("example:unregistered_crumbs").unwrap()).is_err());
        assert!(ParticleFactoryRegistry::register_entity_override(entity,inherited,
            Identifier::parse("minecraft:item_slime").unwrap()).is_err());
        ParticleFactoryRegistry::register_item(id,copy).unwrap();
        let after=entity_particle_from_host("example:cube","item_slime").unwrap();
        assert!(std::ptr::eq(name,after),"replacement reuses the one process-lifetime name, including in-flight event references");
    }
}
