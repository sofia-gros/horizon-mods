#[path = "../../mixins/tconstruct/source-dwarven.rs"] mod tconstruct_source_dwarven;
#[path = "../../mixins/tconstruct/source-mining-math.rs"] mod mining;
use mining::{Conditional,conditional_speed_with_water};
fn run(loss:u32,multiplier:f32,modifiers:&[Conditional],effective:bool)->f32 {
 conditional_speed_with_water(3.0,loss,100,multiplier,1.0,modifiers,None,effective,false,None,None)
}
fn source_factor(loss:u32,multiplier:f32,level:i32,coefficient:f32)->f32 {
 let root=(loss as f32/multiplier.max(f32::MIN_POSITIVE)).sqrt();
 let leveled=root*level.max(0) as f32;
 let scaled=leveled*coefficient;
 scaled+1.0
}
#[test] fn jagged_negative_coefficient_follows_level() {
 let actual=run(1,0.1,&[Conditional::Jagged(5)],true);
 let source=source_factor(1,0.1,5,-0.01)*3.0;
 let old=3.0*(1.0-(1.0f32/0.1).sqrt()*0.01*5.0);
 assert_eq!(actual.to_bits(),source.to_bits());
 assert_ne!(actual.to_bits(),old.to_bits());
}
#[test] fn stonebound_positive_coefficient_follows_level() {
 let actual=run(2,3.0,&[Conditional::Stonebound(5)],true);
 let source=source_factor(2,3.0,5,0.01)*3.0;
 let old=3.0*(1.0+(2.0f32/3.0).sqrt()*0.01*5.0);
 assert_eq!(actual.to_bits(),source.to_bits());
 assert_ne!(actual.to_bits(),old.to_bits());
}
#[test] fn sequential_effects_retain_supplied_order_and_effectiveness() {
 for order in [[Conditional::Jagged(5),Conditional::Stonebound(3)],[Conditional::Stonebound(3),Conditional::Jagged(5)]] {
  let mut expected=3.0;
  for modifier in order {
   expected*=match modifier {Conditional::Jagged(level)=>source_factor(1,0.1,level,-0.01),Conditional::Stonebound(level)=>source_factor(1,0.1,level,0.01),_=>unreachable!()};
  }
  assert_eq!(run(1,0.1,&order,true).to_bits(),expected.max(0.0).to_bits());
  assert_eq!(run(1,0.1,&order,false),3.0);
 }
}
#[test] fn invalid_durability_floor_and_zero_wear_remain_unchanged() {
 for maximum in [0.0,-1.0,f32::NAN,f32::MIN_POSITIVE] {
  assert_eq!(run(0,maximum,&[Conditional::Jagged(5),Conditional::Stonebound(5)],true),3.0);
  assert_eq!(run(1,maximum,&[Conditional::Jagged(1)],true),0.0);
 }
 assert_eq!(run(100,1.0,&[Conditional::Jagged(-1),Conditional::Stonebound(0)],true),3.0);
}
