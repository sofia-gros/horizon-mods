// A complete, bounded host snapshot. None means unknown, including malformed
// frames; an empty known set means the tag exists but has no members.
use std::collections::HashSet;
const MAX: usize = 16_384;
const REFRESH_TICKS: u8 = 20;

// Refresh even without a change: a client can clear its world while a sleeping
// level retains the old per-guest cache. A short bounded delay restores it.
pub(super) fn needs_send(entry: &mut (Vec<u8>, u8), data: &[u8]) -> bool {
    if entry.0 != data || entry.1 == 0 {
        entry.0 = data.to_vec(); entry.1 = REFRESH_TICKS - 1; true
    } else { entry.1 -= 1; false }
}
fn valid(id: &str) -> bool {
    let Some((namespace, path)) = id.split_once(':') else { return false; };
    !namespace.is_empty() && !path.is_empty() && namespace.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b))
        && path.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_./-".contains(&b))
}
pub(super) fn encode(tags: Option<&HashSet<String>>) -> Vec<u8> {
    let Some(tags) = tags else { return vec![0]; };
    let mut ids: Vec<_> = tags.iter().collect(); ids.sort();
    if ids.iter().any(|id| !valid(id)) { return vec![0]; }
    let mut out = vec![1];
    for id in ids { out.extend_from_slice(id.as_bytes()); out.push(b'\n'); if out.len() > MAX { return vec![0]; } }
    out
}
pub(super) fn decode(bytes: &[u8]) -> Option<HashSet<String>> {
    if bytes.len() > MAX || bytes.first() != Some(&1) { return None; }
    let rest = &bytes[1..];
    if rest.is_empty() { return Some(HashSet::new()); }
    if rest.last() != Some(&b'\n') { return None; }
    let text = std::str::from_utf8(&rest[..rest.len()-1]).ok()?;
    let mut values = HashSet::new();
    for id in text.split('\n') { if !valid(id) || !values.insert(id.to_owned()) { return None; } }
    Some(values)
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn known_unknown_and_empty_are_distinct() {
        let set = HashSet::from(["tconstruct:hammer".to_owned(), "test:custom".to_owned()]);
        assert_eq!(decode(&encode(Some(&set))), Some(set));
        assert_eq!(decode(&encode(Some(&HashSet::new()))), Some(HashSet::new()));
        assert_eq!(decode(&encode(None)), None);
    }
    #[test] fn malformed_or_oversized_never_becomes_narrow() {
        for bad in [b"".as_slice(), b"\x01bad\n", b"\x01test:item", b"\x01test:item\ntest:item\n", b"\x00extra"] { assert!(decode(bad).is_none()); }
        assert!(decode(&vec![1; MAX+1]).is_none());
        assert_eq!(encode(Some(&HashSet::from(["bad id".to_owned()]))), vec![0]);
    }
    #[test] fn retained_cache_refreshes_after_dimension_return() {
        let bytes = encode(Some(&HashSet::from(["test:hammer".to_owned()])));
        let mut cache = (Vec::new(), 0);
        assert!(needs_send(&mut cache, &bytes));
        // A cleared guest may return while its sleeping level kept this cache.
        for _ in 0..REFRESH_TICKS - 1 { assert!(!needs_send(&mut cache, &bytes)); }
        assert!(needs_send(&mut cache, &bytes));
        // Changed/unknown snapshots are sent immediately, not after the timer.
        assert!(needs_send(&mut cache, &encode(None)));
        assert!(needs_send(&mut cache, &bytes));
    }
    #[test] fn replacement_reload_and_reconnect() {
        let broad = HashSet::from(["test:hammer".to_owned()]);
        let narrow = HashSet::from(["test:pick".to_owned()]);
        let mut guest = decode(&encode(Some(&broad))); assert_eq!(guest.as_ref().map(|s| s.contains("test:hammer")), Some(true));
        guest = decode(&encode(Some(&narrow))); assert_eq!(guest.as_ref().map(|s| s.contains("test:hammer")), Some(false));
        guest = decode(&encode(None)); assert_eq!(guest.as_ref().map(|s| s.contains("test:hammer")), None);
        // A new connection begins with no snapshot until a fresh frame arrives.
        let fresh: Option<HashSet<String>> = None; assert!(fresh.is_none());
    }
}
