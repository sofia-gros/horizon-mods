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

// The Sunrise host carries no woven field for the screen's pages, so
// the module owns the store: one Pages per live Creative screen,
// keyed by the screen's address. Creative is the creative_screen
// field of the long-lived Game value and is never value-swapped
// between worlds, so the address is stable for the screen's whole
// life. take/put preserve the take-out / mutate / write-back flow
// the consumers were written against; with reads the live value in
// place (its callback must not call back into this store).
static SCREENS: std::sync::Mutex<std::collections::BTreeMap<usize, Pages<super::Tab>>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

fn key(screen: &super::Creative) -> usize {
    std::ptr::from_ref(screen) as usize
}

/// Take the screen's pages out for mutation, leaving the default
/// empty value behind, exactly like mem::take on the old field.
pub fn take(screen: &mut super::Creative) -> Pages<super::Tab> {
    let mut store = SCREENS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    store.remove(&key(screen)).unwrap_or_default()
}

/// Write pages back to the screen's store slot.
pub fn put(screen: &mut super::Creative, pages: Pages<super::Tab>) {
    let mut store = SCREENS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    store.insert(key(screen), pages);
}

/// Read the screen's pages in place; an untouched screen reads as
/// the default empty value.
pub fn with<R>(screen: &super::Creative, run: impl FnOnce(&Pages<super::Tab>) -> R) -> R {
    let store = SCREENS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match store.get(&key(screen)) {
        Some(pages) => run(pages),
        None => run(&Pages::default()),
    }
}
