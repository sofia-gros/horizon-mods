extern crate self as horizon_client;
pub use pluto_inventory_api as pluto_inventory;

#[allow(dead_code)]
#[path = "../../../../../tconstruct/mixins/tconstruct/recipe-order.rs"]
mod consumer;
#[allow(dead_code)]
#[path = "../legacy_recipe_order.rs"]
mod legacy;

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_inventory::java_hashmap::{linked_iteration_order, utf16_hash, OrderError};

    fn compare(ids: &[String]) {
        // Explicitly preserve invalid-ID versus unsupported-tree outcomes.
        let old = legacy::java_recipe_order(ids).map_err(|e| format!("{e:?}"));
        let new = consumer::java_recipe_order(ids).map_err(|e| format!("{e:?}"));
        assert_eq!(new, old, "input={ids:?}");
    }

    #[test]
    fn original_recipe_policy_and_hash_resize_order_are_preserved() {
        // 1,285 comparisons: five source insertion presentations at each size.
        // Sorting/dedup are still owned by the real consumer, not the API.
        for len in 0..=256 {
            let ids: Vec<_> = (0..len).map(|n| format!("ns{}:group/{n}_{}", n % 7, n * 31)).collect();
            compare(&ids);
            let mut reverse = ids.clone(); reverse.reverse(); compare(&reverse);
            let mut rotated = ids.clone();
            if len > 0 { rotated.rotate_left(len / 3); }
            compare(&rotated);
            let mut duplicates = ids.clone(); duplicates.extend(ids.iter().cloned()); compare(&duplicates);
            let mut padded = ids.clone(); padded.extend(["a:_metadata".into(), "z:prefix-early".into(), "a:prefix".into()]); compare(&padded);
        }
    }

    #[test]
    fn invalid_ids_unicode_and_collision_families_match_legacy() {
        for id in ["", "missing", ":path", "namespace:", "a:b:c", "😀:雪", "a:𝄞", "a:é", "a:e\u{301}"] {
            compare(&[id.into()]);
        }
        for len in 0..=16 {
            let ids = (0..len).map(|bits| {
                let path = (0..4).map(|n| if bits & (1 << n) == 0 { "an" } else { "c0" }).collect::<String>();
                format!("test:{path}")
            }).collect::<Vec<_>>();
            compare(&ids);
        }
    }

    #[test]
    fn raw_hash_spread_linked_collisions_and_resizes_are_explicit() {
        assert_eq!(linked_iteration_order(&[]), Ok(vec![]));
        assert_eq!(linked_iteration_order(&[0x10000, 0]), Ok(vec![1, 0]));
        assert_eq!(linked_iteration_order(&[0; 8]), Ok((0..8).collect()));
        // The ninth collision resizes to 32, the tenth to 64, the eleventh
        // would treeify. Equal hashes are NOT silently deduplicated.
        assert_eq!(linked_iteration_order(&[0; 9]), Ok((0..9).collect()));
        assert_eq!(linked_iteration_order(&[0; 10]), Ok((0..10).collect()));
        assert_eq!(linked_iteration_order(&[0; 11]), Err(OrderError::TreeBin));
        // At the 13th insertion load growth splits a linked bin while retaining
        // the order of both its low and high subsequences.
        let hashes = [16, 0, 32, 48, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert_eq!(linked_iteration_order(&hashes), Ok(vec![1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 3]));
    }

    #[test]
    fn java_utf16_hash_uses_wrapping_units_without_normalization() {
        assert_eq!(utf16_hash([]), 0);
        assert_eq!(utf16_hash("abc".encode_utf16()), 96354);
        assert_eq!(utf16_hash("😀".encode_utf16()), 1772899);
        assert_eq!(utf16_hash([0xd800]), 0xd800);
        assert_ne!(utf16_hash("é".encode_utf16()), utf16_hash("e\u{301}".encode_utf16()));
        let long: Vec<_> = (0..4096).map(|n| (n * 53) as u16).collect();
        let expected = long.iter().fold(0i32, |hash, &unit| hash.wrapping_mul(31).wrapping_add(i32::from(unit)));
        assert_eq!(utf16_hash(long), expected as u32);
    }
}
