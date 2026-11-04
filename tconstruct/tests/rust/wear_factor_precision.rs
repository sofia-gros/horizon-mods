#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-material-effects.rs"] mod effects;

// The outer bonus algebra is intentionally unchanged. These regressions isolate
// the pinned per-trait RPN factor, not full modifier order or damage projection.
fn factor(lost:u32,maximum:f32,level:i32,coefficient:f32)->f32 {
 let root=(lost as f32/maximum).sqrt();
 let leveled=root*level as f32;
 let scaled=leveled*coefficient;
 scaled+1.0
}
#[test]
fn jagged_multiplies_level_before_positive_coefficient() {
 let expected=3.0*(factor(8,3.0,5,0.005)-1.0);
 let actual=effects::wear_damage_bonus(3.0,8,3.0,5,0);
 let old=3.0*((1.0+5.0*((8.0f32/3.0).sqrt()*0.005))-1.0);
 assert_eq!(actual.to_bits(),expected.to_bits());
 assert_ne!(actual.to_bits(),old.to_bits());
}
#[test]
fn stonebound_multiplies_level_before_negative_coefficient() {
 let expected=3.0*(factor(2,3.0,5,-0.005)-1.0);
 let actual=effects::wear_damage_bonus(3.0,2,3.0,0,5);
 let old=3.0*((1.0-5.0*((2.0f32/3.0).sqrt()*0.005))-1.0);
 assert_eq!(actual.to_bits(),expected.to_bits());
 assert_ne!(actual.to_bits(),old.to_bits());
}
#[test]
fn supported_factor_combinations_keep_existing_outer_algebra() {
 for (lost,maximum) in [(0,1.0),(1,0.1),(8,3.0),(100,1.0)] {
  for (jagged,stonebound) in [(0,0),(1,0),(0,1),(5,5)] {
   let expected=3.0*(factor(lost,maximum,jagged,0.005)*factor(lost,maximum,stonebound,-0.005)-1.0);
   assert_eq!(effects::wear_damage_bonus(3.0,lost,maximum,jagged,stonebound).to_bits(),expected.to_bits());
  }
 }
}
