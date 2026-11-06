use crate::{cells, consumer, legacy};
use horizon_items::{item_data::Wire, items::ItemStack};
use horizon_world::nbt::Tag;
use std::sync::Arc;

fn stack(count: u32, variant: u8) -> ItemStack {
    let mut item = ItemStack::new(if variant == 1 { "minecraft:dirt" } else { "minecraft:stone" }, count);
    if variant >= 2 {
        item.data_mut().set_other("example:unknown", Some(Tag::compound([("future", Tag::Int(7))])));
        item.data_mut().bundle_selected.0 = 3;
        let mut child = ItemStack::new("minecraft:brick", 1);
        let decoded = child.clone();
        child.data_mut().wire = Wire::new(decoded, Arc::from([4u8, 5, 6]));
        child.damage = 1;
        item.data_mut().bundle_contents.push(child);
        let decoded = item.clone();
        item.data_mut().wire = Wire::new(decoded, Arc::from([1u8, 2, 3]));
        if variant == 3 { item.damage += 1; }
    }
    item
}
fn same(a: &ItemStack, b: &ItemStack) -> bool { a.same_item_same_components(b) }
fn state(value: &impl std::fmt::Debug) -> String { format!("{value:?}") }
fn total(cells: &[Option<ItemStack>], carried: &Option<ItemStack>) -> u64 {
    cells.iter().chain(std::iter::once(carried)).flatten().map(|s| u64::from(s.count)).sum()
}

#[test]
fn pickup_valid_inputs_match_actual_legacy_wrapper() {
    let counts = [1, 2, 3, 16, 63, 64, 65, u32::MAX];
    let mut cases = 0;
    for limit in [1, 16, 64, u32::MAX] {
        for right in [false, true] {
            for cursor in std::iter::once(None).chain(counts.map(|n| Some(stack(n, 2)))) {
                for cell in std::iter::once(None).chain(counts.into_iter().flat_map(|n| (0..4).map(move |v| Some(stack(n, v))))) {
                    let mut old = vec![cell.clone()]; let mut old_cursor = cursor.clone();
                    let mut new = cell; let mut new_cursor = cursor.clone();
                    let before = total(&old, &cursor);
                    let mut delegated = old.clone(); let mut delegated_cursor = cursor.clone();
                    let expected = legacy::pickup_player(&mut old, 0, &mut old_cursor, right, limit);
                    assert_eq!(cells::pickup(&mut new, &mut new_cursor, right, limit, same), Ok(expected));
                    assert_eq!(consumer::pickup_player(&mut delegated, 0, &mut delegated_cursor, right, limit), expected);
                    assert_eq!(state(&(&delegated[0], &delegated_cursor)), state(&(&old[0], &old_cursor)));
                    assert_eq!(state(&(&new, &new_cursor)), state(&(&old[0], &old_cursor)));
                    assert_eq!(total(&[new], &new_cursor), before);
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 2376);
}

#[test]
fn take_valid_inputs_match_actual_legacy_wrapper() {
    for all in [false, true] {
        for value in std::iter::once(None).chain([1, 2, 3, 64, u32::MAX].into_iter().flat_map(|n| (0..4).map(move |v| Some(stack(n, v))))) {
            let mut old = vec![value.clone()]; let mut new = value;
            let mut delegated = old.clone();
            let expected = legacy::take_for_drop(&mut old, 0, all);
            let delegated_result = consumer::take_for_drop(&mut delegated, 0, all);
            assert_eq!(state(&(&delegated[0], &delegated_result)), state(&(&old[0], &expected)));
            let actual = cells::take(&mut new, all).unwrap();
            assert_eq!(state(&(&new, &actual)), state(&(&old[0], &expected)));
        }
    }
}

#[test]
fn ordered_collection_matches_actual_legacy_for_duplicates_padding_and_two_passes() {
    for limit in [1, 16, 64, u32::MAX] {
        for carried in [None, Some(stack(1, 2)), Some(stack(63, 2)), Some(stack(u32::MAX, 2))] {
            for count in [1, 16, 64, 65, u32::MAX] {
                for variant in 0..4 {
                    for reverse in [false, true] {
                        let mut old_first = vec![Some(stack(count, variant)), None, Some(stack(64, 2))];
                        let mut old_second = vec![Some(stack(7, 2)), Some(stack(5, 1))];
                        let mut new_first = old_first.clone(); let mut new_second = old_second.clone();
                        let mut old_cursor = carried.clone(); let mut new_cursor = carried.clone();
                        let mut order = vec![(false, 0), (true, 0), (true, 0), (true, 2), (false, 1), (true, usize::MAX), (false, 9)];
                        if reverse { order.reverse(); }
                        let mut delegated_first = old_first.clone(); let mut delegated_second = old_second.clone(); let mut delegated_cursor = carried.clone();
                        let mapped: Vec<_> = order.iter().map(|&(first, i)| if first { cells::Cell::First(i) } else { cells::Cell::Second(i) }).collect();
                        let expected = legacy::pickup_all_ordered(&mut old_first, &mut old_second, &mut old_cursor, &order, limit);
                        let actual = cells::collect_ordered(&mut new_first, &mut new_second, &mut new_cursor, &mapped, limit, same);
                        assert_eq!(actual, Ok(expected));
                        assert_eq!(consumer::pickup_all_ordered(&mut delegated_first, &mut delegated_second, &mut delegated_cursor, &order, limit), expected);
                        assert_eq!(state(&(delegated_first, delegated_second, delegated_cursor)), state(&(&old_first, &old_second, &old_cursor)));
                        assert_eq!(state(&(new_first, new_second, new_cursor)), state(&(old_first, old_second, old_cursor)));
                    }
                }
            }
        }
    }
}

#[test]
fn invalid_pickup_is_rejected_before_mutation_or_policy() {
    for (cell, carried, limit, expected) in [
        (None, Some(stack(0, 2)), 64, cells::CellError::ZeroCount),
        (Some(stack(0, 0)), Some(stack(1, 2)), 64, cells::CellError::ZeroCount),
        (None, Some(stack(1, 2)), 0, cells::CellError::ZeroLimit),
        (Some(stack(2, 2)), None, 0, cells::CellError::ZeroLimit),
    ] {
        let mut cell = cell; let mut carried = carried;
        let before = state(&(&cell, &carried));
        assert_eq!(cells::pickup(&mut cell, &mut carried, true, limit, |_, _| panic!("invalid input must not invoke policy")), Err(expected));
        assert_eq!(state(&(&cell, &carried)), before);
    }
    let mut zero = Some(stack(0, 2)); let before = state(&zero);
    assert_eq!(cells::take(&mut zero, false).unwrap_err(), cells::CellError::ZeroCount);
    assert_eq!(state(&zero), before);
}

#[test]
fn invalid_collection_rejects_before_any_partial_merge() {
    let mut first = vec![Some(stack(2, 2)), Some(stack(0, 2))];
    let mut second = vec![Some(stack(3, 2))]; let mut cursor = Some(stack(1, 2));
    let before = state(&(&first, &second, &cursor));
    assert_eq!(cells::collect_ordered(&mut first, &mut second, &mut cursor,
        &[cells::Cell::First(0), cells::Cell::First(1)], 64, |_, _| panic!("prevalidate all selected cells")), Err(cells::CellError::ZeroCount));
    assert_eq!(state(&(&first, &second, &cursor)), before);
    assert_eq!(cells::collect_ordered(&mut first, &mut second, &mut cursor,
        &[cells::Cell::First(0)], 0, same), Err(cells::CellError::ZeroLimit));
    assert_eq!(state(&(&first, &second, &cursor)), before);
    // An unselected malformed cell does not affect this explicitly bounded call.
    assert_eq!(cells::collect_ordered(&mut first, &mut second, &mut cursor,
        &[cells::Cell::First(0)], 64, same), Ok(true));
    assert_eq!(first[1].as_ref().unwrap().count, 0);
}

#[test]
fn caller_merge_policy_controls_opaque_identity_and_keeps_carried_identity() {
    let mut incoming = stack(3, 2);
    incoming.data_mut().wire = Wire::new(ItemStack::new("example:different_baseline", 1), Arc::from([9u8]));
    let original = Some(incoming);
    let mut first = vec![original.clone()]; let mut cursor = Some(stack(1, 2));
    let before = state(&(&first, &cursor));
    assert_eq!(cells::collect_ordered(&mut first, &mut [], &mut cursor, &[cells::Cell::First(0)], 64,
        |left, right| same(left, right) && left.data().unwrap().wire.patch() == right.data().unwrap().wire.patch()), Ok(false));
    assert_eq!(state(&(&first, &cursor)), before);
    // Legacy native component equality admits this merge; the helper does not
    // misrepresent that policy choice as preserving the discarded wire identity.
    assert_eq!(cells::collect_ordered(&mut first, &mut [], &mut cursor, &[cells::Cell::First(0)], 64, same), Ok(true));
    assert!(first[0].is_none());
    let mut expected = stack(1, 2); expected.count = 4;
    assert_eq!(state(&cursor), state(&Some(expected)));
}

#[test]
fn consumer_wrappers_reject_malformed_inputs_without_changing_values() {
    let mut cells = vec![None, Some(stack(0, 2))]; let mut cursor = Some(stack(0, 2));
    let before = state(&(&cells, &cursor));
    assert!(!consumer::pickup_player(&mut cells, 0, &mut cursor, true, 64));
    assert!(!consumer::pickup_player(&mut cells, usize::MAX, &mut cursor, true, 64));
    assert!(consumer::take_for_drop(&mut cells, 1, false).is_none());
    assert!(consumer::take_for_drop(&mut cells, usize::MAX, true).is_none());
    assert!(!consumer::pickup_all_ordered(&mut cells, &mut [], &mut cursor, &[(true, 1)], 64));
    assert_eq!(state(&(&cells, &cursor)), before);
    cursor = Some(stack(1, 2));
    let before = state(&(&cells, &cursor));
    assert!(!consumer::pickup_player(&mut cells, 0, &mut cursor, true, 0));
    assert!(!consumer::pickup_all_ordered(&mut cells, &mut [], &mut cursor, &[(true, 0)], 0));
    assert_eq!(state(&(&cells, &cursor)), before);
}
