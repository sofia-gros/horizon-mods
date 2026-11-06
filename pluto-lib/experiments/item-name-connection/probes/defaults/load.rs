fn unchanged_load(
    root: &Path,
    block_for_item: &HashMap<&str, BlockStateId>,
    overlay: &PackOverlay<'_>,
    original: impl FnOnce(&Path, &HashMap<&str, BlockStateId>, &PackOverlay<'_>) -> Result<Items>,
) -> Result<Items> {
    original(root, block_for_item, overlay)
}
