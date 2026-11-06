//! Limited Java-compatible hashing and linked-bin map iteration mechanics.
//!
//! This is not a general map and does not implement equality, replacement,
//! removals, custom load factors or tree bins. Callers supply exactly one hash
//! per distinct logical key, in its insertion order. Equal hashes may represent
//! different keys. The result contains input positions, not values or IDs.

/// An unsupported Java tree-bin transition. No partial order is returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderError {
    TreeBin,
}

/// Java String.hashCode bits over UTF-16 code units, including lone surrogates.
/// There is no normalization, encoding validation or namespace interpretation.
pub fn utf16_hash(units: impl IntoIterator<Item = u16>) -> u32 {
    units.into_iter().fold(0u32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(u32::from(unit))
    })
}

fn resize(buckets: &mut Vec<Vec<(u32, usize)>>) {
    let old_capacity = buckets.len();
    let mut next = vec![Vec::new(); old_capacity * 2];
    for (index, bucket) in std::mem::take(buckets).into_iter().enumerate() {
        for entry in bucket {
            let target = if entry.0 & old_capacity as u32 == 0 {
                index
            } else {
                index + old_capacity
            };
            next[target].push(entry);
        }
    }
    *buckets = next;
}

/// Return default Java HashMap linked-bin traversal as input positions.
///
/// `hashes` are raw key hashCode bits, NOT pre-spread hashes. This applies Java's
/// xor spread, initial capacity 16 and load factor 0.75, preserving linked-bin
/// insertion order through low/high resize splitting. A ninth collision asks
/// for treeification: below capacity 64 it resizes; at 64 or above it returns
/// `TreeBin`. No approximate tree traversal or fallback order is supplied.
///
/// Inputs must already account for key equality and replacement. This function
/// cannot infer duplicate keys from equal hashes. It is an opt-in compatibility
/// algorithm, not the host's native recipe or resource ordering policy.
pub fn linked_iteration_order(hashes: &[u32]) -> Result<Vec<usize>, OrderError> {
    let mut buckets = vec![Vec::<(u32, usize)>::new(); 16];
    let mut size = 0usize;
    for (index, raw) in hashes.iter().copied().enumerate() {
        let hash = raw ^ (raw >> 16);
        let slot = hash as usize & (buckets.len() - 1);
        let treeify = buckets[slot].len() >= 8;
        buckets[slot].push((hash, index));
        size += 1;
        if treeify {
            if buckets.len() >= 64 {
                return Err(OrderError::TreeBin);
            }
            resize(&mut buckets);
        }
        if size > buckets.len() * 3 / 4 {
            resize(&mut buckets);
        }
    }
    Ok(buckets.into_iter().flatten().map(|(_, index)| index).collect())
}
