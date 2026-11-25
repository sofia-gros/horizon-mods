// Static overrides on each declared block state, before the host applies its
// other content properties. Native block settings do not currently write push.
fn pluto_block_settings_gameplay(b:&NewBlock,row:&mut J) {
    crate::mod_content::pluto_item_placement_export::block_settings(&b.decl.id,|push|row.set("push",J::Str(push.into())));
}
