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

#[test] fn broad_overwield_divides_before_multiplying_coefficient() {
 let actual=run(0,1.0,&[Conditional::Overwield(2,8,true)],true);
 let source=3.0*(1.0+(8.0f32/5.0)*0.2);
 let old=3.0*(1.0+0.2f32*8.0/5.0);
 assert_eq!(actual.to_bits(),source.to_bits());
 assert_ne!(actual.to_bits(),old.to_bits());
}
#[test] fn overwield_caps_gating_and_narrow_tools_are_preserved() {
 for broad in [false,true] {
  let cost=if broad {5}else{1};
  for (level,overslime) in [(0,8),(2,-1),(2,0),(2,1),(2,4),(2,5),(2,6),(2,8),(2,10),(2,11),(2,100),(-1,8)] {
   let used=overslime.max(0).min(level.max(0)*cost);
   let expected=3.0*(1.0+(used as f32/cost as f32)*0.2);
   let modifiers=[Conditional::Overwield(level,overslime,broad)];
   assert_eq!(run(0,1.0,&modifiers,true).to_bits(),expected.to_bits());
   assert_eq!(run(0,1.0,&modifiers,false),3.0);
  }
 }
}

fn source_incremental(amount:i32,needed:i32)->f32 {
 let amount=amount.max(0);let remainder=amount%needed;
 let logical=amount/needed+i32::from(remainder!=0);
 if remainder==0 {logical as f32}else{logical as f32+(remainder-needed) as f32/needed as f32}
}
#[test] fn hydraulic_partial_levels_follow_source_logical_rounding() {
 for amount in [-1,0,1,18,35,36,37,71,72,73,36*3+1,i32::MAX] {
  for aqua in [false,true] {
   let level=source_incremental(amount,36);
   let expected=(0.04f32*if aqua {8.0}else{40.0})*level;
   let actual=conditional_speed_with_water(0.0,0,100,1.0,0.04,&[Conditional::Hydraulic(amount)],None,true,false,None,Some(aqua));
   assert_eq!(actual.to_bits(),expected.to_bits());
  }
 }
 let actual=conditional_speed_with_water(0.0,0,100,1.0,0.04,&[Conditional::Hydraulic(1)],None,true,false,None,Some(false));
 assert_ne!(actual.to_bits(),((0.04f32*40.0)*(1.0f32/36.0)).to_bits());
}
#[test] fn blasting_partial_levels_follow_source_logical_rounding() {
 for amount in [-1,0,1,10,19,20,21,39,40,41,20*3+1,i32::MAX] {
  let expected=(1.0f32*source_incremental(amount,20))*0.3;
  let actual=conditional_speed_with_water(0.0,0,100,1.0,0.3,&[Conditional::Blasting(amount)],None,true,false,Some(6.0),None);
  assert_eq!(actual.to_bits(),expected.to_bits());
 }
 let actual=conditional_speed_with_water(0.0,0,100,1.0,0.3,&[Conditional::Blasting(1)],None,true,false,Some(6.0),None);
 assert_ne!(actual.to_bits(),((1.0f32/20.0)*0.3).to_bits());
}
#[test] fn incremental_mining_context_gates_are_unchanged() {
 for effective in [false,true] {
  let both=[Conditional::Hydraulic(1),Conditional::Blasting(1)];
  assert_eq!(conditional_speed_with_water(3.0,0,100,1.0,1.0,&both,None,effective,false,None,None),3.0);
  if !effective {
   assert_eq!(conditional_speed_with_water(3.0,0,100,1.0,1.0,&both,None,effective,false,Some(6.0),Some(false)),3.0);
  }
 }
}
