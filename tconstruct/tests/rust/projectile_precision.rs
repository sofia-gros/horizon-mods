// Execute real helpers linked against Horizon item/world libraries.
extern crate horizon_items as horizon_client;
#[path = "../../mixins/tconstruct/source-dwarven.rs"] mod tconstruct_source_dwarven;
#[path = "../../mixins/tconstruct/source-projectile-stats.rs"] mod projectile;
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;
fn bow(value:f32,raging:i32,ambush:i32,multiplier:f32)->ItemStack {
 let mut bow=ItemStack::new("tconstruct:longbow",1);
 bow.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
 ("tic_projectile_damage",Tag::Float(value.to_bits())),("tic_raging_projectile_multiplier",Tag::Float(multiplier.to_bits())),
 ("tic_raging",Tag::Int(raging)),("tic_ambush_projectile",Tag::Int(ambush))])));bow
}
#[test] fn inactive_conditions_preserve_fractional_stat_and_base_two_arrow() {
 for id in ["tconstruct:longbow","tconstruct:crossbow"] {
 for value in [0.1f32,0.3,0.7,1.1,1.9,2.3,10.1,2.0f32.powi(-20)] {
  let mut bow=bow(value,1,1,1.0);bow.item=id.into();let result=projectile::projectile_bonus_at(&bow,Some(20.0),Some(40.0));
  assert_eq!(result.to_bits(),projectile::projectile_bonus(&bow).to_bits());
  assert_eq!((2.0+result).to_bits(),f64::from(value).to_bits());
 }
 }
}
#[test] fn conditionals_use_original_float_and_widen_before_delta() {
 let value=0.3f32;let multiplier=0.3f32;let health=1.01f32;
 let expected=(value+((11.0f32-health)/10.0).clamp(0.0,1.0)*multiplier+0.5f32*multiplier).clamp(0.0,1024.0);
 let result=projectile::projectile_bonus_at(&bow(value,1,1,multiplier),Some(health),Some(2.0));
 assert_eq!(result.to_bits(),(f64::from(expected)-2.0).to_bits());
 assert_eq!((2.0+result).to_bits(),f64::from(expected).to_bits());
}
#[test] fn fallback_context_and_clamps_unchanged() {
 let mut absent=ItemStack::new("tconstruct:longbow",1);
 absent.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
 ("tic_raging_projectile_multiplier",Tag::Float(1.0f32.to_bits())),
 ("tic_upgrades",Tag::compound([("tconstruct:diamond",Tag::Int(1))]))])));
 assert_eq!(projectile::projectile_bonus_at(&absent,Some(20.0),Some(20.0)),0.5);
 let value=bow(0.3,1,1,1.0);
 assert_eq!(projectile::projectile_bonus_at(&value,None,Some(20.0)),projectile::projectile_bonus(&value));
 assert_eq!(projectile::projectile_bonus_at(&value,Some(f32::NAN),Some(20.0)),projectile::projectile_bonus(&value));
 assert_eq!(projectile::projectile_bonus_at(&bow(-1.0,0,0,1.0),Some(20.0),None),-2.0);
 assert_eq!(projectile::projectile_bonus_at(&bow(1024.0,1,1,1.0),Some(1.0),Some(1.0)),1022.0);
}
#[test] fn tiny_damage_delta_transport_limit_remains_documented() {
 for value in [1e-30f32,f32::from_bits(1)] {
  let result=projectile::projectile_bonus_at(&bow(value,0,0,1.0),Some(20.0),None);
  // The existing two-plus-delta interface still loses tiny/subnormal values.
  assert_eq!(result,-2.0);assert_eq!(2.0+result,0.0);assert!(f64::from(value)>0.0);
 }
}
