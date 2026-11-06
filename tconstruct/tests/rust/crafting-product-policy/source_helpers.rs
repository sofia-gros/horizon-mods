pub(super) fn refill(current: u32, cap: u32, per_item: u32, available: u32, overworked: u32) -> Option<(u32, u32)> {
    if current >= cap || per_item == 0 || available == 0 { return None; }
    let each = per_item.saturating_mul(overworked.saturating_add(1));
    let count = (cap - current).div_ceil(each).min(available);
    Some((current.saturating_add(each.saturating_mul(count)).min(cap), count))
}

pub(super) fn amount(stack: &horizon_items::items::ItemStack, key: &str) -> i32 {
    use horizon_world::nbt::Tag;
    match stack.data().and_then(|d| d.other("minecraft:custom_data")).and_then(|d| d.get(key)) {
        Some(Tag::Int(n)) => (*n).max(0), _ => 0,
    }
}

