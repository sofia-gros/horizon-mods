// Lossless framing for server-derived Pattern IDs carried by standard menu
// data slots. A negative header starts a replacement; a nonnegative header
// commits exactly this many bytes after all data slots have arrived.
use std::convert::TryFrom;
const MAX_BYTES: usize = i16::MAX as usize;

pub(super) fn encode(patterns: &[String]) -> Option<Vec<i16>> {
    let count = u16::try_from(patterns.len()).ok()?;
    let mut bytes = count.to_be_bytes().to_vec();
    for pattern in patterns {
        let value = pattern.as_bytes();
        let len = u16::try_from(value.len()).ok()?;
        bytes.extend(len.to_be_bytes());
        bytes.extend(value);
        if bytes.len() > MAX_BYTES { return None; }
    }
    let size = i16::try_from(bytes.len()).ok()?;
    let mut words = Vec::with_capacity(1 + bytes.len().div_ceil(2));
    words.push(size);
    for pair in bytes.chunks(2) {
        words.push(i16::from_be_bytes([pair[0], pair.get(1).copied().unwrap_or(0)]));
    }
    Some(words)
}

pub(super) fn decode(size: i16, words: &[i16]) -> Option<Vec<String>> {
    let size = usize::try_from(size).ok()?;
    if size > MAX_BYTES || words.len() != size.div_ceil(2) { return None; }
    let mut bytes = Vec::with_capacity(words.len() * 2);
    for word in words { bytes.extend(word.to_be_bytes()); }
    bytes.truncate(size);
    let mut remaining = bytes.as_slice();
    fn length(input: &mut &[u8]) -> Option<usize> {
        let (&a, rest) = input.split_first()?;
        let (&b, rest) = rest.split_first()?;
        *input = rest;
        Some(u16::from_be_bytes([a, b]) as usize)
    }
    let count = length(&mut remaining)?;
    let mut patterns = Vec::with_capacity(count);
    for _ in 0..count {
        let len = length(&mut remaining)?;
        let (value, rest) = remaining.split_at_checked(len)?;
        let value = std::str::from_utf8(value).ok()?;
        if value.is_empty() { return None; }
        patterns.push(value.to_owned());
        remaining = rest;
    }
    remaining.is_empty().then_some(patterns)
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
}
