//! Entity-class query overrides, analogous to the block/item behaviour builders.
//! None delegates to the inherited implementation; Some(0.0) is an override.
use super::base::Identifier;
use std::{collections::BTreeMap,sync::{Arc,Mutex}};

/// The entity and native block factor of a no-argument getJumpPower query.
#[derive(Clone,Debug)]
pub struct JumpPowerContext {
    pub entity:u32,
    pub entity_type:Identifier,
    /// Present only for native cube/slime implementations; not a generic scale.
    pub slime_size:Option<i32>,
    pub block_jump_factor:f32,
}
/// The effective attack attribute before an entity-class contact-damage override.
#[derive(Clone,Debug)]
pub struct AttackDamageContext {
    pub entity:u32,
    pub entity_type:Identifier,
    /// The effective double-valued attribute, not the inherited class's bonus.
    pub attribute:f64,
}
/// Native class calculation after the living-fall event, before block effects.
#[derive(Clone,Debug)]
pub struct CalculateFallDamageContext {
    pub entity:u32,
    pub entity_type:Identifier,
    pub distance:f64,
    pub multiplier:f32,
    /// Inherited native class calculation, before block-specific alterations.
    pub inherited:f64,
}
/// Minecraft attribute modifier operations, before effective-value clamping.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(u8)]
pub enum AttributeModifierOperation {AddValue=0,AddMultipliedBase=1,AddMultipliedTotal=2}
/// A server-side class default modifier, never a permanent saved modifier.
#[derive(Clone,Debug,PartialEq)]
pub struct TransientAttributeModifier {
    pub attribute:Identifier,
    pub id:Identifier,
    pub amount:f64,
    pub operation:AttributeModifierOperation,
}
/// Owned base write requested after loading class data; zero is explicit.
#[derive(Clone,Debug,PartialEq)]
pub struct AttributeBaseValue {pub attribute:Identifier,pub value:f64}
/// The landing a class's own causeFallDamage decides on: it runs where the
/// inherited application would, after the living-fall event and the class
/// fall calculation, and replaces that application wholesale.
#[derive(Clone,Debug)]
pub struct CauseFallDamageContext {
    pub entity:u32,
    pub entity_type:Identifier,
    /// Fall distance at landing, after any living-fall event adjustment.
    pub distance:f64,
    /// The damage multiplier the landing carries.
    pub multiplier:f32,
    /// The entity's motion as it lands.
    pub velocity:[f64;3],
    /// Entity.isSuppressingBounce: a sneaking player; a mob never suppresses.
    pub suppressing_bounce:bool,
}
/// What a causeFallDamage override does instead of applying fall damage.
#[derive(Clone,Debug,PartialEq)]
pub enum FallOutcome {
    /// No damage and no landing effects beyond the block's own fallOn
    /// (the same shape as a cancelled living-fall event).
    Cancel,
    /// No damage; the entity rebounds with this motion and is airborne
    /// again: fall distance cleared, on-ground cleared, and `sound` played
    /// at the entity when present (a slime's bounce).
    Bounce {velocity:[f64;3],sound:Option<Identifier>},
}
/// The equipment drop chance one class computes for one slot, consulted
/// where the game rolls equipment drops on death. A returned value replaces
/// the recorded chance for that slot's roll only; the recorded chances of
/// the mob are not changed.
#[derive(Clone,Debug)]
pub struct EquipmentDropChanceContext {
    pub entity_type:Identifier,
    /// One of mainhand, offhand, head, chest, legs, feet, body, saddle.
    pub slot:String,
    /// Present only for native cube/slime implementations.
    pub slime_size:Option<i32>,
    /// The recorded chance, or the game's default for the slot.
    pub chance:f32,
}
/// Current server load context. Attachment methods on entity use the normal
/// callback scope; this is not a durable entity handle or a constructor event.
#[derive(Clone,Debug)]
pub struct LivingEntityLoadContext {pub entity:super::entity_events_v1::EntityInfo,pub slime_size:Option<i32>}
type LoadBases=Arc<dyn Fn(&LivingEntityLoadContext)->Result<Vec<AttributeBaseValue>,String>+Send+Sync>;
type Fall=Arc<dyn Fn(&CalculateFallDamageContext)->Option<i32>+Send+Sync>;
type Jump=Arc<dyn Fn(&JumpPowerContext)->Option<f32>+Send+Sync>;
type Attack=Arc<dyn Fn(&AttackDamageContext)->Option<f32>+Send+Sync>;
type CauseFall=Arc<dyn Fn(&CauseFallDamageContext)->Option<FallOutcome>+Send+Sync>;
type DropChance=Arc<dyn Fn(&EquipmentDropChanceContext)->Option<f32>+Send+Sync>;
/// Per-type living-entity query methods. This does not create an entity factory.
#[derive(Clone)]
pub struct LivingEntityBehavior { entity_type:Identifier,jump:Option<Jump>,attack:Option<Attack>,fall:Option<Fall>,cause_fall:Option<CauseFall>,drop_chance:Option<DropChance>,load_bases:Option<LoadBases>,attributes:Vec<TransientAttributeModifier> }
impl LivingEntityBehavior {
    pub fn new(entity_type:Identifier)->Self {Self {entity_type,jump:None,attack:None,fall:None,cause_fall:None,drop_chance:None,load_bases:None,attributes:Vec::new()}}
    /// Restore class-dependent bases before server entity-load observers and
    /// after successful in-place native Entity.load operations. The
    /// callback runs in an attachment-capable host scope, outside registry locks.
    pub fn attribute_bases_on_load(mut self,callback:impl Fn(&LivingEntityLoadContext)->Result<Vec<AttributeBaseValue>,String>+Send+Sync+'static)->Self {
        self.load_bases=Some(Arc::new(callback));self
    }
    /// Declare a server-side class modifier. Duplicate attribute/id pairs reject
    /// at registration; missing native attributes are skipped, not invented.
    pub fn transient_attribute_modifier(mut self,modifier:TransientAttributeModifier)->Self {
        self.attributes.push(modifier);self
    }
    /// Override getJumpPower(), not its multiplier overload or a post-jump event.
    pub fn jump_power(mut self,callback:impl Fn(&JumpPowerContext)->Option<f32>+Send+Sync+'static)->Self {
        self.jump=Some(Arc::new(callback));self
    }
    /// Override calculateFallDamage, not causeFallDamage or the fall event.
    pub fn calculate_fall_damage(mut self,callback:impl Fn(&CalculateFallDamageContext)->Option<i32>+Send+Sync+'static)->Self {
        self.fall=Some(Arc::new(callback));self
    }
    /// Override causeFallDamage itself: returning an outcome replaces the
    /// inherited fall-damage application for this landing; returning None
    /// keeps it. Distinct from calculate_fall_damage, which only rewrites
    /// the damage number inside the inherited application.
    pub fn cause_fall_damage(mut self,callback:impl Fn(&CauseFallDamageContext)->Option<FallOutcome>+Send+Sync+'static)->Self {
        self.cause_fall=Some(Arc::new(callback));self
    }
    /// The class's own per-slot equipment drop chance at death (e.g. a cap
    /// that only applies above a size). Returning None keeps the recorded
    /// chance for that slot.
    pub fn equipment_drop_chance(mut self,callback:impl Fn(&EquipmentDropChanceContext)->Option<f32>+Send+Sync+'static)->Self {
        self.drop_chance=Some(Arc::new(callback));self
    }
    /// Override the contact attack's class query, not every living melee attack.
    pub fn attack_damage(mut self,callback:impl Fn(&AttackDamageContext)->Option<f32>+Send+Sync+'static)->Self {
        self.attack=Some(Arc::new(callback));self
    }
}
static REGISTRY:Mutex<BTreeMap<String,LivingEntityBehavior>>=Mutex::new(BTreeMap::new());
/// General entity-class query registration. Callbacks execute outside its lock.
pub struct LivingEntityBehaviors;
impl LivingEntityBehaviors {
    /// Reject duplicate types without changing their existing callbacks.
    pub fn register(behavior:LivingEntityBehavior)->Result<(),String> {
        let mut seen=std::collections::BTreeSet::new();
        for modifier in &behavior.attributes {
            if !seen.insert((modifier.attribute.to_string(),modifier.id.to_string())) {
                return Err("duplicate class attribute modifier".into());
            }
        }
        let mut registry=REGISTRY.lock().map_err(|_|"living behaviour registry poisoned")?;
        let id=behavior.entity_type.to_string();
        if registry.contains_key(&id) {return Err(format!("living behaviour {id} already registered"));}
        registry.insert(id,behavior);Ok(())
    }
    /// Owned class declarations. Registry failure is not an empty policy.
    pub fn transient_attribute_modifiers(entity_type:&Identifier)->Result<Vec<TransientAttributeModifier>,String> {
        let registry=REGISTRY.lock().map_err(|_|"living behaviour registry poisoned")?;
        Ok(registry.get(&entity_type.to_string()).map_or_else(Vec::new,|b|b.attributes.clone()))
    }
    /// Run the load-time base callback without retaining the registry lock.
    /// Duplicate attributes reject the whole returned batch before application.
    pub fn attribute_bases_on_load(context:&LivingEntityLoadContext)->Result<Vec<AttributeBaseValue>,String> {
        let callback=REGISTRY.lock().map_err(|_|"living behaviour registry poisoned")?
            .get(&context.entity.kind).and_then(|b|b.load_bases.clone());
        let values=match callback {Some(callback)=>callback(context)?,None=>return Ok(Vec::new())};
        let mut seen=std::collections::BTreeSet::new();
        for value in &values {if !seen.insert(value.attribute.clone()) {return Err("duplicate class attribute base".into());}}
        Ok(values)
    }
    /// Dispatch the registered jump method. None preserves the inherited query.
    pub fn jump_power(context:&JumpPowerContext)->Option<f32> {
        let callback=REGISTRY.lock().ok()?.get(&context.entity_type.to_string())?.jump.clone()?;
        callback(context)
    }
    /// Dispatch the registered contact-damage method; zero is not a fallback.
    pub fn attack_damage(context:&AttackDamageContext)->Option<f32> {
        let callback=REGISTRY.lock().ok()?.get(&context.entity_type.to_string())?.attack.clone()?;
        callback(context)
    }
    /// Dispatch a class fall calculation; zero does not cancel the fall lifecycle.
    pub fn calculate_fall_damage(context:&CalculateFallDamageContext)->Option<i32> {
        let callback=REGISTRY.lock().ok()?.get(&context.entity_type.to_string())?.fall.clone()?;
        callback(context)
    }
    pub(crate) fn has_load_bases(kind:&str)->Result<bool,String> {
        Ok(REGISTRY.lock().map_err(|_|"living behaviour registry poisoned")?.get(kind).is_some_and(|b|b.load_bases.is_some()))
    }
    pub(crate) fn has_fall(kind:&str)->bool {
        REGISTRY.lock().ok().is_some_and(|r|r.get(kind).is_some_and(|b|b.fall.is_some()))
    }
    pub(crate) fn has_jump(kind:&str)->bool {
        REGISTRY.lock().ok().is_some_and(|r|r.get(kind).is_some_and(|b|b.jump.is_some()))
    }
    pub(crate) fn has_attack(kind:&str)->bool {
        REGISTRY.lock().ok().is_some_and(|r|r.get(kind).is_some_and(|b|b.attack.is_some()))
    }
    /// Dispatch the registered causeFallDamage override. None keeps the
    /// inherited application; the callback runs outside the registry lock.
    pub fn cause_fall_damage(context:&CauseFallDamageContext)->Option<FallOutcome> {
        let callback=REGISTRY.lock().ok()?.get(&context.entity_type.to_string())?.cause_fall.clone()?;
        callback(context)
    }
    pub(crate) fn has_cause_fall(kind:&str)->bool {
        REGISTRY.lock().ok().is_some_and(|r|r.get(kind).is_some_and(|b|b.cause_fall.is_some()))
    }
    /// Dispatch the registered per-slot drop chance; None keeps the recorded one.
    pub fn equipment_drop_chance(context:&EquipmentDropChanceContext)->Option<f32> {
        let callback=REGISTRY.lock().ok()?.get(&context.entity_type.to_string())?.drop_chance.clone()?;
        callback(context)
    }
    pub(crate) fn has_drop_chance(kind:&str)->bool {
        REGISTRY.lock().ok().is_some_and(|r|r.get(kind).is_some_and(|b|b.drop_chance.is_some()))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_callbacks_preserve_zero_decline_duplicates_and_unlocked_registration() {
        let id=Identifier::parse("living_query:subject").unwrap();
        LivingEntityBehaviors::register(LivingEntityBehavior::new(id.clone()).jump_power(|ctx| {
            LivingEntityBehaviors::register(LivingEntityBehavior::new(Identifier::parse("living_query:nested").unwrap())).unwrap();
            assert_eq!(ctx.entity,17);assert_eq!(ctx.slime_size,Some(4));
            Some(0.0)
        }).attack_damage(|ctx|(ctx.attribute==7.5).then_some(9.5))).unwrap();
        assert!(LivingEntityBehaviors::register(LivingEntityBehavior::new(id.clone())).is_err());
        assert_eq!(LivingEntityBehaviors::jump_power(&JumpPowerContext {entity:17,entity_type:id.clone(),slime_size:Some(4),block_jump_factor:0.5}),Some(0.0));
        let mut damage=AttackDamageContext {entity:17,entity_type:id,attribute:7.5};
        assert_eq!(LivingEntityBehaviors::attack_damage(&damage),Some(9.5));
        damage.attribute=3.0;assert_eq!(LivingEntityBehaviors::attack_damage(&damage),None);
        damage.entity_type=Identifier::parse("living_query:unregistered").unwrap();
        assert_eq!(LivingEntityBehaviors::attack_damage(&damage),None);
        let modifier=TransientAttributeModifier {attribute:Identifier::parse("minecraft:armor").unwrap(),
            id:Identifier::parse("living_query:bonus").unwrap(),amount:3.0,operation:AttributeModifierOperation::AddMultipliedTotal};
        assert!(LivingEntityBehaviors::register(LivingEntityBehavior::new(damage.entity_type.clone())
            .transient_attribute_modifier(modifier.clone()).transient_attribute_modifier(modifier)).is_err());
        assert!(LivingEntityBehaviors::transient_attribute_modifiers(&damage.entity_type).unwrap().is_empty());
        let load_id=Identifier::parse("living_query:loaded").unwrap();
        LivingEntityBehaviors::register(LivingEntityBehavior::new(load_id.clone()).attribute_bases_on_load(|context| {
            if context.slime_size==Some(1) {
                LivingEntityBehaviors::register(LivingEntityBehavior::new(Identifier::parse("living_query:load_nested").unwrap()))?;
            }
            let base=AttributeBaseValue {attribute:Identifier::parse("minecraft:armor").unwrap(),value:0.0};
            Ok(if context.slime_size==Some(2) {vec![base.clone(),base]} else {vec![base]})
        })).unwrap();
        let mut load=LivingEntityLoadContext {entity:super::super::entity_events_v1::EntityInfo {id:17,kind:load_id.to_string(),dimension:"minecraft:overworld".into()},slime_size:Some(1)};
        assert_eq!(LivingEntityBehaviors::attribute_bases_on_load(&load).unwrap()[0].value,0.0);
        load.slime_size=Some(2);assert!(LivingEntityBehaviors::attribute_bases_on_load(&load).is_err());
        load.entity.kind="living_query:absent".into();assert!(LivingEntityBehaviors::attribute_bases_on_load(&load).unwrap().is_empty());
    }
    #[test]
    fn cause_fall_and_drop_chance_dispatch_only_to_their_registrations() {
        let id=Identifier::parse("living_query:bouncer").unwrap();
        LivingEntityBehaviors::register(LivingEntityBehavior::new(id.clone())
            .cause_fall_damage(|ctx|(ctx.distance>2.0).then_some(FallOutcome::Bounce {velocity:[ctx.velocity[0]/0.95,-ctx.velocity[1]*0.9,ctx.velocity[2]/0.95],sound:None}))
            .equipment_drop_chance(|ctx|(ctx.slot=="head"&&ctx.slime_size.is_some_and(|s|s>1)&&ctx.chance>0.25).then_some(0.25))).unwrap();
        let mut fall=CauseFallDamageContext {entity:9,entity_type:id.clone(),distance:3.0,multiplier:1.0,velocity:[0.95,-1.0,0.0],suppressing_bounce:false};
        assert_eq!(LivingEntityBehaviors::cause_fall_damage(&fall),Some(FallOutcome::Bounce {velocity:[1.0,0.9,0.0],sound:None}));
        fall.distance=1.0;assert_eq!(LivingEntityBehaviors::cause_fall_damage(&fall),None);
        let mut chance=EquipmentDropChanceContext {entity_type:id.clone(),slot:"head".into(),slime_size:Some(4),chance:2.0};
        assert_eq!(LivingEntityBehaviors::equipment_drop_chance(&chance),Some(0.25));
        chance.slime_size=Some(1);assert_eq!(LivingEntityBehaviors::equipment_drop_chance(&chance),None);
        chance.entity_type=Identifier::parse("living_query:unregistered").unwrap();
        assert_eq!(LivingEntityBehaviors::equipment_drop_chance(&chance),None);
        assert_eq!(LivingEntityBehaviors::cause_fall_damage(&fall),None);
    }
}
