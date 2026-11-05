use horizon_items::items::ItemStack;
use horizon_world::nbt::{self, Limits, Tag};
pub(super) type Frame = ([Option<ItemStack>; 4], u8);
const MAX: usize = 65_536;
fn limits() -> Limits {
    Limits {
        max_bytes: MAX,
        max_nodes: 16_384,
        max_depth: 64,
    }
}
fn matches(left: &Option<ItemStack>, right: &Option<ItemStack>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.count == b.count && a.damage == b.damage && a.same_item_same_components(b)
        }
        _ => false,
    }
}
pub(super) fn encode(armor: &[Option<ItemStack>], capacity: Option<u8>) -> Vec<u8> {
    let Some(capacity) = capacity.filter(|n| *n <= 4) else {
        return vec![0];
    };
    if armor.len() != 4 {
        return vec![0];
    }
    let slots = armor
        .iter()
        .map(|slot| {
            slot.as_ref()
                .map(horizon_items::item_nbt::item_to_nbt)
                .unwrap_or_else(|| Tag::compound([] as [(&str, Tag); 0]))
        })
        .collect();
    let frame = Tag::compound([
        ("armor", Tag::List(slots)),
        ("capacity", Tag::Int(i32::from(capacity))),
    ]);
    let Ok(bytes) = nbt::write_network(&frame, limits()) else {
        return vec![0];
    };
    let mut result = vec![1];
    result.extend(bytes);
    if result.len() > MAX {
        return vec![0];
    }
    // NBT conversion must not discard components from a positive permission.
    if decode(&result)
        .as_ref()
        .and_then(|frame| matching_capacity(Some(frame), armor))
        != Some(capacity)
    {
        return vec![0];
    }
    result
}
pub(super) fn decode(bytes: &[u8]) -> Option<Frame> {
    if bytes.len() > MAX || bytes.first() != Some(&1) {
        return None;
    }
    let (tag, used) = nbt::read_network(&bytes[1..], limits()).ok()?;
    if used != bytes.len() - 1 {
        return None;
    }
    let capacity = match tag.get("capacity")? {
        Tag::Int(n) if (0..=4).contains(n) => *n as u8,
        _ => return None,
    };
    let Tag::List(slots) = tag.get("armor")? else {
        return None;
    };
    if slots.len() != 4 {
        return None;
    }
    let mut armor = [None, None, None, None];
    for (index, slot) in slots.iter().enumerate() {
        if matches!(slot, Tag::Compound(fields) if fields.is_empty()) {
            continue;
        }
        let stack = horizon_items::item_nbt::item_from_nbt(slot)?;
        if stack.count == 0 {
            return None;
        }
        armor[index] = Some(stack);
    }
    Some((armor, capacity))
}
pub(super) fn matching_capacity(frame: Option<&Frame>, armor: &[Option<ItemStack>]) -> Option<u8> {
    let (expected, capacity) = frame?;
    (armor.len() == 4 && expected.iter().zip(armor).all(|(a, b)| matches(a, b)))
        .then_some(*capacity)
}
pub(super) fn needs_send(entry: &mut (Vec<u8>, u8), bytes: &[u8]) -> bool {
    if entry.0 != bytes || entry.1 == 0 {
        entry.0 = bytes.to_vec();
        entry.1 = 19;
        true
    } else {
        entry.1 -= 1;
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_is_bound_to_all_four_current_equipment_slots() {
        let armor = [
            Some(ItemStack::new("tconstruct:slime_helmet", 1)),
            None,
            None,
            None,
        ];
        let bytes = encode(&armor, Some(1));
        let frame = decode(&bytes).unwrap();
        assert_eq!(matching_capacity(Some(&frame), &armor), Some(1));
        let mut changed = armor.clone();
        changed[0].as_mut().unwrap().damage = 1;
        assert_eq!(matching_capacity(Some(&frame), &changed), None);
        assert_eq!(
            matching_capacity(Some(&frame), &[None, None, None, None]),
            None
        );
        assert_eq!(matching_capacity(None, &armor), None);
        let mut trailing = bytes;
        trailing.push(0);
        assert!(decode(&trailing).is_none());
        assert!(decode(&vec![1; MAX + 1]).is_none());
        assert!(decode(&encode(&armor, Some(5))).is_none());
    }
    #[test]
    fn invalidation_and_periodic_refresh_are_sent() {
        let mut cache = (Vec::new(), 0);
        assert!(needs_send(&mut cache, &[1, 2]));
        assert!(!needs_send(&mut cache, &[1, 2]));
        assert!(needs_send(&mut cache, &[0]));
        for _ in 0..19 {
            assert!(!needs_send(&mut cache, &[0]));
        }
        assert!(needs_send(&mut cache, &[0]));
    }
}
