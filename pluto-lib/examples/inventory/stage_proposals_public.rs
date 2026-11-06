// Consumer-owned value example. These are not native ItemStack or Game types.
// Uses the actual public crate; never copy its source or registry modules.
// Source-reviewed only at introduction: a Cargo type check is still pending.
use pluto_inventory_api::inventory::{plan_between, stage_updates, MoveError, StackSnapshot};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Quartz { grade: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Bin {
    Input,
    Output,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Cell {
    item: Variant,
    count: u32,
    limit: u32,
}

fn main() -> Result<(), MoveError> {
    let quartz = Variant::Quartz { grade: 2 };
    let live = BTreeMap::from([
        (Bin::Input, Cell { item: quartz, count: 12, limit: 64 }),
        (Bin::Output, Cell { item: quartz, count: 60, limit: 64 }),
    ]);
    let before = live.clone();
    let source = &live[&Bin::Input];
    let destination = &live[&Bin::Output];

    // Limit, admission and merge identity are this example consumer's policy.
    // Logical indexes 0/1 are not native slot indexes or access capabilities.
    let planned = plan_between(
        Some(StackSnapshot { item: &source.item, count: source.count }),
        Some(StackSnapshot { item: &destination.item, count: destination.count }),
        12,
        |_, _| destination.limit,
        |_, item| *item == quartz,
        |left, right| left == right,
    )?;
    let movement = planned.expect("this example must plan the four available spaces");
    assert_eq!((movement.from, movement.to, movement.count), (0, 1, 4));
    let amount = i32::try_from(movement.count).expect("this example is bounded to 64");

    // Clone caller-owned rows once per partition. No callback touches `live`.
    // The library does not enforce conservation or supply these policy rules.
    let proposals = stage_updates(
        [(Bin::Input, -amount), (Bin::Output, amount)],
        |change| change.0,
        |key| live.get(key).cloned(),
        |cell, (_, delta)| {
            let Some(count) = cell.count.checked_add_signed(delta) else { return false; };
            if count > cell.limit { return false; }
            cell.count = count;
            true
        },
    ).expect("both independent proposals fit");
    assert_eq!(proposals[&Bin::Input].count, 8);
    assert_eq!(proposals[&Bin::Output].count, 64);
    assert_eq!(live, before);

    // A rejected later change drops earlier proposals, not live state.
    // This would NOT undo an external callback's side effects.
    let rejected = stage_updates(
        [(Bin::Input, -amount), (Bin::Output, 100)],
        |change| change.0,
        |key| live.get(key).cloned(),
        |cell, (_, delta)| {
            let Some(count) = cell.count.checked_add_signed(delta) else { return false; };
            if count > cell.limit { return false; }
            cell.count = count;
            true
        },
    );
    assert!(rejected.is_none());
    assert_eq!(live, before);

    // Stop at proposals. A real consumer must independently revalidate its
    // actor/world/slots, snapshots and policy, then commit at its native phase.
    // No generic save, network notification or rollback API is implied here.
    Ok(())
}
