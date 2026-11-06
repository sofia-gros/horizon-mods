// TConstruct transport policy: negative header resets, nonnegative header
// commits the byte count only after all word slots arrive. Actor/session state,
// slot IDs, schema and reassembly stay in the existing send/receive callers.
#[cfg(test)]
const MAX_BYTES: usize = horizon_client::pluto_inventory::menu_words::MAX_BYTES;

pub(super) fn encode(patterns: &[String]) -> Option<Vec<i16>> {
    horizon_client::pluto_inventory::menu_words::encode_utf8_list(patterns)
}

pub(super) fn decode(size: i16, words: &[i16]) -> Option<Vec<String>> {
    horizon_client::pluto_inventory::menu_words::decode_utf8_list(
        size, words, horizon_client::pluto_inventory::menu_words::EmptyStrings::Reject,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_updates_round_trip_and_partial_updates_are_rejected() {
        let patterns = ["tconstruct:pick_head", "tconstruct:tool_handle", "test:custom_pattern"].map(str::to_owned);
        let encoded = encode(&patterns).unwrap();
        assert_eq!(decode(encoded[0], &encoded[1..]), Some(patterns.to_vec()));
        assert_eq!(decode(encoded[0], &encoded[1..encoded.len() - 1]), None);
        assert_eq!(decode(-1, &[]), None);
        assert!(encode(&["x".repeat(MAX_BYTES)]).is_none());
    }

    #[test]
    fn material_information_round_trips_with_json_and_unicode() {
        let info = ["2", "tconstruct:flint", "1.5", r#"{"tconstruct:head":{"durability":100}}"#, r#"{"tconstruct:head":{"test:éclat":1}}"#, "tconstruct:example"].map(str::to_owned);
        let encoded = encode(&info).unwrap();
        assert_eq!(decode(encoded[0], &encoded[1..]), Some(info.to_vec()));
    }

    #[test]
    fn station_reason_frames_replace_failures_and_clear_on_success() {
        let failed = ["{}", " ", "Cannot modify: unsupported saved item components"].map(str::to_owned);
        let cleared = ["{}", " ", " "].map(str::to_owned);
        let frame = encode(&failed).unwrap();
        assert_eq!(decode(frame[0], &frame[1..]), Some(failed.to_vec()));
        let next = encode(&cleared).unwrap();
        assert_eq!(decode(next[0], &next[1..]), Some(cleared.to_vec()));
        assert_eq!(decode(next[0], &next[1..next.len() - 1]), None);
    }

}
