#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-material-effects.rs"] mod tconstruct_source_material_effects;
#[path = "../../mixins/tconstruct/source-overslime.rs"] mod tconstruct_source_overslime;
#[test]
fn tanned_limits_primary_loss_before_overslime_and_reinforced() {
 use tconstruct_source_material_effects::*;
 let primary=tanned_wear(8,1,None);
 let (shield, residual)=tconstruct_source_overslime::absorb(3,primary,None);
 assert_eq!((shield,residual),(2,0));
 assert_eq!(reinforced_wear(residual,0,0,false,None,||0.5),0);
}
#[test]
fn reinforced_partial_levels_dense_order_and_cause() {
 use tconstruct_source_material_effects::*;
 assert!((reinforced_probability(1)-0.0671875).abs()<0.000001);
 assert!((reinforced_probability(4)-0.25).abs()<0.000001);
 assert!((reinforced_probability(20)-0.75).abs()<0.000001);
 let mut draws=[0.1,0.9,0.1,0.9, 0.2,0.8].into_iter();
 assert_eq!(reinforced_wear(4,1,4,false,None,||draws.next().unwrap()),1);
 assert_eq!(reinforced_wear(2,0,20,false,Some("tconstruct:glowing"),||panic!("bypassed cause must not roll")),2);
 assert_eq!(reinforced_wear(2,1,20,true,None,||panic!("unbreakable must not roll")),0);
 assert_eq!(reinforced_wear(2,0,20,true,Some("tconstruct:glowing"),||panic!("bypassed cause must not roll")),2);
}
#[test]
fn source_cause_bypasses_reinforced_only_for_glowing() {
 use tconstruct_source_material_effects::reinforced_cause_bypass;
 assert!(!reinforced_cause_bypass(None));
 assert!(reinforced_cause_bypass(Some("tconstruct:glowing")));
 assert!(!reinforced_cause_bypass(Some("tconstruct:thorns")));
}

#[test]
fn glowing_cause_crosses_tanned_overslime_unbreakable_and_reinforced_in_source_order() {
 use tconstruct_source_material_effects::*;
 let cause = Some("tconstruct:glowing");
 for (tanned, expected) in [(0,5),(1,1)] {
  let loss=tanned_wear(5,tanned,cause);
  let (shield,residual)=tconstruct_source_overslime::absorb(30,loss,cause);
  assert_eq!(shield,30);
  assert_eq!(reinforced_wear(residual,0,20,true,cause,||panic!("bypass must not roll")),expected);
 }
 let loss=tanned_wear(5,0,None);
 let (shield,residual)=tconstruct_source_overslime::absorb(30,loss,None);
 assert_eq!((shield,residual),(25,0));
 assert_eq!(reinforced_wear(residual,0,20,true,None,||panic!("unbreakable must not roll")),0);
}
