// Bounded source Multishot: real ammunition is consumed for every projectile.
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
use serde_json::{Value, json};

pub(crate) const ID: &str = "tconstruct:multishot";
pub(crate) const MAX_LEVEL: i32 = 31;
pub(crate) fn desired(level: i32) -> Option<usize> {
    (1..=MAX_LEVEL)
        .contains(&level)
        .then(|| (1 + 2 * level) as usize)
}
pub(crate) fn known_definition(bytes: Option<&[u8]>) -> bool {
    bytes.and_then(|b| serde_json::from_slice::<Value>(b).ok()).is_some_and(|v| v == json!({
        "level_display":"tconstruct:default", "modules":[{"type":"tconstruct:volatile_int","each_level":2,"flag":ID}]
    }))
}
pub(crate) fn saved_level(stack: &ItemStack) -> Option<i32> {
    let upgrades = stack
        .data()
        .and_then(|d| d.other("minecraft:custom_data"))
        .and_then(|d| d.get("tic_upgrades"));
    match upgrades {
        None => Some(0),
        Some(Tag::Compound(fields)) => match fields.get(&ID.into()) {
            None => Some(0),
            Some(Tag::Int(n)) if *n >= 0 => Some(*n),
            _ => None,
        },
        _ => None,
    }
}
pub(crate) fn arrow(stack: &ItemStack) -> bool {
    matches!(
        stack.item.as_ref(),
        "minecraft:arrow" | "minecraft:tipped_arrow" | "minecraft:spectral_arrow"
    )
}
pub(crate) struct Load {
    pub consumed: Vec<(usize, ItemStack, u32)>,
    pub charged: Vec<ItemStack>,
}
/// `BowAmmoModifierHook.consumeAmmo`: initial source stack first, then only
/// same-item/same-components stacks. Fewer arrows produce a partial load.
pub(crate) fn plan(
    ammo: &ItemStack,
    first: Option<usize>,
    slots: &[(usize, ItemStack)],
    wanted: usize,
    creative: bool,
) -> Option<Load> {
    if !arrow(ammo) || ammo.count == 0 || !(1..=63).contains(&wanted) {
        return None;
    }
    let mut consumed = Vec::new();
    let count = if creative {
        wanted
    } else {
        let first = first?;
        let original = slots.iter().find(|(i, _)| *i == first)?.1.clone();
        if original.count != ammo.count || !original.same_item_same_components(ammo) {
            return None;
        }
        let mut left = wanted as u32;
        let mut seen = std::collections::BTreeSet::new();
        for (i, stack) in
            std::iter::once((first, &original)).chain(slots.iter().map(|(i, s)| (*i, s)))
        {
            if !seen.insert(i) || !stack.same_item_same_components(ammo) || stack.count == 0 {
                continue;
            }
            let take = left.min(stack.count);
            consumed.push((i, stack.clone(), take));
            left -= take;
            if left == 0 {
                break;
            }
        }
        wanted - left as usize
    };
    if count == 0 {
        return None;
    }
    let mut drawn = ammo.clone();
    drawn.count = 1;
    if creative {
        drawn.data_mut().set_other(
            "minecraft:intangible_projectile",
            Some(Tag::Compound(Default::default())),
        );
    }
    Some(Load {
        consumed,
        charged: vec![drawn; count],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matching_stacks_partial_loads_and_real_consumption() {
        let first = ItemStack::new("minecraft:arrow", 2);
        let slots = vec![
            (40, first.clone()),
            (0, ItemStack::new("minecraft:spectral_arrow", 64)),
            (3, ItemStack::new("minecraft:arrow", 4)),
        ];
        let load = plan(&first, Some(40), &slots, 5, false).unwrap();
        assert_eq!(
            load.consumed
                .iter()
                .map(|(i, _, n)| (*i, *n))
                .collect::<Vec<_>>(),
            vec![(40, 2), (3, 3)]
        );
        assert_eq!(load.charged.len(), 5);
        assert!(
            load.charged
                .iter()
                .all(|s| s.count == 1 && s.same_item_same_components(&first))
        );
        let partial = plan(&first, Some(40), &slots, 9, false).unwrap();
        assert_eq!(partial.charged.len(), 6);
        assert_eq!(partial.consumed.iter().map(|(_, _, n)| n).sum::<u32>(), 6);
    }
    #[test]
    fn components_creative_virtual_and_malformed_inputs() {
        let mut tipped = ItemStack::new("minecraft:tipped_arrow", 2);
        tipped
            .data_mut()
            .set_other("minecraft:custom_name", Some(Tag::String("A".into())));
        let mut other = tipped.clone();
        other
            .data_mut()
            .set_other("minecraft:custom_name", Some(Tag::String("B".into())));
        let slots = vec![(0, tipped.clone()), (1, other)];
        assert_eq!(
            plan(&tipped, Some(0), &slots, 5, false)
                .unwrap()
                .charged
                .len(),
            2
        );
        assert!(plan(&tipped, None, &slots, 3, false).is_none());
        let load = plan(&tipped, None, &[], 5, true).unwrap();
        assert!(load.consumed.is_empty());
        assert_eq!(load.charged.len(), 5);
        assert!(load.charged.iter().all(|s| {
            s.data()
                .unwrap()
                .other("minecraft:intangible_projectile")
                .is_some()
        }));
        assert!(
            plan(
                &ItemStack::new("minecraft:firework_rocket", 5),
                Some(0),
                &slots,
                3,
                false
            )
            .is_none()
        );
        assert!(plan(&tipped, Some(0), &slots, 65, false).is_none());
        for n in [-1, 0, 32, i32::MAX] {
            assert!(desired(n).is_none());
        }
        assert_eq!(desired(1), Some(3));
        assert_eq!(desired(2), Some(5));
    }
    #[test]
    fn source_redefinition_and_saved_data_fail_closed() {
        let source=br#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:volatile_int","each_level":2,"flag":"tconstruct:multishot"}]}"#;
        assert!(known_definition(Some(source)));
        assert!(!known_definition(None));
        assert!(!known_definition(Some(b"{}")));
        let mut altered: Value = serde_json::from_slice(source).unwrap();
        altered["modules"][0]["each_level"] = 3.into();
        assert!(!known_definition(Some(
            &serde_json::to_vec(&altered).unwrap()
        )));
        let mut bow = ItemStack::new("tconstruct:crossbow", 1);
        assert_eq!(saved_level(&bow), Some(0));
        bow.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([("tic_upgrades", Tag::Int(1))])),
        );
        assert_eq!(saved_level(&bow), None);
    }
}
