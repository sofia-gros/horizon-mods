#[derive(Clone, Debug)]
pub struct Pages<T> {
    tabs: Vec<T>,
    selected: Vec<usize>,
    page: usize,
}
impl<T> Default for Pages<T> {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            selected: Vec::new(),
            page: 0,
        }
    }
}
impl<T: Clone> Pages<T> {
    pub fn initialize(&mut self, tabs: &[T]) -> bool {
        if !self.tabs.is_empty() || tabs.len() <= 14 {
            return false;
        }
        self.tabs = tabs.to_vec();
        self.selected = vec![0; tabs.len().div_ceil(14)];
        true
    }
    pub fn count(&self) -> usize {
        self.selected.len()
    }
    pub fn page(&self) -> usize {
        self.page
    }
    pub fn visible(&self) -> Vec<T> {
        self.tabs
            .iter()
            .skip(self.page * 14)
            .take(14)
            .cloned()
            .collect()
    }
    pub fn turn(&mut self, delta: isize, selected: usize) -> Option<(Vec<T>, usize)> {
        if self.selected.is_empty() {
            return None;
        }
        let next = self.page.checked_add_signed(delta)?;
        if next >= self.count() {
            return None;
        }
        self.selected[self.page] = selected.min(self.visible().len() - 1);
        self.page = next;
        Some((self.visible(), self.selected[next]))
    }
}
