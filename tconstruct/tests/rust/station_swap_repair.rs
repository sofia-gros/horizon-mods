#[path = "../../mixins/tconstruct/source-station-stats.rs"] mod tconstruct_source_station_stats;
#[path = "../../mixins/tconstruct/source-material-effects.rs"] mod tconstruct_source_material_effects;
use std::collections::BTreeMap;
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
fn stats(kind: &str, durability: f64) -> BTreeMap<String,serde_json::Map<String,serde_json::Value>> {
    BTreeMap::from([(format!("tconstruct:{kind}"),serde_json::json!({"durability":durability}).as_object().unwrap().clone())])
}
fn tool(upgrades: Tag, order: Option<Vec<&str>>) -> ItemStack {
    let mut t=ItemStack::new("tconstruct:pickaxe",1);
    let mut fields=vec![("tic_upgrades",upgrades)];
    if let Some(order)=order {fields.push(("tic_upgrade_order",Tag::List(order.into_iter().map(|id|Tag::String(id.into())).collect())));}
    t.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(fields)));t
}
#[test] fn repairable_part_kinds_and_integer_stat() {
    use tconstruct_source_station_stats::part_swap_repair_base as base;
    for kind in ["head","limb","plating_helmet","plating_chestplate","plating_leggings","plating_boots"] {
        assert_eq!(base(&stats(kind,125.9),kind,2),Some(250.0/3.0));
    }
    for kind in ["handle","grip","binding","maille","bowstring","arrow_head"] {assert_eq!(base(&stats(kind,999.0),kind,2),Some(0.0));}
    assert_eq!(base(&stats("head",1.0),"head",1),Some(1.0/3.0));
    assert_eq!(base(&stats("head",0.0),"head",1),None);
    assert_eq!(base(&stats("head",i32::MAX as f64),"head",2),None);
    assert_eq!(base(&stats("head",100.0),"head",-1),None);
    assert_eq!(base(&BTreeMap::new(),"head",1),None);
    assert_eq!(base(&BTreeMap::new(),"head",0),Some(0.0));
}
#[test] fn new_traits_and_ordered_upgrade_hooks_precede_truncation() {
    use tconstruct_source_material_effects::ordered_known_repair_amount as repair;
    let t=tool(Tag::compound([("tconstruct:emerald",Tag::Int(1))]),None);
    let new=BTreeMap::from([("tconstruct:cultivated".into(),1),("tconstruct:preserved".into(),1)]);
    let order=vec!["tconstruct:cultivated".into(),"tconstruct:preserved".into()];
    let initial=tconstruct_source_station_stats::part_swap_repair_base(&stats("limb",125.0),"limb",2).unwrap();
    assert_eq!(repair(&t,&new,&order,initial).unwrap().to_bits(),((initial*1.5)*1.5*1.15f32).to_bits());
    let new=BTreeMap::from([("tconstruct:cultivated".into(),4)]);
    let amount=repair(&t,&new,&["tconstruct:cultivated".into()],1.0/3.0).unwrap();
    assert_eq!(amount as i32,1); // truncating the raw amount first would yield zero
    let dense=BTreeMap::from([("tconstruct:dense".into(),10)]);
    assert_eq!(repair(&t,&dense,&["tconstruct:dense".into()],initial),Some(0.0));
    assert!(repair(&t,&dense,&[],initial).is_none());
}
#[test] fn ambiguous_upgrade_order_refuses_repair() {
    let t=tool(Tag::compound([("tconstruct:emerald",Tag::Int(1)),("tconstruct:preserved",Tag::Int(1))]),None);
    assert!(tconstruct_source_material_effects::ordered_known_repair_amount(&t,&BTreeMap::new(),&[],100.0).is_none());
    let t=tool(Tag::Int(1),None);
    assert!(tconstruct_source_material_effects::ordered_known_repair_amount(&t,&BTreeMap::new(),&[],100.0).is_none());
}
#[test] fn rebuilt_damage_broken_flag_and_final_truncation() {
    use tconstruct_source_material_effects::finish_station_swap_repair as finish;
    let mut t=ItemStack::new("tconstruct:pickaxe",1);
    for (raw,max,broken,repair,expected,flag) in [
        (100,100,false,10.0,90,0), // old150 was already clamped at rebuild
        (85,200,true,10.0,190,0), // broken before larger replacement
        (0,200,true,10.0,190,0), // saved flag-only broken
        (85,200,true,0.0,85,1), // Dense blocks repair, don't unbreak
        (85,200,true,0.9,85,1), // positive raw repair truncates to zero
        (9,100,false,0.9,9,0), // same material is accepted before truncation
        (9,100,false,1000.0,0,0),
    ] {
        t.damage=raw;
        t.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(max)),("keep",Tag::Int(42))])));
        finish(&mut t,broken,repair).unwrap();
        assert_eq!(t.damage,expected);
        let c=t.data().unwrap().other("minecraft:custom_data").unwrap();
        assert!(matches!(c.get("tic_broken"),Some(Tag::Byte(v)) if *v==flag));
        assert!(matches!(c.get("keep"),Some(Tag::Int(42))));
    }
    assert!(finish(&mut t,false,f32::NAN).is_none());
}
