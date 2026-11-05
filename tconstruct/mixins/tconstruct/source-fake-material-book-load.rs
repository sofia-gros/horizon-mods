//! One-shot preservation across early native saved-book validation. This never
//! grants newly eligible recipes or reloads knowledge from disk during play.
use super::{RecipeIndex, Settings};

pub type Pending = (Vec<String>, Vec<String>);
const PREFIX: &str = "tconstruct:fake_metal_native/";
const LIMIT: usize = 1024; // At most 512 materials in each of two directions.

fn candidate(id: &str) -> bool {
    let Some(rest) = id.strip_prefix(PREFIX) else { return false; };
    let Some(hex) = rest.strip_prefix("block/").or_else(|| rest.strip_prefix("ingots/")) else { return false; };
    !hex.is_empty() && hex.len() <= 512 && hex.len() % 2 == 0
        && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn capture(book: &Settings) -> Option<Pending> {
    let known: Vec<_> = book.recipes.iter().filter(|id| candidate(id))
        .take(LIMIT + 1).cloned().collect();
    if known.is_empty() || known.len() > LIMIT { return None; }
    let highlighted = known.iter().filter(|id| book.to_be_displayed.contains(*id))
        .cloned().collect();
    Some((known, highlighted))
}

/// The caller takes its per-world pending value exactly once, after current
/// recipe generation and index validation. Missing/disabled recipes stay gone.
pub fn restore(book: &mut Settings, index: &RecipeIndex, pending: Pending) -> usize {
    let (known, highlighted) = pending;
    if known.len() > LIMIT || highlighted.len() > LIMIT { return 0; }
    let mut restored = 0;
    for id in known.iter().filter(|id| candidate(id) && index.contains(id)) {
        restored += usize::from(book.recipes.insert(id.clone()));
    }
    for id in highlighted {
        if candidate(&id) && known.contains(&id) && index.contains(&id) && book.recipes.contains(&id) {
            book.to_be_displayed.insert(id);
        }
    }
    restored
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(material: &str) -> String {
        format!("{PREFIX}block/{}", material.bytes().map(|b| format!("{b:02x}")).collect::<String>())
    }
    fn index(ids: &[String]) -> RecipeIndex {
        let mut index = RecipeIndex::default();
        for (n, id) in ids.iter().enumerate() {
            index.recipes.insert(id.clone(), super::super::Known {
                special: false, displays: 1, first_display: n as i32,
                station: "minecraft:crafting_table", notify: true,
                result: Some("tconstruct:fake_storage_block".into()),
            });
        }
        index
    }

    #[test]
    fn captures_only_saved_candidates_and_their_highlights() {
        let bronze = id("tconstruct:bronze");
        let unsaved = id("tconstruct:invar");
        let mut saved = Settings::default();
        saved.recipes.extend([bronze.clone(), "minecraft:stick".into(), format!("{PREFIX}block/not_hex")]);
        saved.to_be_displayed.extend([bronze.clone(), unsaved]);
        assert_eq!(capture(&saved), Some((vec![bronze.clone()], vec![bronze])));
        assert_eq!(capture(&Settings::default()), None);
    }

    #[test]
    fn restores_only_current_world_saved_members_once_without_new_unlocks() {
        let bronze = id("tconstruct:bronze");
        let missing = id("tconstruct:invar");
        let new = id("tconstruct:electrum");
        let current = index(&[bronze.clone(), new.clone()]);
        let mut pending = Some((vec![bronze.clone(), missing.clone()], vec![bronze.clone(), missing.clone(), new.clone()]));
        let mut book = Settings::default();
        book.recipes.insert("minecraft:stick".into());
        book.open[0] = true;
        book.filtering[0] = true;
        assert_eq!(restore(&mut book, &current, pending.take().unwrap()), 1);
        assert!(pending.is_none());
        assert!(book.recipes.contains(&bronze));
        assert!(book.recipes.contains("minecraft:stick"));
        assert!(!book.recipes.contains(&new));
        assert!(!book.recipes.contains(&missing));
        assert_eq!(book.to_be_displayed, [bronze.clone()].into_iter().collect());
        assert!(book.open[0] && book.filtering[0]);
        // Once consumed, a later take/removal cannot be undone by the snapshot.
        book.recipes.remove(&bronze);
        assert!(pending.take().is_none());
        assert!(!book.recipes.contains(&bronze));
    }

    #[test]
    fn absent_world_index_does_not_restore_saved_recipes() {
        let bronze = id("tconstruct:bronze");
        let mut book = Settings::default();
        assert_eq!(restore(&mut book, &RecipeIndex::default(),
            (vec![bronze.clone()], vec![bronze])), 0);
        assert!(book.recipes.is_empty() && book.to_be_displayed.is_empty());
    }

    #[test]
    fn bounded_and_exact_identifier_filter() {
        assert!(candidate(&id("tconstruct:bronze")));
        for bad in [format!("{PREFIX}block/"), format!("{PREFIX}block/abc"),
            format!("{PREFIX}ingots/FF"), format!("{PREFIX}other/00"),
            format!("{PREFIX}block/{}", "a".repeat(514)), "minecraft:stick".into()] {
            assert!(!candidate(&bad), "{bad}");
        }
        let mut book = Settings::default();
        for n in 0..=LIMIT { book.recipes.insert(format!("{PREFIX}block/{n:04x}")); }
        assert!(capture(&book).is_none());
    }
}
