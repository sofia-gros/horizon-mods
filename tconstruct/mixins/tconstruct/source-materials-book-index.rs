// One-based book page numbers for chapter and page indexes.
pub(super) fn layout(sizes: &[usize]) -> Option<(usize, Vec<(usize, usize)>)> {
    let indexes = sizes.len().checked_add(4)? / 5;
    let mut next = indexes.checked_add(1)?;
    let mut chapters = Vec::with_capacity(sizes.len());
    for &pages in sizes {
        let chapter_indexes = pages.checked_add(4)? / 5;
        let first_page = next.checked_add(chapter_indexes)?;
        chapters.push((next, first_page));
        next = first_page.checked_add(pages)?;
    }
    Some((indexes, chapters))
}

#[cfg(test)]
mod tests {
    use super::layout;
    #[test]
    fn chapter_and_page_links_account_for_multiple_index_pages() {
        assert_eq!(layout(&[6, 1, 0, 5, 2, 8]), Some((2, vec![(3, 5), (11, 12), (13, 13), (13, 14), (19, 20), (22, 24)])));
        assert_eq!(layout(&[usize::MAX]), None);
    }
}
