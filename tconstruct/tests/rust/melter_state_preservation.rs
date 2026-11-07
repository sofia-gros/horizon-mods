#[path = "../../mixins/tconstruct/source-melter-state.rs"] mod tconstruct_source_melter_state;
use horizon_world::nbt::Tag;
use tconstruct_source_melter_state::{Melter,Tank};
#[test] fn tick_save_keeps_machine_and_tank_extension_fields() {
 let mut m=Melter::new(); assert!(m.tank.insert("x:copper",90)); m.fuel=4;
 let mut raw=m.to_nbt();
 if let Tag::Compound(fields)=&mut raw { fields.insert("extension".into(),Tag::Int(11));
  if let Some(Tag::Compound(tank))=fields.get_mut(&"tank".into()) {tank.insert("extension".into(),Tag::Int(12));} }
 let mut m=Melter::from_nbt(&raw).unwrap(); assert_eq!(m.tank.take(90).as_deref(),Some("x:copper")); m.fuel-=1; m.phase=1;
 let saved=m.to_nbt_preserving(&raw).unwrap();
 assert_eq!(saved.get("extension"),Some(&Tag::Int(11))); let tank=saved.get("tank").unwrap();
 assert_eq!(tank.get("extension"),Some(&Tag::Int(12))); assert!(tank.get("fluid").is_none()); assert!(tank.get("fluid_tag").is_none());
 let round=Melter::from_nbt(&saved).unwrap(); assert_eq!((round.fuel,round.phase,round.tank.amount),(3,1,0));
}
#[test] fn fuel_tank_save_retains_extensions_and_rejects_bad_original() {
 let mut tank=Tank::new(4000); assert!(tank.insert("minecraft:lava",100)); let mut raw=tank.to_nbt();
 if let Tag::Compound(fields)=&mut raw {fields.insert("extension".into(),Tag::String("keep".into()));}
 assert!(tank.take(50).is_some()); let saved=tank.to_nbt_preserving(&raw).unwrap();
 assert_eq!(saved.get("extension"),Some(&Tag::String("keep".into()))); assert_eq!(Tank::from_nbt(&saved,4000).unwrap().amount,50);
 assert!(tank.to_nbt_preserving(&Tag::Int(1)).is_none());
}

#[test] fn lossy_identifier_cannot_be_rewritten_by_preserving_save() {
 let bad=Tag::compound([("fluid",Tag::String(horizon_world::nbt::JavaString(vec![0xd800]))),("amount",Tag::Int(10)),("extension",Tag::Int(7))]);
 let tank=Tank::from_nbt(&bad,4000).unwrap(); let before=bad.clone();
 assert!(tank.to_nbt_preserving(&bad).is_none()); assert_eq!(bad,before);
 let mut melter=Melter::new(); assert!(melter.tank.insert("minecraft:water",10));
 let mut machine=melter.to_nbt();
 if let Tag::Compound(fields)=&mut machine { fields.insert("tank".into(),bad); }
 let decoded=Melter::from_nbt(&machine).unwrap(); assert!(decoded.to_nbt_preserving(&machine).is_none());
 let good=Tag::compound([("fluid",Tag::String("test:😀".into())),("amount",Tag::Int(1))]);
 let tank=Tank::from_nbt(&good,4000).unwrap(); assert_eq!(tank.to_nbt_preserving(&good),Some(good));
}

#[test] fn fuel_update_preserves_native_extensions_and_refuses_aliases() {
 let mut tank=Tank::new(4000); assert!(tank.insert("minecraft:lava",1000));
 let mut raw=tank.to_nbt(); if let Tag::Compound(f)=&mut raw {f.insert("future".into(),Tag::Int(7));}
 assert!(tank.take(50).is_some()); let out=tank.to_nbt_preserving_fuel(&raw).unwrap();
 assert_eq!(out.get("future"),Some(&Tag::Int(7))); assert_eq!(out.get("amount"),Some(&Tag::Int(950)));
 for key in ["FluidName","Amount","Tag","Tank","Fluid"] {
  let mut aliased=raw.clone();if let Tag::Compound(fields)=&mut aliased{fields.insert(key.into(),Tag::Int(9));}
  let before=aliased.clone();assert!(tank.to_nbt_preserving_fuel(&aliased).is_none());assert_eq!(aliased,before);
 }
 let bad=Tag::compound([("fluid",Tag::String(horizon_world::nbt::JavaString(vec![0xd800]))),("amount",Tag::Int(1))]);
 let decoded=Tank::from_nbt(&bad,4000).unwrap(); assert!(decoded.to_nbt_preserving_fuel(&bad).is_none());
 tank.amount=4001;assert!(tank.to_nbt_preserving_fuel(&raw).is_none());
}
#[test] fn fuel_debit_preserves_raw_data_through_partial_and_final_consumption() {
 for capacity in [4000,4320]{let mut tank=Tank::new(capacity);assert!(tank.insert("minecraft:lava",100));
  let mut raw=tank.to_nbt();if let Tag::Compound(fields)=&mut raw{fields.insert("future".into(),Tag::Int(7));}
  let before=raw.clone();let partial=Tank::drain_fuel_preserving(&raw,capacity,30).unwrap();assert_eq!(raw,before);
  assert_eq!(partial.get("amount"),Some(&Tag::Int(70)));assert_eq!(partial.get("future"),Some(&Tag::Int(7)));
  let empty=Tank::drain_fuel_preserving(&partial,capacity,70).unwrap();assert_eq!(empty.get("amount"),Some(&Tag::Int(0)));
  assert!(empty.get("fluid").is_none());assert!(empty.get("fluid_tag").is_none());assert_eq!(empty.get("future"),Some(&Tag::Int(7)));
  assert!(Tank::drain_fuel_preserving(&empty,capacity,1).is_none());
 }
}
#[test] fn fuel_debit_refuses_ambiguous_or_unsupported_inputs_without_mutation() {
 let mut tank=Tank::new(4000);assert!(tank.insert("minecraft:lava",100));let raw=tank.to_nbt();
 for mode in 0..12{let mut bad=raw.clone();let mut amount=10;
  if let Tag::Compound(fields)=&mut bad{match mode{
   0..=4=>{fields.insert(["FluidName","Amount","Tag","Tank","Fluid"][mode].into(),Tag::Int(9));},
   5=>{fields.insert("fluid".into(),Tag::String(horizon_world::nbt::JavaString(vec![0xd800])));},
   6=>{fields.insert("amount".into(),Tag::String("bad".into()));},
   7=>{fields.insert("amount".into(),Tag::Int(4001));},
   8=>{fields.insert("fluid_tag".into(),Tag::compound([("Potion",Tag::String("minecraft:healing".into()))]));},
   9=>amount=0,10=>amount=101,
   11=>{fields.insert("fluid_tag".into(),Tag::Int(1));},
   _=>unreachable!(),
  }}
  let before=bad.clone();assert!(Tank::drain_fuel_preserving(&bad,4000,amount).is_none());assert_eq!(bad,before);
 }
}
