//! Owned item prototypes from a server reload, never a live-world lookup.
use super::{base::Identifier, player_api::StackData, resource_loader_v0::RegistryLookup};
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};

/// One immutable generation of item defaults in PlutoLib's reduced StackData
/// representation (count/damage/durability/custom NBT, not every native component),
/// plus separately captured item-level properties such as crafting remainders.
#[derive(Clone,Debug,PartialEq)]
pub struct ItemRegistrySnapshot {
    stacks:Arc<BTreeMap<Identifier,StackData>>,
    crafting_remainders:Option<Arc<BTreeMap<Identifier,Option<Identifier>>>>,
}
impl ItemRegistrySnapshot {
    /// A fresh owned copy of this item's captured default stack; None for an
    /// unknown item. Reading or editing the copy never refreshes this generation.
    pub fn default_stack(&self,item:&Identifier)->Option<StackData> { self.stacks.get(item).cloned() }
    /// The captured item-level crafting remainder, not its use/consumption
    /// remainder. Ok(None) means a known item has none; unknown items and an
    /// unavailable property capture are errors, never invented negative answers.
    pub fn crafting_remainder(&self,item:&Identifier)->Result<Option<Identifier>,String> {
        if !self.stacks.contains_key(item) {return Err(format!("unknown item {item}"));}
        let values=self.crafting_remainders.as_ref().ok_or("item crafting remainders unavailable")?;
        values.get(item).cloned().ok_or_else(||format!("missing crafting remainder for {item}"))
    }
    pub(crate) fn with_crafting_remainders(mut self,records:Vec<(String,Option<String>)>)->Result<Self,String> {
        let mut values=BTreeMap::new();
        for (item,remainder) in records {
            let item=Identifier::parse(&item).map_err(|error|error.to_string())?;
            if !self.stacks.contains_key(&item) {return Err(format!("unknown crafting-remainder item {item}"));}
            let remainder=remainder.map(|id|Identifier::parse(&id).map_err(|error|error.to_string())).transpose()?;
            if remainder.as_ref().is_some_and(|id|!self.stacks.contains_key(id)) {return Err("unknown crafting-remainder target".into());}
            if values.insert(item,remainder).is_some() {return Err("duplicate crafting-remainder item".into());}
        }
        if values.len()!=self.stacks.len() {return Err("incomplete crafting-remainder snapshot".into());}
        self.crafting_remainders=Some(Arc::new(values));
        Ok(self)
    }
    pub(crate) fn decode(lookup:&RegistryLookup,stacks:Vec<StackData>)->Result<Self,String> {
        let mut parsed=BTreeMap::new();
        for stack in stacks {
            let id=Identifier::parse(&stack.item).map_err(|e|e.to_string())?;
            if !lookup.contains(&id) || stack.count!=1 || stack.damage!=0 || stack.max_damage.is_some_and(|n|n<=0) {
                return Err(format!("invalid default item stack {}",stack.item));
            }
            if parsed.insert(id,stack).is_some() { return Err("duplicate item prototype".into()); }
        }
        if parsed.len()!=lookup.entry_count() { return Err("incomplete item prototype snapshot".into()); }
        Ok(Self { stacks:Arc::new(parsed),crafting_remainders:None })
    }
}
thread_local! { static CURRENT:RefCell<Vec<Option<ItemRegistrySnapshot>>>=const { RefCell::new(Vec::new()) }; }
pub(crate) fn with_item_registry<T>(snapshot:Option<ItemRegistrySnapshot>,action:impl FnOnce()->T)->T {
    struct Scope;
    impl Drop for Scope { fn drop(&mut self) { CURRENT.with(|v| { v.borrow_mut().pop(); }); } }
    CURRENT.with(|v|v.borrow_mut().push(snapshot));
    let _scope=Scope;
    action()
}
/// Fabric-style access to registered item prototypes. The current backend
/// provides owned snapshots during server JSON reload callbacks only; elsewhere
/// it is unavailable rather than consulting another world or builtin item list.
pub struct ItemRegistry;
impl ItemRegistry {
    /// The current server reload's item defaults. None outside a supported
    /// callback or when capture/validation failed. Retain it for later work.
    pub fn snapshot()->Option<ItemRegistrySnapshot> {
        CURRENT.with(|v|v.borrow().last().cloned().flatten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::base::Nbt;
    #[test]
    fn crafting_remainders_preserve_absence_availability_and_whole_generation_ownership() {
        let filled=Identifier::parse("test:filled").unwrap();let container=Identifier::parse("test:container").unwrap();
        let lookup=RegistryLookup::from_parts(&[filled.to_string(),container.to_string()],&[]).unwrap();
        let stacks=[&filled,&container].map(|id|StackData {item:id.to_string(),count:1,..Default::default()}).to_vec();
        let base=ItemRegistrySnapshot::decode(&lookup,stacks).unwrap();
        assert!(base.crafting_remainder(&filled).unwrap_err().contains("unavailable"));
        let records=vec![(filled.to_string(),Some(container.to_string())),(container.to_string(),None)];
        let original=base.clone().with_crafting_remainders(records.clone()).unwrap();
        assert_eq!(original.crafting_remainder(&filled).unwrap(),Some(container.clone()));
        assert_eq!(original.crafting_remainder(&container).unwrap(),None);
        assert!(original.crafting_remainder(&Identifier::parse("test:unknown").unwrap()).is_err());
        let retained=original.clone();
        let changed=base.clone().with_crafting_remainders(vec![(filled.to_string(),None),(container.to_string(),None)]).unwrap();
        with_item_registry(Some(changed),|| {
            assert_eq!(ItemRegistry::snapshot().unwrap().crafting_remainder(&filled).unwrap(),None);
            assert_eq!(retained.crafting_remainder(&filled).unwrap(),Some(container.clone()));
        });
        for invalid in [vec![],vec![records[0].clone()],vec![records[0].clone(),records[0].clone()],
            vec![(filled.to_string(),Some("test:unknown".into())),records[1].clone()],
            vec![("test:unknown".into(),None),records[1].clone()]] {
            assert!(base.clone().with_crafting_remainders(invalid).is_err());
            assert_eq!(retained.crafting_remainder(&filled).unwrap(),Some(container.clone()));
        }
    }

    #[test]
    fn prototypes_validate_whole_generation_and_keep_owned_typed_data_across_nested_reloads() {
        let item=Identifier::parse("test:tool").unwrap();
        let lookup=RegistryLookup::from_parts(&[item.to_string()],&[]).unwrap();
        let stack=StackData { item:item.to_string(),count:1,max_damage:Some(41),custom_data:[
            ("test:long".into(),Nbt::Long(9)),("test:bytes".into(),Nbt::ByteArray(vec![0,255]))
        ].into_iter().collect(),..Default::default() };
        let original=ItemRegistrySnapshot::decode(&lookup,vec![stack.clone()]).unwrap();
        assert!(ItemRegistry::snapshot().is_none());
        let kept=with_item_registry(Some(original.clone()),|| {
            let kept=ItemRegistry::snapshot().unwrap();
            let mut copied=kept.default_stack(&item).unwrap();
            copied.custom_data.clear(); copied.max_damage=Some(2);
            assert_eq!(kept.default_stack(&item),Some(stack.clone()));
            assert!(kept.default_stack(&Identifier::parse("test:absent").unwrap()).is_none());
            let failed=std::panic::catch_unwind(||with_item_registry(None,|| {
                assert!(ItemRegistry::snapshot().is_none()); panic!("nested reload failed");
            }));
            assert!(failed.is_err()); assert_eq!(ItemRegistry::snapshot(),Some(original.clone()));
            let mut changed=stack.clone(); changed.max_damage=Some(99);
            with_item_registry(Some(ItemRegistrySnapshot::decode(&lookup,vec![changed]).unwrap()),|| {
                assert_eq!(ItemRegistry::snapshot().unwrap().default_stack(&item).unwrap().max_damage,Some(99));
            });
            kept
        });
        assert!(ItemRegistry::snapshot().is_none());
        assert_eq!(kept.default_stack(&item),Some(stack.clone()));
        assert!(ItemRegistrySnapshot::decode(&lookup,vec![]).is_err());
        assert!(ItemRegistrySnapshot::decode(&lookup,vec![stack.clone(),stack.clone()]).is_err());
        let mut invalid=stack; invalid.item="test:unknown".into();
        assert!(ItemRegistrySnapshot::decode(&lookup,vec![invalid]).is_err());
    }
}
