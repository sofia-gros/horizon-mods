use super::source_builder_wire as consumer;
use pluto_inventory_api::menu_words::{decode_utf8_list, encode_utf8_list, EmptyStrings, MAX_BYTES};
#[path = "legacy_codec.rs"]
mod legacy;

#[test]
fn encoding_and_consumer_results_match_frozen_source() {
    let seeds = ["x", "é", "😀", "", "\0", "{\"example\":42}"];
    for count in 0..=64 {
        for offset in 0..seeds.len() {
            let values: Vec<_> = (0..count).map(|n| seeds[(n + offset) % seeds.len()].to_owned()).collect();
            let before = values.clone();
            let old = legacy::encode(&values); let new = consumer::encode(&values);
            assert_eq!(new, old);
            let frame = new.unwrap();
            assert_eq!(consumer::decode(frame[0], &frame[1..]), legacy::decode(frame[0], &frame[1..]));
            assert_eq!(decode_utf8_list(frame[0], &frame[1..], EmptyStrings::Allow), Some(values.clone()));
            assert_eq!(values, before);
        }
    }
}

#[test]
fn malformed_frames_preserve_all_7600_legacy_results() {
    let alphabet = [i16::MIN, -1, 0, 1, 255, 256, i16::MAX];
    let mut compared = 0;
    for len in 0..=3 {
        for pattern in 0..7usize.pow(len as u32) {
            let mut code = pattern;
            let words: Vec<_> = (0..len).map(|_| { let value = alphabet[code % 7]; code /= 7; value }).collect();
            let before = words.clone();
            for size in -3..=15 {
                assert_eq!(consumer::decode(size, &words), legacy::decode(size, &words), "size={size} words={words:?}");
                compared += 1;
            }
            assert_eq!(words, before);
        }
    }
    assert_eq!(compared, 7600);
}

#[test]
fn signed_words_utf8_and_noncanonical_padding_stay_exact() {
    let emoji = vec!["😀".to_owned()];
    assert_eq!(encode_utf8_list(&emoji), Some(vec![8, 1, 4, 0xf09fu16 as i16, 0x9880u16 as i16]));
    let mut frame = consumer::encode(&["x".into()]).unwrap();
    assert_eq!(frame, vec![5, 1, 1, 0x7800]);
    *frame.last_mut().unwrap() |= 0x7f;
    // Unused padding is deliberately not a canonicality/authentication check.
    assert_eq!(consumer::decode(frame[0], &frame[1..]), Some(vec!["x".into()]));
    assert_eq!(consumer::decode(frame[0], &frame[1..]), legacy::decode(frame[0], &frame[1..]));
    assert_eq!(consumer::decode(6, &[1, 2, 0xc328u16 as i16]), None);
    assert_eq!(consumer::decode(3, &[0, 0]), None); // trailing declared byte
    assert_eq!(consumer::decode(2, &[1]), None); // missing length
    assert_eq!(consumer::decode(0, &[]), None); // missing count
    assert_eq!(consumer::decode(2, &[0]), Some(vec![]));
}

#[test]
fn empty_field_policy_remains_an_explicit_consumer_choice() {
    let fields = vec!["".into(), "example:value".into(), "".into()];
    let frame = encode_utf8_list(&fields).unwrap();
    assert_eq!(decode_utf8_list(frame[0], &frame[1..], EmptyStrings::Allow), Some(fields));
    assert_eq!(decode_utf8_list(frame[0], &frame[1..], EmptyStrings::Reject), None);
    assert_eq!(consumer::decode(frame[0], &frame[1..]), None);
    assert_eq!(legacy::decode(frame[0], &frame[1..]), None);
    assert_eq!(decode_utf8_list(2, &[0], EmptyStrings::Reject), Some(vec![]));
}

#[test]
fn declared_length_count_and_byte_limits_preserve_failures() {
    for length in [MAX_BYTES - 4, MAX_BYTES - 3, u16::MAX as usize, u16::MAX as usize + 1] {
        let values = vec!["x".repeat(length)]; let before = values.clone();
        assert_eq!(consumer::encode(&values), legacy::encode(&values));
        assert_eq!(consumer::encode(&values).is_some(), length == MAX_BYTES - 4);
        assert_eq!(values, before);
    }
    for count in [16_382, 16_383, u16::MAX as usize, u16::MAX as usize + 1] {
        let values = vec![String::new(); count];
        assert_eq!(consumer::encode(&values), legacy::encode(&values));
        assert_eq!(consumer::encode(&values).is_some(), count == 16_382);
    }
    let frame = encode_utf8_list(&["x".repeat(MAX_BYTES - 4)]).unwrap();
    assert_eq!(frame[0], i16::MAX);
    assert_eq!(decode_utf8_list(frame[0], &frame[1..], EmptyStrings::Reject), Some(vec!["x".repeat(MAX_BYTES - 4)]));
    assert_eq!(consumer::decode(-1, &frame[1..]), None);
    assert_eq!(consumer::decode(frame[0], &frame[1..frame.len() - 1]), None);
}
