#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-material-effects.rs"] mod tconstruct_source_material_effects;
#[test]
fn reinforced_partial_levels_dense_order_and_cause() {
 use tconstruct_source_material_effects::*;
 assert!((reinforced_probability(1)-0.0671875).abs()<0.000001);
 assert!((reinforced_probability(4)-0.25).abs()<0.000001);
 assert!((reinforced_probability(20)-0.75).abs()<0.000001);
 let mut draws=[0.1,0.9,0.1,0.9, 0.2,0.8].into_iter();
 assert_eq!(reinforced_wear(4,1,4,false,false,||draws.next().unwrap()),1);
 assert_eq!(reinforced_wear(2,0,20,false,true,||panic!("bypassed cause must not roll")),2);
 assert_eq!(reinforced_wear(2,0,20,true,false,||panic!("unbreakable must not roll")),2);
}
#[test]
fn source_cause_bypasses_reinforced_only_for_glowing() {
 use tconstruct_source_material_effects::reinforced_cause_bypass;
 assert!(!reinforced_cause_bypass(None));
 assert!(reinforced_cause_bypass(Some("tconstruct:glowing")));
 assert!(!reinforced_cause_bypass(Some("tconstruct:thorns")));
}
