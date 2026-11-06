// EXCLUDED comparison only. Not installed or called by a production provider.
// Precondition: Batch is produced by the exact pinned safe geometry module.
// Its builders guarantee finite, nonnegative positions and finite unit UVs;
// its private backing has no public mutation or deserialization entry point.
// Do not generalize this predicate to arbitrary vertices or future builders.
pub(super) fn cell_local(batch: &super::geometry::Batch) -> bool {
    batch.as_slice().iter().all(|quad| {
        quad.vertices.iter().all(|vertex| {
            vertex.position.iter().all(|&value| value <= 1.0)
        })
    })
}
