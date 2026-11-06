//! Pure bounded UTF-8 list framing in signed 16-bit words.
//!
//! This codec has no receiver, callbacks, transport, slot IDs or mutable state.
//! A successful frame is only serialized data, never a delivery/commit token.

/// The byte-count header must fit a nonnegative signed 16-bit value.
pub const MAX_BYTES: usize = i16::MAX as usize;

/// Caller-selected structural validation, not content/schema interpretation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyStrings { Allow, Reject }

/// Encode a big-endian u16 count and u16-byte-length-prefixed UTF-8 strings.
/// The first returned word is the byte count. Subsequent signed words preserve
/// all 16 bits; an odd final byte is padded with zero in the low byte.
/// Empty fields are encodable, independent of a receiver's validation choice.
/// Failure returns no partial frame and does not change the input.
pub fn encode_utf8_list(values: &[String]) -> Option<Vec<i16>> {
    let count = u16::try_from(values.len()).ok()?;
    let mut bytes = count.to_be_bytes().to_vec();
    for value in values {
        let value = value.as_bytes();
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

/// Decode a complete declared frame; `words` excludes the byte-count header.
/// Reject negative sizes, wrong word counts, truncated lengths/data, invalid
/// UTF-8 and trailing declared bytes. The unused low padding byte of an odd
/// frame is deliberately ignored (not required to be zero).
///
/// `empty` controls empty-field validation at the same point as UTF-8 decoding.
/// There are no user callbacks or receiver updates on success or failure. A
/// malformed count can reserve up to u16::MAX String entries, but traversal and
/// byte reads remain bounded by the declared frame; no streaming allocation
/// or authenticated/canonical framing guarantee is provided.
pub fn decode_utf8_list(size: i16, words: &[i16], empty: EmptyStrings) -> Option<Vec<String>> {
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
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let len = length(&mut remaining)?;
        let (value, rest) = remaining.split_at_checked(len)?;
        let value = std::str::from_utf8(value).ok()?;
        if empty == EmptyStrings::Reject && value.is_empty() { return None; }
        values.push(value.to_owned());
        remaining = rest;
    }
    remaining.is_empty().then_some(values)
}
