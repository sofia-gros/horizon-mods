// Actor-specific chest Haste snapshot. Unknown never becomes a zero-proof value.
use horizon_items::items::ItemStack;
use horizon_world::nbt::{self, Limits, Tag};
const MAX: usize = 16_384;
const REFRESH: u8 = 20;
fn limits() -> Limits { Limits { max_bytes: MAX, max_nodes: 4096, max_depth: 64 } }

pub(super) fn encode(value: Option<(&ItemStack, i32)>) -> Vec<u8> {
    let Some((stack, level)) = value.filter(|(_, level)| (0..=32767).contains(level)) else { return vec![0]; };
    let item = horizon_items::item_nbt::item_to_nbt(stack);
    let Some(roundtrip) = horizon_items::item_nbt::item_from_nbt(&item) else { return vec![0]; };
    if roundtrip.count != stack.count || !roundtrip.same_item_same_components(stack) { return vec![0]; }
    let data = Tag::compound([("item", item), ("level", Tag::Int(level))]);
    match nbt::write_network(&data, limits()) {
        Ok(data) if data.len()+1 <= MAX => { let mut out=vec![1]; out.extend(data); out },
        _ => vec![0],
    }
}
pub(super) fn decode(bytes: &[u8]) -> Option<(ItemStack, i32)> {
    if bytes.len() > MAX || bytes.first() != Some(&1) { return None; }
    let (tag, used) = nbt::read_network(&bytes[1..], limits()).ok()?;
    if used != bytes.len()-1 { return None; }
    let level = match tag.get("level")? { Tag::Int(level) if (0..=32767).contains(level) => *level, _ => return None };
    Some((horizon_items::item_nbt::item_from_nbt(tag.get("item")?)?, level))
}
pub(super) fn matching_level(snapshot: Option<&(ItemStack, i32)>, chest: Option<&ItemStack>) -> Option<i32> {
    let (item, level) = snapshot?; let chest = chest?;
    (item.count == chest.count && item.same_item_same_components(chest)).then_some(*level)
}
pub(super) fn needs_send(entry: &mut (Vec<u8>,u8), bytes: &[u8]) -> bool {
    if entry.0 != bytes || entry.1 == 0 { entry.0=bytes.to_vec(); entry.1=REFRESH-1; true }
    else { entry.1-=1; false }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn roundtrip_requires_matching_equipment_and_is_bounded() {
        let mut item=ItemStack::new("tconstruct:plate_chestplate",1);
        item.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_materials",Tag::List(vec![Tag::String("test:one".into())]))])));
        let bytes=encode(Some((&item,2))); let snapshot=decode(&bytes).unwrap();
        assert_eq!(matching_level(Some(&snapshot),Some(&item)),Some(2));
        assert_eq!(matching_level(Some(&snapshot),None),None);
        let mut changed=item.clone();changed.damage=1;
        assert_eq!(matching_level(Some(&snapshot),Some(&changed)),None);
        assert_eq!(decode(&encode(None)),None);
        assert_eq!(decode(&vec![1;MAX+1]),None);
        let mut trailing=bytes;trailing.push(0);assert_eq!(decode(&trailing),None);
    }
    #[test] fn changes_and_reconnect_refresh_without_cross_actor_reuse() {
        let a=ItemStack::new("test:a",1);let b=ItemStack::new("test:b",1);
        let pa=encode(Some((&a,1)));let pb=encode(Some((&b,3)));let mut cache=(Vec::new(),0);
        assert!(needs_send(&mut cache,&pa));assert!(!needs_send(&mut cache,&pa));assert!(needs_send(&mut cache,&pb));
        assert_eq!(matching_level(decode(&pa).as_ref(),Some(&b)),None);
        let mut fresh=(Vec::new(),0);assert!(needs_send(&mut fresh,&pb));
        assert!(needs_send(&mut cache,&encode(None)));
    }
    #[test] fn opaque_wire_patch_is_not_part_of_decoded_component_comparison() {
        use std::sync::Arc;
        let mut item=ItemStack::new("test:chest",1);
        item.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_materials",Tag::List(vec![Tag::String("test:one".into())]))])));
        let snapshot=decode(&encode(Some((&item,2)))).unwrap();
        let mut received=item.clone();
        received.data_mut().wire=horizon_items::item_data::Wire::new(item,Arc::from(vec![1u8,2,3]));
        // Horizon's ItemStack comparison excludes the opaque patch. The
        // fixed Haste lookup reads the decoded item ID and material data.
        assert_eq!(matching_level(Some(&snapshot),Some(&received)),Some(2));
    }
    #[test] fn reload_requires_invalidation_before_the_next_frame() {
        let chest=ItemStack::new("test:chest",1);
        let old=decode(&encode(Some((&chest,2))));
        // A world-only change cannot be inferred from the unchanged item.
        assert_eq!(matching_level(old.as_ref(),Some(&chest)),Some(2));
        let invalidated: Option<(ItemStack,i32)>=None;
        assert_eq!(matching_level(invalidated.as_ref(),Some(&chest)),None);
        let fresh=decode(&encode(Some((&chest,1))));
        assert_eq!(matching_level(fresh.as_ref(),Some(&chest)),Some(1));
    }

}
