// The settle of the menu opens a mod's block use queued during its right-click dispatch
// (pluto-menu-open), in the host's right-click bridge right after that dispatch: a click
// nothing else claimed opens the clicked block's menu here and is claimed by it, as a
// block use that acted claims its click. (The settle cannot ride the use itself: the
// weaver refuses every re-emitting reach into use_hand's try/closure body.)
fn pluto_menu_open_settle(value: (), game: &mut crate::game::Game<'_>, pos: [i32; 3], cancel: &mut bool) {
    game.settle_queued_menu_open(pos, cancel);
    value
}
