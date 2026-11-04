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
