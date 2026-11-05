//! Index the bounded Travelers projections after authoritative generation.
//! Inputs remain ordinary native ingredients; the normal placement path stays.
use super::{Known, RecipeIndex};
const PREFIX: &str = "tconstruct:travelers_native/";
const OUTPUTS: [&str; 5] = ["tconstruct:travelers_helmet", "tconstruct:travelers_chestplate",
    "tconstruct:travelers_leggings", "tconstruct:travelers_boots", "tconstruct:travelers_shield"];

/// Descriptors are produced by the validated world projection, not raw files.
/// Reuse unoccupied display numbers so repeated refreshes alongside other
/// synthetic families cannot continually push the display numbering upward.
pub fn replace(index: &mut RecipeIndex, displays: &[(String, String, bool)]) {
    index.recipes.retain(|id, _| !id.starts_with(PREFIX));
    if displays.len() > 8192 { return; }
    let occupied: Option<Vec<(i32, i32)>> = index.recipes.values().map(|known| {
        let count = i32::try_from(known.displays).ok()?;
        Some((known.first_display.max(0), known.first_display.checked_add(count)?))
    }).collect();
    let Some(mut occupied) = occupied else { return; };
    occupied.retain(|(start, end)| start < end);
    occupied.sort_unstable();
    let records: std::collections::BTreeMap<_, _> = displays.iter()
        .filter(|(id, output, _)| id.starts_with(PREFIX) && OUTPUTS.contains(&output.as_str()))
        .map(|(id, output, notify)| (id, (output, *notify))).collect();
    let mut next = 0i32;
    let mut range = 0;
    for (id, (output, notify)) in records {
        while range < occupied.len() {
            let (start, end) = occupied[range];
            if end <= next { range += 1; continue; }
            if start > next { break; }
            next = end;
            range += 1;
        }
        let Some(after) = next.checked_add(1) else { break; };
        index.recipes.insert(id.clone(), Known {
            special: false, displays: 1, first_display: next,
            station: "minecraft:crafting_table", notify, result: Some(output.clone()),
        });
        next = after;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(name: &str, notify: bool) -> (String, String, bool) {
        (format!("{PREFIX}{name}"), OUTPUTS[0].into(), notify)
    }
    fn native(index: &mut RecipeIndex, id: &str, start: i32, count: usize) {
        index.recipes.insert(id.into(), Known { special: false, displays: count,
            first_display: start, station: "minecraft:furnace", notify: false, result: None });
    }

    #[test]
    fn source_notification_and_result_use_one_native_display() {
        let mut index = RecipeIndex::default();
        let yes = record("a", true);
        let no = record("b", false);
        replace(&mut index, &[yes.clone(), no.clone()]);
        assert_eq!(index.displays(&yes.0), 1);
        assert_eq!(index.toast_station(&yes.0), Some("minecraft:crafting_table"));
        assert_eq!(index.toast_station(&no.0), None);
        assert_eq!(index.result(&yes.0), Some(OUTPUTS[0]));
        assert_eq!(index.recipe_of_display(0), Some(yes.0.as_str()));
        assert!(!index.special(&yes.0));
    }

    #[test]
    fn refresh_reuses_gaps_without_moving_other_families_or_leaking_worlds() {
        let mut index = RecipeIndex::default();
        native(&mut index, "minecraft:other", 1, 2);
        native(&mut index, "tconstruct:fake_metal_native/block/example", 5, 1);
        let a = record("a", true);
        let b = record("b", true);
        replace(&mut index, &[b.clone(), a.clone()]);
        assert_eq!(index.display_ids(&a.0), 0..1);
        assert_eq!(index.display_ids(&b.0), 3..4);
        assert_eq!(index.display_ids("minecraft:other"), 1..3);
        assert_eq!(index.display_ids("tconstruct:fake_metal_native/block/example"), 5..6);
        replace(&mut index, &[a.clone(), b.clone()]);
        assert_eq!(index.display_ids(&a.0), 0..1);
        assert_eq!(index.display_ids(&b.0), 3..4);
        assert!(!RecipeIndex::default().contains(&a.0));
        replace(&mut index, &[]);
        assert!(!index.contains(&a.0) && !index.contains(&b.0));
        assert_eq!(index.display_ids("minecraft:other"), 1..3);
    }

    #[test]
    fn unsupported_descriptors_and_over_budget_projection_stay_absent() {
        let mut index = RecipeIndex::default();
        replace(&mut index, &[("other:recipe".into(), OUTPUTS[0].into(), true),
            (format!("{PREFIX}wrong"), "minecraft:diamond".into(), true)]);
        assert_eq!(index.ids().count(), 0);
        replace(&mut index, &vec![record("a", true); 8193]);
        assert_eq!(index.ids().count(), 0);
    }
}
