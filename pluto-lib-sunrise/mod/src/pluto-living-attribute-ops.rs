// Uses the native unsaved Instance modifier sets. The native scalar attribute
// store remains authoritative for knockback bases/permanent modifiers; only
// transient operations are borrowed from this instance map during computation.
use sunrise_client_lib::mobs::{Mob,Attr,Op};
use sunrise_world::attributes::{Modifier,Operation,entity::Catalog};
use std::sync::Arc;
use crate::pluto_living_entity_access::AttributeModifier;

fn scalar_has(mob:&Mob,attribute:Attr,id:&str)->bool {
    mob.extra.attributes.as_ref().is_some_and(|s|s.modifiers.iter().any(|m|m.attr==attribute && m.id==id))
}
fn supported(attribute:&str)->bool {
    matches!(attribute,"minecraft:armor"|"minecraft:armor_toughness"|"minecraft:knockback_resistance")
}
pub fn initialize(mob:&mut Mob,catalog:&Arc<Catalog>,declarations:Vec<AttributeModifier>)->Result<(),String> {
    if declarations.is_empty() {return Ok(());}
    // Resolve keys against the map that actually owns them, never another
    // catalog's numeric indexes (including after a retained-map resource reload).
    let catalog=mob.extra.armor_attributes.as_ref().map(|m|m.catalog().clone()).unwrap_or_else(||catalog.clone());
    let mut prepared=Vec::new();let mut seen=std::collections::BTreeSet::new();
    for declaration in declarations {
        if !supported(&declaration.attribute) {return Err(format!("native transient attribute consumer unavailable: {}",declaration.attribute));}
        if !seen.insert((declaration.attribute.clone(),declaration.id.clone())) {return Err("duplicate native class modifier".into());}
        let operation=match declaration.operation {0=>Operation::AddValue,1=>Operation::AddMultipliedBase,2=>Operation::AddMultipliedTotal,
            _=>return Err("invalid native attribute operation".into())};
        let key=catalog.id(&declaration.attribute).ok_or_else(||format!("unregistered attribute {}",declaration.attribute))?;
        let modifier=Modifier::new(&declaration.id,declaration.amount,operation).map_err(|e|e.to_string())?;
        prepared.push((key,declaration.attribute,modifier));
    }
    if mob.extra.armor_attributes.is_none() {
        mob.extra.armor_attributes=Some(Box::new(catalog.create(&mob.kind.id).ok_or("missing entity attribute supplier")?));
        // A pre-join size/class write may precede creation of this map. Only
        // migrate an explicit scalar base, never overwrite a restored map.
        if let Some(base)=mob.extra.attributes.as_ref().and_then(|s|s.bases.iter().find(|(a,_)|*a==Attr::Armor)).map(|(_,v)|*v) {
            mirror_armor_base(mob,Attr::Armor,base);
        }
    }
    prepared.retain(|(_,attribute,modifier)|attribute!="minecraft:knockback_resistance"
        || !scalar_has(mob,Attr::KnockbackResistance,modifier.id()));
    let map=mob.extra.armor_attributes.as_mut().unwrap();
    let mut resistance=false;
    for (key,attribute,modifier) in prepared {
        // Mirrors tryAddAttribute: do not manufacture a missing attribute.
        // Keep any loaded same-ID modifier rather than replacing saved state.
        if !map.has(key) || map.has_modifier(key,modifier.id()) {continue;}
        map.edit(key,|instance,observer|instance.add_transient(modifier,observer))
            .ok_or("missing attribute instance")?.map_err(|e|format!("{e:?}"))?;
        resistance|=attribute=="minecraft:knockback_resistance";
    }
    if resistance {
        sunrise_client_lib::mobs::refresh(mob,Attr::KnockbackResistance);
        mob.gear_knockback_resistance=None;
    }
    Ok(())
}
/// Validate a complete load batch before mutating any attribute instance.
/// Only the equipment-aware armor/toughness consumers are exposed here; other
/// attributes reject instead of writing a map their gameplay does not read.
pub fn apply_load_bases(mob:&mut Mob,catalog:&Arc<Catalog>,values:Vec<(String,f64)>)->Result<(),String> {
    if values.is_empty() {return Ok(());}
    let catalog=mob.extra.armor_attributes.as_ref().map(|m|m.catalog().clone()).unwrap_or_else(||catalog.clone());
    let mut seen=std::collections::BTreeSet::new();let mut prepared=Vec::new();
    for (name,value) in values {
        if !matches!(name.as_str(),"minecraft:armor"|"minecraft:armor_toughness") {
            return Err(format!("native class base consumer unavailable: {name}"));
        }
        if !seen.insert(name.clone()) {return Err("duplicate native class attribute base".into());}
        prepared.push((catalog.id(&name).ok_or_else(||format!("unregistered attribute {name}"))?,value));
    }
    if mob.extra.armor_attributes.is_none() {
        mob.extra.armor_attributes=Some(Box::new(catalog.create(&mob.kind.id).ok_or("missing entity attribute supplier")?));
    }
    let map=mob.extra.armor_attributes.as_mut().unwrap();
    for (key,value) in prepared {
        if !map.has(key) {continue;}
        map.edit(key,|instance,observer|instance.set_base_value(value,observer)
            .expect("native class base observer is infallible"));
    }
    Ok(())
}
/// Mirror an explicit native set_base write into the equipment-aware armor
/// consumer. Do not create a map/attribute or replace its modifier sets. The
/// owning catalog supplies the key and its normal observer tracks the change.
/// Toughness has no scalar Attr variant; resistance stays scalar-authoritative.
pub fn mirror_armor_base(mob:&mut Mob,attribute:Attr,base:f64) {
    if attribute!=Attr::Armor {return;}
    let Some(map)=mob.extra.armor_attributes.as_mut() else {return};
    let Some(key)=map.catalog().id(attribute.id()) else {return};
    if !map.has(key) {return;}
    map.edit(key,|instance,observer|instance.set_base_value(base,observer)
        .expect("native armor base observer is infallible"));
}
/// Called at the scalar ops STORE, before effects/equipment and final clamping.
/// Keep native operation-bucket order, including the host's retained buckets.
pub fn append(mob:&Mob,attribute:Attr,mut ops:Vec<(f64,Op)>)->Vec<(f64,Op)> {
    if attribute!=Attr::KnockbackResistance {return ops;}
    let Some(map)=mob.extra.armor_attributes.as_deref() else {return ops};
    let Some(key)=map.catalog().id(attribute.id()) else {return ops};
    let Some(instance)=map.instance(key) else {return ops};
    for (operation,native) in [(Operation::AddValue,Op::AddValue),(Operation::AddMultipliedBase,Op::AddMultipliedBase),
        (Operation::AddMultipliedTotal,Op::AddMultipliedTotal)] {
        for modifier in instance.operation_modifiers(operation) {
            if !instance.permanent_modifiers().iter().any(|m|m.id()==modifier.id()) && !scalar_has(mob,attribute,modifier.id()) {
                ops.push((modifier.amount(),native));
            }
        }
    }
    ops
}
pub fn transient_amount(mob:&Mob,attribute:Attr,id:&str)->Option<f64> {
    if attribute!=Attr::KnockbackResistance || scalar_has(mob,attribute,id) {return None;}
    let map=mob.extra.armor_attributes.as_deref()?;
    let key=map.catalog().id(attribute.id())?;let instance=map.instance(key)?;
    if instance.permanent_modifiers().iter().any(|m|m.id()==id) {return None;}
    instance.modifier(id).map(Modifier::amount)
}
pub fn remove_transient(mob:&mut Mob,attribute:Attr,id:&str) {
    if attribute!=Attr::KnockbackResistance {return;}
    let mut removed=false;
    if let Some(map)=mob.extra.armor_attributes.as_mut() {
        if let Some(key)=map.catalog().id(attribute.id()) {
            if map.instance(key).is_some_and(|i|i.modifier(id).is_some() && !i.permanent_modifiers().iter().any(|m|m.id()==id)) {
                removed=map.edit(key,|i,o|i.remove(id,o)).is_some_and(|r|matches!(r,Ok(true)));
            }
        }
    }
    // The original remove_modifier caller refreshes its scalar value afterward.
    // No transient match must retain native behavior, including its gear cache.
    if removed {mob.gear_knockback_resistance=None;}
}
