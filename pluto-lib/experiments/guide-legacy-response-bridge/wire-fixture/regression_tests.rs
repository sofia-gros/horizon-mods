// Actual Text and original legacy codec when compiled. No Game, actor, screen
// admission, native observer, socket or live overlay is constructed by this test.
use super::{old_apply, candidate_apply, tconstruct_materials_book_wire as wire};
use horizon_client_core::text::{ClickEvent, Style, Text};

const ROOT: &str = "tconstruct:materials-book-root";
const LIMIT: usize = 262_144;

fn pages(title: &str, count: usize) -> Vec<Text> {
    assert!(count > 0);
    let mut result = vec![Text::styled(format!("{title}\n\n"), Style { insertion: Some(ROOT.into()), ..Default::default() })];
    result.extend((1..count).map(|n| Text::literal(format!("page {n}"))));
    result
}
fn compare(current: &[Text], page: usize, sequence: u64, data: &[u8]) -> (Vec<Text>, usize) {
    let mut old = current.to_vec(); let mut new = current.to_vec();
    let mut old_page = page; let mut new_page = page;
    old_apply(sequence, &mut old, &mut old_page, data);
    candidate_apply(sequence, &mut new, &mut new_page, data);
    assert_eq!(old, new); assert_eq!(old_page, new_page);
    assert_eq!(old.iter().map(Text::component).collect::<Vec<_>>(), new.iter().map(Text::component).collect::<Vec<_>>());
    (new, new_page)
}
fn reply(body: &[u8], sequence: u64) -> Vec<u8> {
    let mut result = sequence.to_be_bytes().to_vec(); result.extend_from_slice(body); result
}

#[test]
fn all_six_legacy_titles_refresh_without_new_title_identity() {
    for title in ["Materials & You", "Puny Smelting", "Mighty Smelting", "Encyclopedia", "Tinkers' Gadgetry", "Fantastic Foundry"] {
        let original = pages(title, 4); let replacement = pages(title, 2);
        let data = wire::encode_reply(11, &replacement).unwrap();
        let result = compare(&original, 3, 11, &data);
        assert_eq!(result, (replacement, 1));
    }
}
#[test]
fn cover_sentinel_and_current_page_clamp_match_original_assignment() {
    let original = pages("Puny Smelting", 4); let replacement = pages("Puny Smelting", 2);
    let data = wire::encode_reply(11, &replacement).unwrap();
    for page in [usize::MAX, 0, 1, 2, 9] {
        let (_, got) = compare(&original, page, 11, &data);
        assert_eq!(got, if page == usize::MAX { usize::MAX } else { page.min(1) });
    }
}
#[test]
fn subsequent_same_sequence_reply_is_applied_again_not_consumed() {
    let original = pages("Puny Smelting", 4);
    let first = wire::encode_reply(5, &pages("Puny Smelting", 2)).unwrap();
    let (current, page) = compare(&original, 3, 5, &first);
    let second_pages = pages("Puny Smelting", 3);
    let second = wire::encode_reply(5, &second_pages).unwrap();
    assert_eq!(compare(&current, page, 5, &second), (second_pages, 1));
}
#[test]
fn wrong_sequence_or_other_book_keeps_all_current_values() {
    let original = pages("Puny Smelting", 4);
    for (sequence, title) in [(12, "Puny Smelting"), (11, "Mighty Smelting")] {
        let data = wire::encode_reply(sequence, &pages(title, 2)).unwrap();
        assert_eq!(compare(&original, usize::MAX, 11, &data), (original.clone(), usize::MAX));
    }
}
#[test]
fn truncated_header_malformed_empty_and_oversize_body_preserve_current() {
    let original = pages("Puny Smelting", 2);
    let mut cases: Vec<Vec<u8>> = (0..8).map(|n| vec![0; n]).collect();
    cases.extend([reply(b"not json", 11), reply(b"[]", 11), reply(&vec![b' '; LIMIT + 1], 11)]);
    for data in cases { assert_eq!(compare(&original, 1, 11, &data), (original.clone(), 1)); }
}
#[test]
fn invalid_current_root_or_unknown_title_does_not_borrow_another_book() {
    let data = wire::encode_reply(11, &pages("Puny Smelting", 2)).unwrap();
    let mut unmarked = pages("Puny Smelting", 2); unmarked[0].spans[0].style.insertion = None;
    for original in [unmarked, pages("Translated unknown title", 2), vec![Text::empty()]] {
        assert_eq!(compare(&original, usize::MAX, 11, &data), (original.clone(), usize::MAX));
    }
}
#[test]
fn maximum_body_plus_eight_byte_header_is_not_the_observer_ceiling() {
    let original = pages("Puny Smelting", 2);
    let mut value = pages("Puny Smelting", 1);
    loop { value.push(Text::literal("x".repeat(4000))); if wire::encode(&value).is_none() { value.pop(); break; } }
    value.push(Text::literal(""));
    let (mut low, mut high) = (0, 4000);
    while low < high { let middle = (low + high + 1) / 2; *value.last_mut().unwrap() = Text::literal("x".repeat(middle)); if wire::encode(&value).is_some() { low = middle; } else { high = middle - 1; } }
    *value.last_mut().unwrap() = Text::literal("x".repeat(low));
    let data = wire::encode_reply(11, &value).unwrap();
    assert_eq!(data.len(), LIMIT + 8);
    assert_eq!(compare(&original, 1, 11, &data).0, value);
}
#[test]
fn decoded_span_and_metadata_filtering_remains_the_legacy_policy() {
    let original = pages("Puny Smelting", 1);
    let body = serde_json::json!([[{"text":"Puny Smelting\n\n","insertion":ROOT},
        {"text":"link","page":1,"insertion":"tconstruct:book-title","bold":true},
        {"text":"bad metadata","page":100,"insertion":"https://example.invalid"}]]);
    let data = reply(&serde_json::to_vec(&body).unwrap(), 11);
    let (got, _) = compare(&original, 0, 11, &data);
    assert_eq!(got[0].spans[1].style.click, Some(ClickEvent::ChangePage(1)));
    assert_eq!(got[0].spans[1].style.underlined, Some(true));
    assert_eq!(got[0].spans[2].style.click, None);
    assert_eq!(got[0].spans[2].style.insertion, None);
}
#[test]
fn current_view_values_can_be_supplied_without_an_owned_opening_token() {
    // No Game/overlay is modeled. This checks only the retained-value helper:
    // legacy admission remains root/title+sequence, not a new owner or tracker.
    let externally_created = pages("Fantastic Foundry", 3);
    let data = wire::encode_reply(u64::MAX, &pages("Fantastic Foundry", 1)).unwrap();
    assert_eq!(compare(&externally_created, 2, u64::MAX, &data).1, 0);
}
#[test]
fn invalid_replies_do_not_prevent_later_valid_same_sequence_refresh() {
    let original = pages("Puny Smelting", 4);
    let (after_bad, page) = compare(&original, 2, 11, &reply(b"[]", 11));
    let wanted = pages("Puny Smelting", 3); let good = wire::encode_reply(11, &wanted).unwrap();
    assert_eq!(compare(&after_bad, page, 11, &good), (wanted, 2));
}
