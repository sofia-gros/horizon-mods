//! Actual attribute helpers against storage facades, including scalar arithmetic.
//! Native types, save/sync, catalog implementation, world join and attachment are not exercised.
use std::{collections::BTreeMap,sync::Arc};
use crate::mobs::pluto_living_entity_access::AttributeModifier;
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
enum Attr {KnockbackResistance,Armor}
impl Attr {fn id(self)->&'static str {match self {Self::KnockbackResistance=>"minecraft:knockback_resistance",Self::Armor=>"minecraft:armor"}}}
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
enum Op {AddValue,AddMultipliedBase,AddMultipliedTotal}
mod sunrise_world {pub mod attributes {
    use super::super::*;
    #[derive(Clone,Copy,PartialEq,Eq,Debug)]
    pub enum Operation {AddValue,AddMultipliedBase,AddMultipliedTotal}
    #[derive(Clone,Debug)]
    pub struct Modifier {id:String,amount:f64,operation:Operation}
    impl Modifier {
        pub fn new(id:&str,amount:f64,operation:Operation)->Result<Self,String> {Ok(Self {id:id.into(),amount,operation})}
        pub fn id(&self)->&str {&self.id}
        pub fn amount(&self)->f64 {self.amount}
    }
    #[derive(Default)]
    pub struct Instance {modifiers:Vec<Modifier>,permanent:Vec<Modifier>,base:f64,base_writes:usize}
    impl Instance {
        pub fn base_value(&self)->f64 {self.base}
        pub fn base_writes(&self)->usize {self.base_writes}
        pub fn set_base_value(&mut self,value:f64,_:&mut ())->Result<(),String> {self.base=value;self.base_writes+=1;Ok(())}
        pub fn modifier(&self,id:&str)->Option<&Modifier> {self.modifiers.iter().find(|m|m.id()==id)}
        pub fn permanent_modifiers(&self)->&[Modifier] {&self.permanent}
        pub fn operation_modifiers(&self,op:Operation)->impl Iterator<Item=&Modifier> {self.modifiers.iter().filter(move|m|m.operation==op)}
        pub fn add_transient(&mut self,m:Modifier,_:&mut ())->Result<(),String> {
            if self.modifier(m.id()).is_some() {return Err("duplicate".into());}self.modifiers.push(m);Ok(())
        }
        pub fn remove(&mut self,id:&str,_:&mut ())->Result<bool,String> {
            let before=self.modifiers.len();self.modifiers.retain(|m|m.id()!=id);Ok(before!=self.modifiers.len())
        }
    }
    pub mod entity {
        use super::*;
        pub struct Catalog {pub keys:BTreeMap<String,usize>}
        impl Catalog {
            pub fn id(&self,name:&str)->Option<usize> {self.keys.get(name).copied()}
            pub fn create(self:&Arc<Self>,kind:&str)->Option<EntityAttributes> {
                (kind=="attributes:subject").then(||EntityAttributes {catalog:self.clone(),instances:BTreeMap::new()})
            }
        }
        pub struct EntityAttributes {catalog:Arc<Catalog>,instances:BTreeMap<usize,Instance>}
        impl EntityAttributes {
            pub fn catalog(&self)->&Arc<Catalog> {&self.catalog}
            pub fn has(&self,key:usize)->bool {self.catalog.keys.values().any(|k|*k==key)}
            pub fn instance(&self,key:usize)->Option<&Instance> {self.instances.get(&key)}
            pub fn has_modifier(&self,key:usize,id:&str)->bool {self.instance(key).is_some_and(|i|i.modifier(id).is_some())}
            pub fn edit<T>(&mut self,key:usize,f:impl FnOnce(&mut Instance,&mut ())->T)->Option<T> {
                if !self.has(key) {return None;}Some(f(self.instances.entry(key).or_default(),&mut ()))
            }
        }
    }
}}
struct Kind {id:String}
struct ScalarModifier {attr:Attr,id:String,amount:f64,op:Op}
struct ScalarAttributes {modifiers:Vec<ScalarModifier>,bases:Vec<(Attr,f64)>}
struct Extra {armor_attributes:Option<Box<sunrise_world::attributes::entity::EntityAttributes>>,attributes:Option<ScalarAttributes>}
struct Mob {kind:Kind,extra:Extra,gear_knockback_resistance:Option<f64>,refreshes:usize}
fn refresh(m:&mut Mob,a:Attr) {assert_eq!(a,Attr::KnockbackResistance);m.refreshes+=1;}
mod ops {
    use super::sunrise_world;
    include!("../../../mod/pluto-living-attribute-ops.rs");
}
// These functions supply host inputs, not a second copy of the calculation.
mod compute_head {
    use super::{Mob,Attr,Op,ops as pluto_living_attribute_ops};
    impl Attr {
        fn range(self)->(f64,f64) {match self {Self::KnockbackResistance=>(0.0,1.0),Self::Armor=>(0.0,30.0)}}
    }
    fn base_value(m:&Mob,a:Attr)->f64 {
        m.extra.attributes.as_ref().unwrap().bases.iter().find(|(attr,_)|*attr==a).unwrap().1
    }
    fn transient(_m:&Mob,_a:Attr)->Vec<(f64,Op)> {vec![(0.1,Op::AddValue)]}
    fn effect_modifiers(_m:&Mob,_a:Attr)->Vec<(f64,Op)> {vec![(0.25,Op::AddMultipliedTotal)]}
    include!("../../../mod/pluto-living-attributes-compute.rs");
    pub(super) fn compute(m:&Mob,transient:bool,extra:&[(f64,Op)])->std::ops::ControlFlow<f64> {
        pluto_living_attributes_compute(m,Attr::KnockbackResistance,transient,extra)
    }
}
#[test]
fn actual_scalar_head_retains_native_fallback_operation_order_transient_switch_and_clamp() {
    use std::ops::ControlFlow;
    use sunrise_world::attributes::entity::Catalog;
    let catalog=Arc::new(Catalog {keys:[("minecraft:knockback_resistance".into(),0)].into()});
    let mut mob=Mob {kind:Kind {id:"attributes:subject".into()},extra:Extra {armor_attributes:None,
        attributes:Some(ScalarAttributes {modifiers:vec![ScalarModifier {attr:Attr::KnockbackResistance,
            id:"native:permanent".into(),amount:0.1,op:Op::AddValue}],bases:vec![(Attr::KnockbackResistance,0.1)]})},
        gear_knockback_resistance:None,refreshes:0};
    assert_eq!(compute_head::compute(&mob,true,&[]),ControlFlow::Continue(()));
    ops::initialize(&mut mob,&catalog,vec![AttributeModifier {attribute:"minecraft:knockback_resistance".into(),
        id:"attributes:class".into(),amount:0.1,operation:0}]).unwrap();
    // Permanent + class addition precede multiplied-base; native transient and
    // effect inputs are included only when requested. Production helper runs.
    for (transient,expected) in [(false,0.45),(true,0.75)] {
        let ControlFlow::Break(value)=compute_head::compute(&mob,transient,&[(0.5,Op::AddMultipliedBase)]) else {panic!("class modifier omitted")};
        assert!((value-expected).abs()<1e-12);
    }
    assert_eq!(compute_head::compute(&mob,false,&[(f64::NAN,Op::AddValue)]),ControlFlow::Break(0.0));
    assert_eq!(compute_head::compute(&mob,false,&[(10.0,Op::AddValue)]),ControlFlow::Break(1.0));
    assert_eq!(mob.extra.attributes.as_ref().unwrap().bases,vec![(Attr::KnockbackResistance,0.1)]);
}
mod base_return {
    use super::{Mob,Attr,ops as pluto_living_attribute_ops};
    include!("../../../mod/pluto-living-attributes-base.rs");
    pub(super) fn apply(mob:&mut Mob,attribute:Attr,base:f64) {pluto_living_attributes_base((),mob,attribute,base);}
}
#[test]
fn actual_base_bridge_migrates_prejoin_writes_and_keeps_modifiers_across_later_zero_and_size_changes() {
    use sunrise_world::attributes::entity::Catalog;
    let catalog=Arc::new(Catalog {keys:[("minecraft:armor".into(),0)].into()});
    let declarations=||vec![AttributeModifier {attribute:"minecraft:armor".into(),id:"attributes:class".into(),amount:3.0,operation:2}];
    let mut mob=Mob {kind:Kind {id:"attributes:subject".into()},extra:Extra {armor_attributes:None,
        attributes:Some(ScalarAttributes {modifiers:vec![],bases:vec![(Attr::Armor,12.0)]})},gear_knockback_resistance:Some(0.25),refreshes:0};
    base_return::apply(&mut mob,Attr::Armor,12.0);
    assert!(mob.extra.armor_attributes.is_none(),"a setter must not manufacture an attribute map");
    ops::initialize(&mut mob,&catalog,declarations()).unwrap();
    let armor=|mob:&Mob| {let instance=mob.extra.armor_attributes.as_ref().unwrap().instance(0).unwrap();
        assert_eq!(instance.modifier("attributes:class").unwrap().amount(),3.0);
        assert!(instance.permanent_modifiers().is_empty());(instance.base_value(),instance.base_writes())};
    assert_eq!(armor(&mob),(12.0,1));
    base_return::apply(&mut mob,Attr::Armor,4.0);
    assert_eq!(armor(&mob),(4.0,2));
    base_return::apply(&mut mob,Attr::Armor,0.0);
    assert_eq!(armor(&mob),(0.0,3),"zero is an explicit base, not an inherited fallback");
    base_return::apply(&mut mob,Attr::KnockbackResistance,0.75);
    assert_eq!(armor(&mob),(0.0,3));assert_eq!(mob.gear_knockback_resistance,Some(0.25));assert_eq!(mob.refreshes,0);
    // Reload/join of an already owned map must not replay the old scalar base.
    let other=Arc::new(Catalog {keys:[("minecraft:armor".into(),100)].into()});
    ops::initialize(&mut mob,&other,declarations()).unwrap();assert_eq!(armor(&mob),(0.0,3));
    assert!(ops::apply_load_bases(&mut mob,&catalog,vec![("minecraft:armor".into(),9.0),("minecraft:attack_damage".into(),2.0)]).is_err());
    assert_eq!(armor(&mob),(0.0,3),"unsupported later entries reject before the first write");
    assert!(ops::apply_load_bases(&mut mob,&catalog,vec![("minecraft:armor".into(),9.0),("minecraft:armor".into(),2.0)]).is_err());
    assert_eq!(armor(&mob),(0.0,3));
    ops::apply_load_bases(&mut mob,&other,vec![("minecraft:armor".into(),6.0)]).unwrap();
    assert_eq!(armor(&mob),(6.0,4),"owned catalog keys and class multipliers survive load-time restoration");
    let mut unrelated=Mob {kind:Kind {id:"attributes:subject".into()},extra:Extra {armor_attributes:None,attributes:None},gear_knockback_resistance:None,refreshes:0};
    ops::initialize(&mut unrelated,&catalog,vec![]).unwrap();
    base_return::apply(&mut unrelated,Attr::Armor,8.0);assert!(unrelated.extra.armor_attributes.is_none());
    let absent=Arc::new(Catalog {keys:[("minecraft:knockback_resistance".into(),0)].into()});
    unrelated.extra.armor_attributes=Some(Box::new(absent.create("attributes:subject").unwrap()));
    base_return::apply(&mut unrelated,Attr::Armor,8.0);
    assert!(unrelated.extra.armor_attributes.as_ref().unwrap().instance(0).is_none(),"a missing armor key must not edit another attribute");
}
#[test]
fn actual_attribute_bridge_owns_transients_preserves_scalar_ops_and_removes_without_permanent_writes() {
    use sunrise_world::attributes::entity::Catalog;
    let catalog=Arc::new(Catalog {keys:["minecraft:armor","minecraft:armor_toughness","minecraft:knockback_resistance"]
        .into_iter().enumerate().map(|(i,k)|(k.into(),i)).collect()});
    let declarations=||["armor","armor_toughness","knockback_resistance"].into_iter().map(|a|AttributeModifier {
        attribute:format!("minecraft:{a}"),id:format!("attributes:{a}"),amount:3.0,operation:2,
    }).collect::<Vec<_>>();
    let mut mob=Mob {kind:Kind {id:"attributes:subject".into()},extra:Extra {armor_attributes:None,attributes:None},gear_knockback_resistance:Some(0.25),refreshes:0};
    let mut invalid=declarations();invalid[2].operation=7;
    assert!(ops::initialize(&mut mob,&catalog,invalid).is_err());assert!(mob.extra.armor_attributes.is_none());
    ops::initialize(&mut mob,&catalog,declarations()).unwrap();
    assert_eq!(mob.refreshes,1);assert_eq!(mob.gear_knockback_resistance,None);
    let map=mob.extra.armor_attributes.as_ref().unwrap();
    for (name,key) in &catalog.keys {
        let instance=map.instance(*key).unwrap();assert!(instance.permanent_modifiers().is_empty());
        assert_eq!(instance.modifier(&format!("attributes:{}",name.strip_prefix("minecraft:").unwrap())).unwrap().amount(),3.0);
    }
    let inherited=vec![(0.125,Op::AddValue)];
    assert_eq!(ops::append(&mob,Attr::KnockbackResistance,inherited.clone()),vec![(0.125,Op::AddValue),(3.0,Op::AddMultipliedTotal)]);
    assert_eq!(ops::append(&mob,Attr::Armor,inherited.clone()),inherited);
    assert_eq!(ops::transient_amount(&mob,Attr::KnockbackResistance,"attributes:knockback_resistance"),Some(3.0));
    // A retained map uses its own catalog keys, not a new catalog's indexes.
    let other=Arc::new(Catalog {keys:catalog.keys.iter().map(|(k,v)|(k.clone(),100+v)).collect()});
    ops::initialize(&mut mob,&other,declarations()).unwrap();assert_eq!(mob.refreshes,1);
    mob.extra.attributes=Some(ScalarAttributes {modifiers:vec![ScalarModifier {attr:Attr::KnockbackResistance,id:"attributes:knockback_resistance".into(),amount:0.0,op:Op::AddValue}],bases:vec![]});
    assert_eq!(ops::append(&mob,Attr::KnockbackResistance,Vec::new()),Vec::new(),"a scalar permanent replacement must not be multiplied again");
    assert_eq!(ops::transient_amount(&mob,Attr::KnockbackResistance,"attributes:knockback_resistance"),None);
    ops::remove_transient(&mut mob,Attr::KnockbackResistance,"attributes:knockback_resistance");
    mob.extra.attributes=None; // native remove then deletes the scalar modifier

    assert_eq!(ops::transient_amount(&mob,Attr::KnockbackResistance,"attributes:knockback_resistance"),None);
    assert_eq!(ops::append(&mob,Attr::KnockbackResistance,Vec::new()),Vec::new());
    mob.gear_knockback_resistance=Some(0.5);
    ops::remove_transient(&mut mob,Attr::KnockbackResistance,"attributes:missing");
    assert_eq!(mob.gear_knockback_resistance,Some(0.5),"no match must not invalidate native gear state");
}
