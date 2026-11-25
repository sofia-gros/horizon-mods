fn pluto_creative_rebuild(this: &mut crate::creative::Creative) {
    // only when the inventory is first built; later rebuilds (a permission change) keep what is there
    if !this.tabs.is_empty() || !pluto_creative::begin() { return; }
    let existing: Vec<String> = this.exported.iter().map(|e| e.tab.id.clone()).collect();
    let mod_tabs = this.exported.iter().filter(|e| e.tab.page >= 1).count();
    if let Some(document) = pluto_creative::document(&existing, mod_tabs) {
        if let Ok(extra) = crate::creative::Creative::from_json(&document) {
            println!("pluto_lib: creative tabs added, tabs={}", extra.exported.len());
            for e in extra.exported {
                if e.tab.search { continue; }
                match this.exported.iter_mut().find(|x| x.tab.id == e.tab.id) {
                    Some(x) => { x.tab.items.extend(e.tab.items); x.operator.extend(e.operator); }
                    None => this.exported.push(e),
                }
            }
            this.exported_search.extend(extra.exported_search);
        }
    }
    pluto_creative::end();
}
