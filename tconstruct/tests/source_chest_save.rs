extern crate horizon_client;
extern crate horizon_world;
mod containers { pub use horizon_client::containers::BlockEntity; }
mod save {
    use horizon_client::save::decode_items;
    include!("../mixins/tconstruct/source-machine-decode.rs");
    include!("../mixins/tconstruct/source-machine-encode-id.rs");
    #[cfg(test)]
    mod tests {
        use super::*;
        use horizon_client::items::ItemStack;
        use horizon_world::nbt::Tag;
        #[test]
        fn source_chests_keep_high_slots_and_component_data() {
            for (name, id, n) in [("tconstruct:crafting_station", "tconstruct:crafting_station", 9usize), ("tconstruct:part_chest", "tconstruct:part_chest", 256usize), ("tconstruct:tinkers_chest", "tconstruct:modifier_chest", 64usize)] {
                let mut slots = vec![None; n];
                let mut stack = ItemStack::new("tconstruct:tool_handle", 3);
                stack.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("Material", Tag::String("tconstruct:wood".into()))])));
                slots[n-1] = Some(stack.clone());
                let entity = crate::containers::BlockEntity::Chest(slots.clone());
                assert_eq!(tconstruct_machine_encode_id(&entity,name),std::ops::ControlFlow::Break(id.to_string()));
                let data = Tag::compound([("id",Tag::String(id.into())),("x",Tag::Int(1)),("y",Tag::Int(2)),("z",Tag::Int(3)),("Items",horizon_client::save::encode_items(&slots))]);
                let std::ops::ControlFlow::Break(Some((pos,crate::containers::BlockEntity::Chest(restored)))) = tconstruct_machine_decode(&data) else { panic!("decode failed") };
                assert_eq!(pos,(1,2,3));assert_eq!(restored.len(),n);
                assert!(restored[n-1].as_ref().unwrap().same_item_same_components(&stack));
                assert_eq!(restored[n-1].as_ref().unwrap().count,3);
            }
        }
    }
}
