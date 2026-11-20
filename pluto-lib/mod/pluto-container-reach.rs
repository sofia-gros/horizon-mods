
// A container's slots through a side as plain values for `Host` (`NativeContainerSlot` is crate-private, so the generated `Host` cannot name it).
impl Game<'_> {
    /// The slots of the container at `pos` that its side `face` reaches, as `(slot, stack, can_take)`: what `native_container_slots` reads.
    /// None where it has none. API 48.
    pub(in crate::game) fn native_container_reach(
        &self,
        dimension: &str,
        pos: [i32; 3],
        face: Direction,
    ) -> Option<Vec<(usize, Option<ItemStack>, bool)>> {
        self.native_container_slots(dimension, pos, face)
            .map(|slots| slots.into_iter().map(|slot| (slot.slot, slot.stack, slot.can_take)).collect())
    }
}
